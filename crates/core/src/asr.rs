use serde::{Deserialize, Serialize};

use crate::{Result, audio::AudioChunk, transcript::Segment};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendInfo {
    pub name: String,
    /// Encoder model path identifying the loaded model.
    pub model: String,
    pub threads: i32,
    /// Model-specific; an empty list means unspecified, not every language.
    pub languages: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Hypothesis {
    pub segment: Segment,
    pub is_final: bool,
}

/// A recognizer is owned by one worker. Inference is synchronous and must not
/// run inside an audio callback, UI callback, or async reactor task.
pub trait StreamingRecognizer {
    fn info(&self) -> BackendInfo;
    fn push(&mut self, chunk: &AudioChunk) -> Result<Vec<Hypothesis>>;
    /// Flush pending speech at EOS, including a final short utterance.
    fn finish(&mut self) -> Result<Vec<Hypothesis>>;
}
