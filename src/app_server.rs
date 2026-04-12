use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStderr, ChildStdout, Command};
use tokio::sync::{Mutex, mpsc, oneshot};

use crate::error::{NscError, Result};

#[derive(Debug, Clone)]
pub struct RunSliceRequest {
    pub worktree_path: PathBuf,
    pub slice_repo_relative_path: PathBuf,
    pub prompt_template: String,
}

#[derive(Debug)]
pub struct StartedRun {
    pub thread_id: String,
    pub completion: oneshot::Receiver<Result<TerminalRunOutcome>>,
}

#[derive(Debug, Clone)]
pub enum TerminalRunOutcome {
    FinishedPromptOwned { terminal_message: String },
    AttentionNeeded { reason: String },
}

#[async_trait]
pub trait AppServerClient {
    async fn start_run(&self, request: RunSliceRequest) -> Result<StartedRun>;
}

#[derive(Debug, Clone)]
pub struct CodexAppServerClient {
    codex_bin: String,
}

impl Default for CodexAppServerClient {
    fn default() -> Self {
        Self {
            codex_bin: "codex".to_string(),
        }
    }
}

#[async_trait]
impl AppServerClient for CodexAppServerClient {
    async fn start_run(&self, request: RunSliceRequest) -> Result<StartedRun> {
        let mut session =
            Session::start(self.codex_bin.clone(), request.worktree_path.clone()).await?;
        session.initialize().await?;
        let thread_id = session.start_thread().await?;
        let turn_id = session.start_turn(build_prompt_text(&request)).await?;

        let (tx, rx) = oneshot::channel();
        tokio::spawn(async move {
            let result = session.wait_for_terminal_outcome(turn_id).await;
            let _ = tx.send(result);
        });

        Ok(StartedRun {
            thread_id,
            completion: rx,
        })
    }
}

fn build_prompt_text(request: &RunSliceRequest) -> String {
    format!(
        "{template}\n{slice_path}\n\nRead that exact slice file before acting.\nTreat it as immutable for this run.\n",
        template = request.prompt_template.trim_end(),
        slice_path = request.slice_repo_relative_path.display(),
    )
}

const MAX_RUN_DURATION: Duration = Duration::from_secs(8 * 60 * 60);

struct Session {
    child: Child,
    stdin: tokio::process::ChildStdin,
    stdout_events: mpsc::UnboundedReceiver<Value>,
    stderr_accumulator: Arc<Mutex<String>>,
    next_request_id: i64,
    responses: HashMap<i64, Value>,
    notifications: VecDeque<Value>,
    cwd: String,
    thread_id: Option<String>,
}

impl Session {
    async fn start(codex_bin: String, worktree_path: PathBuf) -> Result<Self> {
        let mut child = Command::new(codex_bin)
            .args([
                "-c",
                "shell_environment_policy.inherit=all",
                "app-server",
                "--listen",
                "stdio://",
            ])
            .current_dir(&worktree_path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|err| {
                NscError::AppServer(format!("failed to start codex app-server: {err}"))
            })?;

        let stdin = child.stdin.take().ok_or_else(|| {
            NscError::AppServer("codex app-server missing stdin pipe".to_string())
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            NscError::AppServer("codex app-server missing stdout pipe".to_string())
        })?;
        let stderr = child.stderr.take().ok_or_else(|| {
            NscError::AppServer("codex app-server missing stderr pipe".to_string())
        })?;

        let (tx, rx) = mpsc::unbounded_channel();
        let stderr_accumulator = Arc::new(Mutex::new(String::new()));
        spawn_stdout_reader(stdout, tx);
        spawn_stderr_reader(stderr, Arc::clone(&stderr_accumulator));

        Ok(Self {
            child,
            stdin,
            stdout_events: rx,
            stderr_accumulator,
            next_request_id: 1,
            responses: HashMap::new(),
            notifications: VecDeque::new(),
            cwd: worktree_path.display().to_string(),
            thread_id: None,
        })
    }

    async fn initialize(&mut self) -> Result<()> {
        let response = self
            .send_request(
                "initialize",
                json!({
                    "clientInfo": {
                        "name": "Night Shift Coordinator",
                        "version": "1",
                    },
                    "capabilities": {
                        "experimentalApi": false,
                    },
                }),
            )
            .await?;
        if response
            .get("userAgent")
            .and_then(Value::as_str)
            .unwrap_or("")
            .is_empty()
        {
            return Err(self.protocol_error("initialize response missing userAgent"));
        }
        self.send_notification("initialized", None).await
    }

    async fn start_thread(&mut self) -> Result<String> {
        let cwd = self.cwd_string()?;
        let response = self
            .send_request(
                "thread/start",
                json!({
                    "approvalPolicy": "never",
                    "cwd": cwd,
                    "ephemeral": true,
                    "serviceName": "Night Shift Coordinator",
                }),
            )
            .await?;
        let thread_id = response
            .get("thread")
            .and_then(Value::as_object)
            .and_then(|thread| thread.get("id"))
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| self.protocol_error("thread/start response missing thread.id"))?;
        let thread_id = thread_id.to_string();
        self.thread_id = Some(thread_id.clone());
        Ok(thread_id)
    }

    async fn start_turn(&mut self, prompt_text: String) -> Result<String> {
        let cwd = self.cwd_string()?;
        let response = self
            .send_request(
                "turn/start",
                json!({
                    "approvalPolicy": "never",
                    "cwd": cwd,
                    "input": [
                        {
                            "type": "text",
                            "text": prompt_text,
                        }
                    ],
                    "threadId": self
                        .thread_id
                        .clone()
                        .ok_or_else(|| self.protocol_error("thread id unavailable before turn/start"))?,
                }),
            )
            .await?;
        let turn_id = response
            .get("turn")
            .and_then(Value::as_object)
            .and_then(|turn| turn.get("id"))
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| self.protocol_error("turn/start response missing turn.id"))?;
        Ok(turn_id.to_string())
    }

    async fn wait_for_terminal_outcome(mut self, turn_id: String) -> Result<TerminalRunOutcome> {
        let outcome =
            apply_run_timeout(MAX_RUN_DURATION, self.collect_terminal_outcome(&turn_id)).await;
        let _ = self.child.kill().await;
        let _ = self.child.wait().await;
        outcome
    }

    async fn collect_terminal_outcome(&mut self, turn_id: &str) -> Result<TerminalRunOutcome> {
        let mut notifications = Vec::new();
        loop {
            let notification = self.wait_for_notification().await?;
            let method = notification
                .get("method")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            notifications.push(notification.clone());
            if method == "turn/completed" {
                let completed_turn = notification
                    .get("params")
                    .and_then(Value::as_object)
                    .and_then(|params| params.get("turn"))
                    .and_then(Value::as_object)
                    .ok_or_else(|| self.protocol_error("turn/completed missing params.turn"))?;
                let completed_turn_id = completed_turn
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if completed_turn_id != turn_id {
                    continue;
                }
                let status = completed_turn
                    .get("status")
                    .and_then(Value::as_str)
                    .unwrap_or("failed");
                if status != "completed" {
                    break Ok(TerminalRunOutcome::AttentionNeeded {
                        reason: format!("codex app-server turn finished with status {status}"),
                    });
                }
                let terminal_message = canonical_terminal_message(&notifications, turn_id)
                    .ok_or_else(|| {
                        self.protocol_error(
                            "codex app-server completed turn without terminal message content",
                        )
                    })?;
                return Ok(TerminalRunOutcome::FinishedPromptOwned { terminal_message });
            }
        }
    }

    async fn send_request(&mut self, method: &str, params: Value) -> Result<Value> {
        let request_id = self.next_request_id;
        self.next_request_id += 1;
        self.send_message(json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "method": method,
            "params": params,
        }))
        .await?;

        loop {
            if let Some(response) = self.responses.remove(&request_id) {
                if let Some(error) = response.get("error") {
                    return Err(NscError::AppServer(format!(
                        "{method} returned error: {}",
                        error
                    )));
                }
                return response.get("result").cloned().ok_or_else(|| {
                    self.protocol_error(&format!("{method} response missing result"))
                });
            }

            let payload = self.recv_payload().await?;
            if is_server_request(&payload) {
                return Err(self.protocol_error("server-initiated requests are not supported"));
            }
            if let Some(id) = payload.get("id").and_then(Value::as_i64) {
                self.responses.insert(id, payload);
            } else if payload.get("method").is_some() {
                self.notifications.push_back(payload);
            } else {
                return Err(self.protocol_error("malformed JSON-RPC payload"));
            }
        }
    }

    async fn send_notification(&mut self, method: &str, params: Option<Value>) -> Result<()> {
        let mut payload = json!({
            "jsonrpc": "2.0",
            "method": method,
        });
        if let Some(params) = params {
            payload["params"] = params;
        }
        self.send_message(payload).await
    }

    async fn send_message(&mut self, payload: Value) -> Result<()> {
        let encoded = serde_json::to_string(&payload)? + "\n";
        self.stdin.write_all(encoded.as_bytes()).await?;
        self.stdin.flush().await?;
        Ok(())
    }

    async fn wait_for_notification(&mut self) -> Result<Value> {
        loop {
            if let Some(notification) = self.notifications.pop_front() {
                return Ok(notification);
            }
            let payload = self.recv_payload().await?;
            if is_server_request(&payload) {
                return Err(self.protocol_error("server-initiated requests are not supported"));
            }
            if let Some(id) = payload.get("id").and_then(Value::as_i64) {
                self.responses.insert(id, payload);
                continue;
            }
            if payload.get("method").is_some() {
                return Ok(payload);
            }
            return Err(self.protocol_error("malformed JSON-RPC payload"));
        }
    }

    async fn recv_payload(&mut self) -> Result<Value> {
        match self.stdout_events.recv().await {
            Some(payload) => Ok(payload),
            None => {
                let stderr = self.stderr_accumulator.lock().await.clone();
                if let Some(status) = self.child.try_wait()? {
                    return Err(NscError::AppServer(format!(
                        "codex app-server exited with code {:?}: {}",
                        status.code(),
                        stderr.lines().next().unwrap_or_default()
                    )));
                }
                Err(NscError::ChannelClosed)
            }
        }
    }

    fn protocol_error(&self, message: &str) -> NscError {
        NscError::Protocol(message.to_string())
    }

    fn cwd_string(&self) -> Result<String> {
        Ok(self.cwd.clone())
    }
}

async fn apply_run_timeout<F>(duration: Duration, future: F) -> Result<TerminalRunOutcome>
where
    F: Future<Output = Result<TerminalRunOutcome>>,
{
    match tokio::time::timeout(duration, future).await {
        Ok(result) => result,
        Err(_) => Ok(TerminalRunOutcome::AttentionNeeded {
            reason: format!(
                "codex app-server run timed out after {} seconds",
                duration.as_secs()
            ),
        }),
    }
}

fn spawn_stdout_reader(stdout: ChildStdout, sender: mpsc::UnboundedSender<Value>) {
    tokio::spawn(async move {
        let mut reader = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Ok(payload) = serde_json::from_str::<Value>(line) {
                let _ = sender.send(payload);
            }
        }
    });
}

fn spawn_stderr_reader(stderr: ChildStderr, accumulator: Arc<Mutex<String>>) {
    tokio::spawn(async move {
        let mut reader = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            let mut guard = accumulator.lock().await;
            if !guard.is_empty() {
                guard.push('\n');
            }
            guard.push_str(&line);
        }
    });
}

fn is_server_request(payload: &Value) -> bool {
    payload.get("method").is_some()
        && payload.get("id").is_some()
        && payload.get("result").is_none()
        && payload.get("error").is_none()
}

fn canonical_terminal_message(notifications: &[Value], turn_id: &str) -> Option<String> {
    let mut item_completed = Vec::new();
    let mut wrapped_item_completed = Vec::new();
    let mut task_complete_messages = Vec::new();

    for notification in notifications {
        if notification_turn_id(notification).as_deref() != Some(turn_id) {
            continue;
        }
        let method = notification
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if method == "item/completed" {
            if let Some(item) = extract_item(notification) {
                let phase = normalize_label(item.get("phase"));
                let item_type = normalize_label(item.get("type"));
                if phase == "finalanswer" && item_type == "agentmessage" {
                    let text = extract_item_text(item);
                    if !text.is_empty() {
                        item_completed.push(text);
                    }
                }
            }
        } else if method == "codex/event/item_completed" {
            if let Some(item) = extract_item(notification) {
                let phase = normalize_label(item.get("phase"));
                let item_type = normalize_label(item.get("type"));
                if phase == "finalanswer" && item_type == "agentmessage" {
                    let text = extract_item_text(item);
                    if !text.is_empty() {
                        wrapped_item_completed.push(text);
                    }
                }
            }
        } else if (method == "task_complete" || method == "codex/event/task_complete")
            && let Some(message) = extract_task_complete_message(notification)
        {
            task_complete_messages.push(message);
        }
    }

    if !item_completed.is_empty() {
        Some(item_completed.join(""))
    } else if !wrapped_item_completed.is_empty() {
        Some(wrapped_item_completed.join(""))
    } else {
        task_complete_messages.pop()
    }
}

fn normalize_label(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_lowercase()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect()
}

fn extract_item(notification: &Value) -> Option<&Value> {
    let params = notification.get("params")?;
    if let Some(item) = params.get("item") {
        return Some(item);
    }
    params.get("msg")?.get("item")
}

fn extract_item_text(item: &Value) -> String {
    if let Some(text) = item.get("text").and_then(Value::as_str) {
        return text.to_string();
    }
    item.get("content")
        .and_then(Value::as_array)
        .map(|content| {
            content
                .iter()
                .filter_map(|entry| entry.get("text").and_then(Value::as_str))
                .collect::<String>()
        })
        .unwrap_or_default()
}

fn extract_task_complete_message(notification: &Value) -> Option<String> {
    let params = notification.get("params")?;
    if let Some(value) = params.get("last_agent_message").and_then(Value::as_str) {
        return Some(value.to_string());
    }
    if let Some(value) = params.get("lastAgentMessage").and_then(Value::as_str) {
        return Some(value.to_string());
    }
    let msg = params.get("msg")?;
    if let Some(value) = msg.get("last_agent_message").and_then(Value::as_str) {
        return Some(value.to_string());
    }
    msg.get("lastAgentMessage")
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn notification_turn_id(notification: &Value) -> Option<String> {
    let params = notification.get("params")?;
    if let Some(turn_id) = params.get("turnId").and_then(Value::as_str) {
        return Some(turn_id.to_string());
    }
    if let Some(turn) = params.get("turn")
        && let Some(turn_id) = turn.get("id").and_then(Value::as_str)
    {
        return Some(turn_id.to_string());
    }
    let msg = params.get("msg");
    if let Some(turn_id) = msg
        .and_then(|msg| msg.get("turnId"))
        .and_then(Value::as_str)
    {
        return Some(turn_id.to_string());
    }
    if let Some(turn_id) = msg
        .and_then(|msg| msg.get("turn_id"))
        .and_then(Value::as_str)
    {
        return Some(turn_id.to_string());
    }
    params
        .get("id")
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use serde_json::json;
    use tokio::time::sleep;

    use super::{
        RunSliceRequest, TerminalRunOutcome, apply_run_timeout, build_prompt_text,
        canonical_terminal_message,
    };

    #[test]
    fn canonical_terminal_message_prefers_item_completed() {
        let notifications = vec![json!({
            "method": "item/completed",
            "params": {
                "turnId": "turn-1",
                "item": {
                    "type": "agent_message",
                    "phase": "final_answer",
                    "text": "done",
                }
            }
        })];

        assert_eq!(
            canonical_terminal_message(&notifications, "turn-1").as_deref(),
            Some("done")
        );
    }

    #[test]
    fn canonical_terminal_message_falls_back_to_task_complete() {
        let notifications = vec![json!({
            "method": "task_complete",
            "params": {
                "turnId": "turn-1",
                "last_agent_message": "blocked"
            }
        })];

        assert_eq!(
            canonical_terminal_message(&notifications, "turn-1").as_deref(),
            Some("blocked")
        );
    }

    #[test]
    fn build_prompt_text_uses_repo_relative_slice_path() {
        let request = RunSliceRequest {
            worktree_path: PathBuf::from("/tmp/worktree"),
            slice_repo_relative_path: PathBuf::from("slices/01_slice.md"),
            prompt_template: "Prompt body".to_string(),
        };

        let text = build_prompt_text(&request);

        assert!(text.contains("slices/01_slice.md"));
    }

    #[tokio::test]
    async fn apply_run_timeout_returns_attention_needed_for_hung_run() {
        let outcome = apply_run_timeout(Duration::from_millis(10), async {
            sleep(Duration::from_millis(50)).await;
            Ok(TerminalRunOutcome::FinishedPromptOwned {
                terminal_message: "done".to_string(),
            })
        })
        .await
        .unwrap();

        match outcome {
            TerminalRunOutcome::AttentionNeeded { reason } => {
                assert!(reason.contains("timed out"));
            }
            other => panic!("unexpected outcome: {other:?}"),
        }
    }
}
