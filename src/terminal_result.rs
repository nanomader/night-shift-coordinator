use serde::{Deserialize, Serialize};

use crate::error::{NscError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProgressPhase {
    Planning,
    Implementing,
    WaitingForReview,
    FixingReviewFindings,
    WaitingForMerge,
}

impl ProgressPhase {
    pub fn as_marker_str(self) -> &'static str {
        match self {
            Self::Planning => "PLANNING",
            Self::Implementing => "IMPLEMENTING",
            Self::WaitingForReview => "WAITING_FOR_REVIEW",
            Self::FixingReviewFindings => "FIXING_REVIEW_FINDINGS",
            Self::WaitingForMerge => "WAITING_FOR_MERGE",
        }
    }

    pub fn as_display_str(self) -> &'static str {
        match self {
            Self::Planning => "planning",
            Self::Implementing => "implementing",
            Self::WaitingForReview => "waiting_for_review",
            Self::FixingReviewFindings => "fixing_review_findings",
            Self::WaitingForMerge => "waiting_for_merge",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalResultKind {
    MergedSuccess,
    ManualFollowup,
    Blocked,
    Stopped,
}

impl TerminalResultKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MergedSuccess => "MERGED_SUCCESS",
            Self::ManualFollowup => "MANUAL_FOLLOWUP",
            Self::Blocked => "BLOCKED",
            Self::Stopped => "STOPPED",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalResultMarkers {
    pub result: TerminalResultKind,
    pub merged_main_sha: Option<String>,
    pub pr_number: Option<u64>,
}

pub fn parse_progress_marker(text: &str) -> Option<ProgressPhase> {
    let mut latest = None;

    for line in text.lines().map(str::trim) {
        let Some(value) = line.strip_prefix("NSC_PROGRESS:") else {
            continue;
        };
        latest = match value.trim() {
            "PLANNING" => Some(ProgressPhase::Planning),
            "IMPLEMENTING" => Some(ProgressPhase::Implementing),
            "WAITING_FOR_REVIEW" => Some(ProgressPhase::WaitingForReview),
            "FIXING_REVIEW_FINDINGS" => Some(ProgressPhase::FixingReviewFindings),
            "WAITING_FOR_MERGE" => Some(ProgressPhase::WaitingForMerge),
            _ => latest,
        };
    }

    latest
}

pub fn parse_terminal_result_markers(text: &str) -> Result<Option<TerminalResultMarkers>> {
    let mut saw_any_marker = false;
    let mut result = None;
    let mut merged_main_sha = None;
    let mut pr_number = None;

    for line in text.lines().map(str::trim) {
        if let Some(value) = line.strip_prefix("NSC_RESULT:") {
            saw_any_marker = true;
            let value = value.trim();
            let parsed = match value {
                "MERGED_SUCCESS" => TerminalResultKind::MergedSuccess,
                "MANUAL_FOLLOWUP" => TerminalResultKind::ManualFollowup,
                "BLOCKED" => TerminalResultKind::Blocked,
                "STOPPED" => TerminalResultKind::Stopped,
                _ => {
                    return Err(NscError::InvalidTerminalResult(format!(
                        "unknown NSC_RESULT value: {value}"
                    )));
                }
            };
            if result.replace(parsed).is_some() {
                return Err(NscError::InvalidTerminalResult(
                    "multiple NSC_RESULT lines found".to_string(),
                ));
            }
            continue;
        }

        if let Some(value) = line.strip_prefix("NSC_MERGED_MAIN_SHA:") {
            saw_any_marker = true;
            let value = value.trim();
            if value.len() != 40 || !value.chars().all(|ch| ch.is_ascii_hexdigit()) {
                return Err(NscError::InvalidTerminalResult(format!(
                    "invalid NSC_MERGED_MAIN_SHA value: {value}"
                )));
            }
            if merged_main_sha.replace(value.to_string()).is_some() {
                return Err(NscError::InvalidTerminalResult(
                    "multiple NSC_MERGED_MAIN_SHA lines found".to_string(),
                ));
            }
            continue;
        }

        if let Some(value) = line.strip_prefix("NSC_PR:") {
            saw_any_marker = true;
            let value = value.trim();
            let parsed = value.parse::<u64>().map_err(|_| {
                NscError::InvalidTerminalResult(format!("invalid NSC_PR value: {value}"))
            })?;
            if pr_number.replace(parsed).is_some() {
                return Err(NscError::InvalidTerminalResult(
                    "multiple NSC_PR lines found".to_string(),
                ));
            }
        }
    }

    if !saw_any_marker {
        return Ok(None);
    }

    let Some(result) = result else {
        return Err(NscError::InvalidTerminalResult(
            "marker output is missing NSC_RESULT".to_string(),
        ));
    };

    Ok(Some(TerminalResultMarkers {
        result,
        merged_main_sha,
        pr_number,
    }))
}
