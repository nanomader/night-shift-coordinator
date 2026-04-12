use std::fmt::{Display, Formatter};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub const MAX_PARALLEL_SLICES: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SliceCandidate {
    pub path: PathBuf,
    pub label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SliceStatus {
    Queued,
    Starting,
    Running,
    AttentionNeeded,
    FinishedPromptOwned,
}

impl Display for SliceStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            SliceStatus::Queued => "queued",
            SliceStatus::Starting => "starting",
            SliceStatus::Running => "running",
            SliceStatus::AttentionNeeded => "attention_needed",
            SliceStatus::FinishedPromptOwned => "finished_prompt_owned",
        };
        f.write_str(value)
    }
}
