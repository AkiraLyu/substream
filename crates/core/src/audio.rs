use crate::{Error, Result};

pub const SAMPLE_RATE: u32 = 16_000;
pub const FRAME_SAMPLES: usize = 320;
pub const MAX_CHUNK_SAMPLES: usize = 3_200;

/// Mono f32 PCM at 16 kHz. Offsets count samples, never wall-clock milliseconds.
#[derive(Debug, Clone)]
pub struct AudioChunk {
    sequence: u32,
    start_sample: u64,
    samples: Vec<f32>,
}

impl AudioChunk {
    pub fn new(sequence: u32, start_sample: u64, samples: Vec<f32>) -> Result<Self> {
        if samples.is_empty() || samples.len() > MAX_CHUNK_SAMPLES {
            return Err(Error::Audio("chunks must contain 1..=3200 samples".into()));
        }
        if samples
            .iter()
            .any(|s| !s.is_finite() || !(-1.0..=1.0).contains(s))
        {
            return Err(Error::Audio(
                "PCM must be finite and normalized to [-1, 1]".into(),
            ));
        }
        if start_sample.checked_add(samples.len() as u64).is_none() {
            return Err(Error::Audio("sample clock overflow".into()));
        }
        Ok(Self {
            sequence,
            start_sample,
            samples,
        })
    }

    pub fn sequence(&self) -> u32 {
        self.sequence
    }
    pub fn start_sample(&self) -> u64 {
        self.start_sample
    }
    pub fn end_sample(&self) -> u64 {
        self.start_sample + self.samples.len() as u64
    }
    pub fn samples(&self) -> &[f32] {
        &self.samples
    }
}

pub const fn samples_to_ms(samples: u64) -> u64 {
    samples / (SAMPLE_RATE as u64 / 1_000)
}
