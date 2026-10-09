//! yt-dlp metadata and download adapter. Cookies stay in the local process.

use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, de::DeserializeOwned};
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
    pub cookies_file: Option<PathBuf>,
    pub timeout: Duration,
}

#[derive(Debug, Deserialize)]
pub struct VideoMetadata {
    pub id: String,
    pub title: String,
    language: Option<String>,
    subtitles: Option<BTreeMap<String, Vec<SubtitleFormat>>>,
    automatic_captions: Option<BTreeMap<String, Vec<SubtitleFormat>>>,
}

#[derive(Debug, Deserialize)]
struct SubtitleFormat {
    ext: Option<String>,
}

#[derive(Debug)]
pub struct SubtitleTrack {
    pub language: String,
    pub automatic: bool,
}

impl VideoMetadata {
    /// Prefer provided captions, then automatic captions in the requested language.
    /// Auto uses the original language when known, then English, then another track.
    pub fn subtitle(&self, language: &str) -> Option<SubtitleTrack> {
        for (automatic, tracks) in [(false, &self.subtitles), (true, &self.automatic_captions)] {
            let Some(tracks) = tracks else { continue };
            let available: Vec<_> = tracks
                .iter()
                .filter(|(name, formats)| {
                    !matches!(name.as_str(), "live_chat" | "danmaku")
                        && formats.iter().any(|format| {
                            matches!(
                                format.ext.as_deref(),
                                Some("srt" | "vtt" | "ttml" | "ass" | "ssa")
                            )
                        })
                })
                .map(|(name, _)| name.as_str())
                .collect();
            let find = |wanted: &str| {
                available
                    .iter()
                    .copied()
                    .find(|name| *name == wanted)
                    .or_else(|| {
                        available.iter().copied().find(|name| {
                            name.split(['-', '_']).next() == wanted.split(['-', '_']).next()
                        })
                    })
            };
            let chosen = if language == "auto" {
                self.language
                    .as_deref()
                    .and_then(find)
                    .or_else(|| {
                        available
                            .iter()
                            .copied()
                            .find(|name| name.ends_with("-orig"))
                    })
                    .or_else(|| find("en"))
                    .or_else(|| available.first().copied())
            } else {
                find(language)
            };
            if let Some(language) = chosen {
                return Some(SubtitleTrack {
                    language: language.to_owned(),
                    automatic,
                });
            }
        }
        None
    }
}

#[derive(Debug, Deserialize)]
pub struct DownloadedVideo {
    pub filepath: PathBuf,
    requested_subtitles: Option<BTreeMap<String, DownloadedSubtitle>>,
}

#[derive(Debug, Deserialize)]
struct DownloadedSubtitle {
    filepath: PathBuf,
}

impl DownloadedVideo {
    pub fn subtitle_path(&self, track: &SubtitleTrack) -> Result<&Path> {
        self.requested_subtitles
            .as_ref()
            .and_then(|tracks| tracks.get(&track.language))
            .map(|subtitle| subtitle.filepath.as_path())
            .context("yt-dlp did not download the selected subtitle track")
    }
}

impl YtDlp {
    fn command(&self, directory: &Path) -> Result<Command> {
        let mut command = Command::new(&self.executable);
        command.current_dir(directory).args([
            "--ignore-config",
            "--no-plugin-dirs",
            "--no-playlist",
            "--playlist-items",
            "1",
            "--no-progress",
            "--no-cache-dir",
            "--no-overwrites",
            "--match-filters",
            "!is_live & !is_upcoming",
            "--output",
            "media.%(ext)s",
        ]);
        if self.ffmpeg != Path::new("ffmpeg") {
            command.arg("--ffmpeg-location").arg(&self.ffmpeg);
        }
        if let Some(browser) = &self.cookies_from_browser {
            command.arg("--cookies-from-browser").arg(browser);
        }
        if let Some(file) = &self.cookies_file {
            // yt-dlp writes its cookie jar back on exit. Never modify the user's source file.
            let jar = directory.join("cookies.txt");
            if !jar.exists() {
                fs::copy(file, &jar).context("copy cookies into the private job directory")?;
            }
            command.arg("--cookies").arg(jar);
        }
        Ok(command)
    }

    pub fn inspect(
        &self,
        url: &str,
        directory: &Path,
        cancellation: &Cancellation,
    ) -> Result<VideoMetadata> {
        validate_video_url(url)?;
        let mut command = self.command(directory)?;
        command
            .args([
                "--skip-download",
                "--print-to-file",
                "%()j",
                "probe.json",
                "--",
            ])
            .arg(url);
        process::run(&mut command, self.timeout, cancellation)
            .context("check video subtitles with yt-dlp")?;
        read_metadata(&directory.join("probe.json"))
    }

    pub fn download(
        &self,
        url: &str,
        directory: &Path,
        subtitle: Option<&SubtitleTrack>,
        cancellation: &Cancellation,
    ) -> Result<DownloadedVideo> {
        validate_video_url(url)?;
        let directory = directory.canonicalize()?;
        let mut command = self.command(&directory)?;
        command.args([
            "--no-simulate",
            "--format",
            "bv*+ba/b",
            "--merge-output-format",
            "mkv",
            "--print-to-file",
            "after_move:%()j",
            "download.json",
        ]);
        if let Some(track) = subtitle {
            command
                .arg(if track.automatic {
                    "--write-auto-subs"
                } else {
                    "--write-subs"
                })
                .arg("--sub-langs")
                .arg(format!("^{}$", regex::escape(&track.language)))
                .args([
                    "--sub-format",
                    "srt/vtt/ttml/ass/ssa",
                    "--convert-subs",
                    "srt",
                ]);
        }
        command.arg("--").arg(url);
        process::run(&mut command, self.timeout, cancellation)
            .context("download video with yt-dlp")?;
        let mut result: DownloadedVideo = read_metadata(&directory.join("download.json"))?;
        result.filepath = local_file(&directory, &result.filepath)?;
        if let Some(tracks) = &mut result.requested_subtitles {
            for track in tracks.values_mut() {
                track.filepath = local_file(&directory, &track.filepath)?;
            }
        }
        // A successful result never retains the private cookie jar.
        let jar = directory.join("cookies.txt");
        if jar.exists() {
            fs::remove_file(jar)?;
        }
        Ok(result)
    }
}

fn local_file(directory: &Path, path: &Path) -> Result<PathBuf> {
    let path = directory
        .join(path)
        .canonicalize()
        .context("downloaded file is missing")?;
    ensure!(
        path.is_file() && path.parent() == Some(directory),
        "downloaded file must stay in the job directory"
    );
    Ok(path)
}

fn read_metadata<T: DeserializeOwned>(path: &Path) -> Result<T> {
    const MAX_JSON: u64 = 32 * 1024 * 1024;
    let mut bytes = Vec::new();
    fs::File::open(path)
        .context("yt-dlp did not produce metadata")?
        .take(MAX_JSON + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= MAX_JSON,
        "video metadata exceeds 32 MiB"
    );
    let result = serde_json::from_slice(&bytes).context("invalid yt-dlp metadata")?;
    fs::remove_file(path)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_provided_subtitles_then_automatic_captions_in_the_requested_language() {
        let metadata: VideoMetadata = serde_json::from_value(serde_json::json!({
            "id": "clip", "title": "Clip", "language": "ja",
            "subtitles": {"en-US": [{"ext": "vtt"}], "ja": [{"ext": "srt"}],
                          "live_chat": [{"ext": "json"}]},
            "automatic_captions": {"ja": [{"ext": "vtt"}], "zh-Hans": [{"ext": "vtt"}]}
        }))
        .unwrap();
        let original = metadata.subtitle("auto").unwrap();
        assert_eq!(original.language, "ja");
        assert!(!original.automatic);
        assert_eq!(metadata.subtitle("en").unwrap().language, "en-US");
        let automatic = metadata.subtitle("zh").unwrap();
        assert_eq!(automatic.language, "zh-Hans");
        assert!(automatic.automatic);
        assert!(metadata.subtitle("fr").is_none());
    }
}
