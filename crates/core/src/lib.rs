//! Model-independent audio, recognition, and subtitle contracts.
//!
//! No executor, device, GUI, or inference runtime belongs in this crate.

pub mod asr;
pub mod audio;
pub mod pipeline;
pub mod stabilizer;
pub mod subtitle;
pub mod transcript;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid audio: {0}")]
    Audio(String),
    #[error("invalid transcript: {0}")]
    Transcript(String),
    #[error("recognition backend: {0}")]
    Backend(String),
    #[error("invalid stream state: {0}")]
    State(String),
}

pub type Result<T> = std::result::Result<T, Error>;
