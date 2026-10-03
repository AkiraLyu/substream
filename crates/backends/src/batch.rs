use std::{
    fs::File,
    io::Read,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use substream_core::transcript::{Segment, Transcript};

use crate::process::{self, Cancellation};

/// Recognizes a mono 16 kHz WAV file and returns timed subtitle segments.
pub trait BatchRecognizer {
    fn transcribe(&self, wav: &Path, cancellation: &Cancellation) -> Result<Transcript>;
}

pub struct Ffmpeg {
    pub executable: PathBuf,
    pub timeout: Duration,
}

impl Ffmpeg {
    /// Converts a local media file to mono 16 kHz WAV.
    pub fn decode(&self, input: &Path, output: &Path, cancellation: &Cancellation) -> Result<()> {
        let input = input.canonicalize().context("open local media input")?;
        ensure!(input.is_file(), "media input must be a regular file");
        let mut command = Command::new(&self.executable);
        command
            .args([
                "-nostdin",
                "-hide_banner",
                "-loglevel",
                "error",
                "-n",
                "-protocol_whitelist",
                "file,pipe",
                "-i",
            ])
            .arg(input)
            .args([
                "-map",
                "0:a:0",
                "-vn",
                "-ac",
                "1",
                "-ar",
                "16000",
                "-c:a",
                "pcm_s16le",
                "-f",
                "wav",
            ])
            .arg(output);
        process::run(&mut command, self.timeout, cancellation).context("FFmpeg normalization")
    }
}

pub struct WhisperCpp {
    pub executable: PathBuf,
    pub model: PathBuf,
    pub language: String,
    pub threads: u16,
    pub timeout: Duration,
}

impl BatchRecognizer for WhisperCpp {
    fn transcribe(&self, wav: &Path, cancellation: &Cancellation) -> Result<Transcript> {
        ensure!(
            self.model.is_file(),
            "Whisper model does not exist: {}",
            self.model.display()
        );
        ensure!(
            (1..=64).contains(&self.threads),
            "Whisper threads must be 1..=64"
        );
        ensure!(
            !self.language.is_empty(),
            "language must be a language code or auto"
        );
        let directory = tempfile::Builder::new()
            .permissions(std::fs::Permissions::from_mode(0o700))
            .tempdir()
            .context("create Whisper output directory")?;
        let prefix = directory.path().join("transcript");
        let mut command = Command::new(&self.executable);
        command
            .arg("--model")
            .arg(&self.model)
            .arg("--file")
            .arg(wav)
            .arg("--language")
            .arg(&self.language)
            .arg("--threads")
            .arg(self.threads.to_string())
            .arg("--output-json")
            .arg("--output-file")
            .arg(&prefix);
        process::run(&mut command, self.timeout, cancellation).context("whisper.cpp inference")?;
        let file = File::open(prefix.with_extension("json"))
            .context("whisper-cli did not produce JSON")?;
        const MAX_JSON: u64 = 16 * 1024 * 1024;
        let mut bytes = Vec::new();
        file.take(MAX_JSON + 1).read_to_end(&mut bytes)?;
        ensure!(
            bytes.len() as u64 <= MAX_JSON,
            "Whisper JSON exceeds 16 MiB"
        );
        parse_whisper_json(&bytes)
    }
}

#[derive(Deserialize)]
struct WhisperOutput {
    result: WhisperResult,
    transcription: Vec<WhisperSegment>,
}

#[derive(Deserialize)]
struct WhisperResult {
    language: String,
}

#[derive(Deserialize)]
struct WhisperSegment {
    offsets: Offsets,
    text: String,
}

#[derive(Deserialize)]
struct Offsets {
    from: u64,
    to: u64,
}

pub fn parse_whisper_json(bytes: &[u8]) -> Result<Transcript> {
    let output: WhisperOutput =
        serde_json::from_slice(bytes).context("invalid whisper.cpp JSON schema")?;
    let segments = output
        .transcription
        .into_iter()
        .filter(|s| !s.text.trim().is_empty())
        .enumerate()
        .map(|(id, segment)| Segment {
            id: id as u64,
            start_ms: segment.offsets.from,
            end_ms: segment.offsets.to,
            text: segment.text.trim().to_owned(),
        })
        .collect();
    let transcript = Transcript {
        schema_version: 1,
        source: "whisper.cpp".into(),
        language: Some(output.result.language),
        segments,
    };
    transcript.validate()?;
    Ok(transcript)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_text_timing_and_language_and_rejects_invalid_timing() {
        let transcript =
            parse_whisper_json(include_bytes!("../../../fixtures/whisper-output.json")).unwrap();
        assert_eq!(transcript.segments[0].end_ms, 1230);
        assert_eq!(transcript.segments[0].text, "こんにちは。");
        assert_eq!(transcript.language.as_deref(), Some("ja"));
        assert!(parse_whisper_json(br#"{"result":{"language":"en"},"transcription":[{"text":"bad","offsets":{"from":20,"to":10}}]}"#).is_err());
    }
}
