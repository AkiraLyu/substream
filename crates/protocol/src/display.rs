//! Desktop-neutral snapshots for caption renderers.

use serde::{Deserialize, Serialize};
use substream_core::{asr::BackendInfo, pipeline::CaptionUpdate};

use crate::VERSION;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DisplayStatus {
    #[default]
    Idle,
    Loading,
    Listening,
    Finished,
    Error,
}

/// A complete view of the latest session, independent of any desktop toolkit.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename = "display")]
pub struct DisplayState {
    pub version: u8,
    pub session_id: u64,
    pub status: DisplayStatus,
    pub backend: Option<BackendInfo>,
    pub source: Option<String>,
    pub samples_received: u64,
    pub caption: Option<CaptionUpdate>,
    /// Age of the caption at delivery, so reconnecting does not revive old text.
    pub caption_age_ms: Option<u64>,
    pub message: Option<String>,
}

impl Default for DisplayState {
    fn default() -> Self {
        Self {
            version: VERSION,
            session_id: 0,
            status: DisplayStatus::Idle,
            backend: None,
            source: None,
            samples_received: 0,
            caption: None,
            caption_age_ms: None,
            message: None,
        }
    }
}
