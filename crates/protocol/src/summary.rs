//! Summary requests contain prompt overrides, never provider credentials.

use crate::video::VideoSource;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SummaryRequest {
    pub system_prompt: Option<String>,
    pub user_prompt: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SummaryResult {
    pub schema_version: u8,
    pub source: VideoSource,
    pub model: String,
    pub markdown: String,
}
