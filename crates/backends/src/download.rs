//! yt-dlp adapter. Browser cookies remain in the local downloader process.

use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use url::Url;

use crate::process::{self, Cancellation};

pub fn validate_video_url(value: &str) -> Result<()> {
    ensure!(value.len() <= 8192, "video URL exceeds 8192 bytes");
    let url = Url::parse(value).context("invalid video URL")?;
    ensure!(
        matches!(url.scheme(), "http" | "https") && url.has_host(),
        "video URL must use http or https"
    );
    ensure!(
        url.username().is_empty() && url.password().is_none(),
        "video URL must not contain credentials"
    );
    Ok(())
}

pub struct YtDlp {
    pub executable: PathBuf,
    pub ffmpeg: PathBuf,
    pub cookies_from_browser: Option<String>,
    pub timeout: Duration,
}

#[derive(Debug, Deserialize)]
pub struct DownloadedVideo {
    pub id: String,
    pub title: String,
    pub filepath: PathBuf,
}

impl YtDlp {
    pub fn download(
        &self,
        url: &str,
        directory: &Path,
        cancellation: &Cancellation,
    ) -> Result<DownloadedVideo> {
        validate_video_url(url)?;
        let directory = directory.canonicalize()?;
        let metadata = directory.join("download.json");
        ensure!(
            !metadata.exists(),
            "download directory already contains a result"
        );
        let mut command = Command::new(&self.executable);
        command.current_dir(&directory).args([
            "--ignore-config",
            "--no-plugin-dirs",
            "--no-playlist",
            "--playlist-items",
            "1",
            "--no-progress",
            "--no-cache-dir",
            "--no-overwrites",
            "--no-simulate",
            "--match-filters",
            "!is_live & !is_upcoming",
            "--format",
            "bv*+ba/b",
            "--merge-output-format",
            "mkv",
            "--output",
            "media.%(ext)s",
            "--print-to-file",
            r#"after_move:{"id":%(id)j,"title":%(title)j,"filepath":%(filepath)j}"#,
            "download.json",
        ]);
        if self.ffmpeg != Path::new("ffmpeg") {
            command.arg("--ffmpeg-location").arg(&self.ffmpeg);
        }
        if let Some(browser) = &self.cookies_from_browser {
            command.arg("--cookies-from-browser").arg(browser);
        }
        command.arg("--").arg(url);
        process::run(&mut command, self.timeout, cancellation)
            .context("download video with yt-dlp")?;
        let mut bytes = Vec::new();
        fs::File::open(&metadata)
            .context("yt-dlp did not produce a completed video")?
            .take(128 * 1024 + 1)
            .read_to_end(&mut bytes)?;
        ensure!(bytes.len() <= 128 * 1024, "video metadata exceeds 128 KiB");
        let mut result: DownloadedVideo =
            serde_json::from_slice(&bytes).context("invalid yt-dlp result")?;
        result.filepath = directory
            .join(&result.filepath)
            .canonicalize()
            .context("downloaded video is missing")?;
        ensure!(
            result.filepath.is_file() && result.filepath.parent() == Some(directory.as_path()),
            "downloaded video must stay in the job directory"
        );
        fs::remove_file(metadata)?;
        Ok(result)
    }
}
