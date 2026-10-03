//! A complete local video job, independent of its CLI or HTTP entry point.

use std::{
    fs,
    io::Read,
    num::NonZeroUsize,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use substream_backends::{
    batch::{BatchRecognizer, Ffmpeg, WhisperCpp},
    download::{YtDlp, validate_video_url},
    process::Cancellation,
};
use substream_core::subtitle;
use substream_protocol::video::{VideoDocument, VideoResult, VideoSource, VideoStage};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VideoConfig {
    pub output_dir: PathBuf,
    pub model: PathBuf,
    pub cookies_from_browser: Option<String>,
    #[serde(default = "yt_dlp")]
    pub yt_dlp_bin: PathBuf,
    #[serde(default = "whisper")]
    pub whisper_bin: PathBuf,
    #[serde(default = "ffmpeg")]
    pub ffmpeg_bin: PathBuf,
    #[serde(default = "language")]
    pub language: String,
    #[serde(default = "threads")]
    pub threads: u16,
    #[serde(default = "timeout")]
    pub timeout_secs: u64,
}

fn yt_dlp() -> PathBuf {
    "yt-dlp".into()
}
fn whisper() -> PathBuf {
    "whisper-cli".into()
}
fn ffmpeg() -> PathBuf {
    "ffmpeg".into()
}
fn language() -> String {
    "auto".into()
}
fn threads() -> u16 {
    4
}
fn timeout() -> u64 {
    3600
}

impl VideoConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let path = path.canonicalize().context("open video configuration")?;
        let mut config: Self =
            toml::from_str(&fs::read_to_string(&path)?).context("parse video configuration")?;
        let base = path.parent().context("configuration has no parent")?;
        for path in [&mut config.model, &mut config.output_dir] {
            if path.is_relative() {
                *path = base.join(&*path);
            }
        }
        for path in [
            &mut config.yt_dlp_bin,
            &mut config.whisper_bin,
            &mut config.ffmpeg_bin,
        ] {
            if path.is_relative() && path.components().count() > 1 {
                *path = base.join(&*path);
            }
        }
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<()> {
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
            (1..=86400).contains(&self.timeout_secs),
            "stage timeout must be 1..=86400 seconds"
        );
        ensure!(
            !self.language.is_empty(),
            "language must be a language code or auto"
        );
        if let Some(browser) = &self.cookies_from_browser {
            let name = browser.split(['+', ':']).next().unwrap_or_default();
            ensure!(
                matches!(
                    name,
                    "brave"
                        | "chrome"
                        | "chromium"
                        | "edge"
                        | "firefox"
                        | "opera"
                        | "safari"
                        | "vivaldi"
                        | "whale"
                ),
                "unsupported cookies_from_browser selector"
            );
        }
        Ok(())
    }
}

pub fn new_job_id() -> Result<String> {
    let mut bytes = [0_u8; 16];
    fs::File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

pub fn run_video(
    config: &VideoConfig,
    url: &str,
    cancellation: &Cancellation,
    mut progress: impl FnMut(VideoStage) -> Result<()>,
) -> Result<VideoResult> {
    config.validate()?;
    validate_video_url(url)?;
    ensure!(!cancellation.is_cancelled(), "video job cancelled");
    fs::create_dir_all(&config.output_dir).context("create video output directory")?;
    let root = config.output_dir.canonicalize()?;
    let directory = tempfile::Builder::new()
        .prefix("video-")
        .permissions(fs::Permissions::from_mode(0o700))
        .tempdir_in(root)?;
    let timeout = Duration::from_secs(config.timeout_secs);
    progress(VideoStage::Downloading)?;
    let video = YtDlp {
        executable: config.yt_dlp_bin.clone(),
        ffmpeg: config.ffmpeg_bin.clone(),
        cookies_from_browser: config.cookies_from_browser.clone(),
        timeout,
    }
    .download(url, directory.path(), cancellation)?;
    let wav = directory.path().join("audio.wav");
    progress(VideoStage::Converting)?;
    Ffmpeg {
        executable: config.ffmpeg_bin.clone(),
        timeout,
    }
    .decode(&video.filepath, &wav, cancellation)?;
    progress(VideoStage::Transcribing)?;
    let mut transcript = WhisperCpp {
        executable: config.whisper_bin.clone(),
        model: config.model.clone(),
        language: config.language.clone(),
        threads: config.threads,
        timeout,
    }
    .transcribe(&wav, cancellation)?;
    transcript.source = url.to_owned();
    let result = VideoResult {
        directory: directory.path().to_owned(),
        video: video.filepath,
        srt: directory.path().join("subtitles.srt"),
        vtt: directory.path().join("subtitles.vtt"),
        document: directory.path().join("document.json"),
    };
    let width = NonZeroUsize::new(42).expect("positive width");
    fs::write(
        &result.srt,
        subtitle::render(&transcript, subtitle::Format::Srt, width)?,
    )?;
    fs::write(
        &result.vtt,
        subtitle::render(&transcript, subtitle::Format::Vtt, width)?,
    )?;
    let document = VideoDocument {
        schema_version: 1,
        source: VideoSource {
            url: url.to_owned(),
            id: video.id,
            title: video.title,
        },
        transcript,
    };
    fs::write(&result.document, serde_json::to_vec_pretty(&document)?)?;
    fs::remove_file(wav)?;
    ensure!(!cancellation.is_cancelled(), "video job cancelled");
    // Only a completed job keeps its private output directory. Failure removes partial files.
    let _ = directory.keep();
    Ok(result)
}
