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
    subtitles,
};
use substream_core::subtitle;
use substream_protocol::video::{
    SubtitleSource, VideoDocument, VideoResult, VideoSource, VideoStage,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VideoConfig {
    pub output_dir: PathBuf,
    pub model: Option<PathBuf>,
    pub cookies_from_browser: Option<String>,
    pub cookies_file: Option<PathBuf>,
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
        for path in std::iter::once(&mut config.output_dir)
            .chain(config.model.iter_mut())
            .chain(config.cookies_file.iter_mut())
        {
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
        Ok(config)
    }

    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.cookies_file.is_none() || self.cookies_from_browser.is_none(),
            "choose either cookies_file or cookies_from_browser"
        );
        if let Some(file) = &self.cookies_file {
            ensure!(
                file.is_file(),
                "cookies file does not exist: {}",
                file.display()
            );
        }
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
    let downloader = YtDlp {
        executable: config.yt_dlp_bin.clone(),
        ffmpeg: config.ffmpeg_bin.clone(),
        cookies_from_browser: config.cookies_from_browser.clone(),
        cookies_file: config.cookies_file.clone(),
        timeout,
    };
    progress(VideoStage::CheckingSubtitles)?;
    let metadata = downloader.inspect(url, directory.path(), cancellation)?;
    let track = metadata.subtitle(&config.language);
    if track.is_none() {
        ensure!(
            config.model.as_ref().is_some_and(|model| model.is_file()),
            "no usable subtitles found; configure an existing Whisper model for recognition"
        );
    }
    progress(VideoStage::Downloading)?;
    let video = downloader.download(url, directory.path(), track.as_ref(), cancellation)?;
    let (mut transcript, subtitle_source) = if let Some(track) = track {
        progress(VideoStage::ImportingSubtitles)?;
        let path = video.subtitle_path(&track)?;
        let transcript = subtitles::read_srt(path, &track.language)?;
        fs::remove_file(path)?;
        (
            transcript,
            if track.automatic {
                SubtitleSource::Automatic
            } else {
                SubtitleSource::Provided
            },
        )
    } else {
        let wav = directory.path().join("audio.wav");
        progress(VideoStage::Converting)?;
        Ffmpeg {
            executable: config.ffmpeg_bin.clone(),
            timeout,
        }
        .decode(&video.filepath, &wav, cancellation)?;
        progress(VideoStage::Transcribing)?;
        let transcript = WhisperCpp {
            executable: config.whisper_bin.clone(),
            model: config
                .model
                .clone()
                .context("Whisper model is not configured")?,
            language: config.language.clone(),
            threads: config.threads,
            timeout,
        }
        .transcribe(&wav, cancellation)?;
        fs::remove_file(wav)?;
        (transcript, SubtitleSource::Recognition)
    };
    transcript.source = url.to_owned();
    let result = VideoResult {
        subtitle_source,
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
            id: metadata.id,
            title: metadata.title,
        },
        subtitle_source,
        transcript,
    };
    fs::write(&result.document, serde_json::to_vec_pretty(&document)?)?;
    ensure!(!cancellation.is_cancelled(), "video job cancelled");
    // Only a completed job keeps its private output directory. Failure removes partial files.
    let _ = directory.keep();
    Ok(result)
}
