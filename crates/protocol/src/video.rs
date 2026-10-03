//! Video job and document contracts shared by local and browser clients.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use substream_core::transcript::Transcript;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VideoRequest {
    pub url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VideoStage {
    Queued,
    Downloading,
    Converting,
    Transcribing,
    Cancelling,
    Completed,
    Cancelled,
    Failed,
}

impl VideoStage {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Cancelled | Self::Failed)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VideoSource {
    pub url: String,
    pub id: String,
    pub title: String,
}

/// Timestamped text and source metadata for downstream analysis or summarization.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VideoDocument {
    pub schema_version: u8,
    pub source: VideoSource,
    pub transcript: Transcript,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VideoResult {
    pub directory: PathBuf,
    pub video: PathBuf,
    pub srt: PathBuf,
    pub vtt: PathBuf,
    pub document: PathBuf,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VideoJob {
    pub id: String,
    pub stage: VideoStage,
    pub url: String,
    pub result: Option<VideoResult>,
    pub error: Option<String>,
}
