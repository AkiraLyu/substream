//! Versioned control messages and a small, bounded binary audio format.

use serde::{Deserialize, Serialize};
use substream_core::{
    asr::BackendInfo,
    audio::{AudioChunk, MAX_CHUNK_SAMPLES, SAMPLE_RATE},
    pipeline::CaptionUpdate,
};
use thiserror::Error;

pub mod display;

pub const VERSION: u8 = 1;
pub const HEADER_BYTES: usize = 24;
pub const MAX_FRAME_BYTES: usize = HEADER_BYTES + MAX_CHUNK_SAMPLES * 2;

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ClientMessage {
    Authenticate {
        version: u8,
        token: String,
        /// Optional capture label, such as a browser tab title.
        #[serde(default)]
        source: Option<String>,
    },
    Finish,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Ready {
        version: u8,
        backend: BackendInfo,
    },
    Caption {
        caption: CaptionUpdate,
    },
    Finished {
        samples_processed: u64,
        processing_ms: u64,
    },
    Error {
        code: String,
        message: String,
    },
}

#[derive(Debug, Error)]
pub enum FrameError {
    #[error("invalid frame length")]
    Length,
    #[error("unsupported audio header; expected v1 mono 16 kHz PCM16LE")]
    Header,
    #[error(transparent)]
    Audio(#[from] substream_core::Error),
}

/// All integers are little endian. The format is specified in docs/protocol.md.
pub fn decode_audio(bytes: &[u8]) -> Result<AudioChunk, FrameError> {
    if bytes.len() <= HEADER_BYTES
        || bytes.len() > MAX_FRAME_BYTES
        || !(bytes.len() - HEADER_BYTES).is_multiple_of(2)
    {
        return Err(FrameError::Length);
    }
    if &bytes[..4] != b"SUBS"
        || bytes[4..8] != [VERSION, 1, 1, 0]
        || u32::from_le_bytes(bytes[8..12].try_into().expect("checked header")) != SAMPLE_RATE
    {
        return Err(FrameError::Header);
    }
    let sequence = u32::from_le_bytes(bytes[12..16].try_into().expect("checked header"));
    let start_sample = u64::from_le_bytes(bytes[16..24].try_into().expect("checked header"));
    let samples = bytes[HEADER_BYTES..]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|s| f32::from(i16::from_le_bytes([s[0], s[1]])) / 32768.0)
        .collect();
    Ok(AudioChunk::new(sequence, start_sample, samples)?)
}

pub fn encode_audio(chunk: &AudioChunk) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(HEADER_BYTES + chunk.samples().len() * 2);
    bytes.extend_from_slice(b"SUBS");
    bytes.extend_from_slice(&[VERSION, 1, 1, 0]);
    bytes.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    bytes.extend_from_slice(&chunk.sequence().to_le_bytes());
    bytes.extend_from_slice(&chunk.start_sample().to_le_bytes());
    for sample in chunk.samples() {
        let pcm = (sample * 32768.0).round().clamp(-32768.0, 32767.0) as i16;
        bytes.extend_from_slice(&pcm.to_le_bytes());
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_the_shared_browser_protocol_sample() {
        let frame = include_bytes!("../../../fixtures/audio-v1.bin");
        let chunk = decode_audio(frame).unwrap();
        assert_eq!(chunk.sequence(), 7);
        assert_eq!(chunk.start_sample(), 2_240);
        assert_eq!(chunk.samples(), &[-1.0, 0.0, 32767.0 / 32768.0]);
        assert_eq!(encode_audio(&chunk), frame);
    }
}
