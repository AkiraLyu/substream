use serde::{Deserialize, Serialize};

use crate::{Error, Result};

pub const MAX_TEXT_BYTES: usize = 16_384;

/// Segment-level timing, not fabricated word alignment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Segment {
    pub id: u64,
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
}

impl Segment {
    pub fn validate(&self) -> Result<()> {
        if self.start_ms >= self.end_ms {
            return Err(Error::Transcript(
                "segment end must follow its start".into(),
            ));
        }
        if self.text.trim().is_empty() || self.text.len() > MAX_TEXT_BYTES {
            return Err(Error::Transcript(
                "segment text is empty or too large".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transcript {
    pub schema_version: u32,
    pub source: String,
    /// A language tag supplied by the backend, or None when unknown.
    pub language: Option<String>,
    /// Explicit provenance: synthetic transcripts must never look like real ASR.
    pub synthetic: bool,
    pub segments: Vec<Segment>,
}

impl Transcript {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != 1 {
            return Err(Error::Transcript("unsupported schema version".into()));
        }
        let mut previous: Option<&Segment> = None;
        for segment in &self.segments {
            segment.validate()?;
            if let Some(prev) = previous
                && (segment.id <= prev.id || segment.start_ms < prev.end_ms)
            {
                return Err(Error::Transcript(
                    "segments must have increasing IDs and non-overlapping times".into(),
                ));
            }
            previous = Some(segment);
        }
        Ok(())
    }
}
