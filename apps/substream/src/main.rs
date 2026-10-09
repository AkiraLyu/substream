use std::{
    io::{self, Write},
    net::SocketAddr,
    num::NonZeroUsize,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use substream::{
    auth::{Token, create_token},
    config::RecognizerConfig,
    server::{ServerConfig, serve},
    video::{VideoConfig, new_job_id, run_video},
};
use substream_backends::{
    batch::{BatchRecognizer, Ffmpeg, WhisperCpp},
    pcm::PcmReader,
    process::Cancellation,
};
use substream_core::{pipeline::LivePipeline, subtitle, transcript::Transcript};
use substream_protocol::{
    ServerMessage, VERSION,
    video::{VideoJob, VideoStage},
};

#[derive(Parser)]
#[command(version, about = "Local speech recognition and subtitles")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Read raw mono 16 kHz PCM16LE from stdin; emit NDJSON caption events.
    Stream {
        #[command(flatten)]
        recognizer: Recognizer,
    },
    /// Serve authenticated live audio and video jobs on loopback.
    Serve {
        #[arg(long, default_value = "127.0.0.1:9743")]
        listen: SocketAddr,
        #[arg(long)]
        token_file: PathBuf,
        /// Exact browser extension origin; repeat for more than one extension.
        #[arg(long = "allow-origin")]
        origins: Vec<String>,
        /// Streaming model configuration. Omit for a video-only service.
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long, value_parser = clap::value_parser!(i32).range(1..=64))]
        threads: Option<i32>,
        /// Enable complete video jobs using a local configuration.
        #[arg(long)]
        video_config: Option<PathBuf>,
    },
    /// Download one video and generate SRT, VTT, and a timestamped JSON document.
    Video {
        url: String,
        #[arg(long)]
        config: PathBuf,
        /// Use a Netscape cookies file instead of the configured cookies source.
        #[arg(long, conflicts_with_all = ["cookies_from_browser", "no_cookies"])]
        cookies_file: Option<PathBuf>,
        /// Read cookies from a browser, optionally with a profile selector.
        #[arg(long, conflicts_with = "no_cookies")]
        cookies_from_browser: Option<String>,
        /// Download anonymously, ignoring any configured cookies source.
        #[arg(long)]
        no_cookies: bool,
    },
    /// Create a private 256-bit authentication token file without overwriting.
    Token {
        #[arg(long)]
        output: PathBuf,
    },
    /// Normalize a local media file with FFmpeg and transcribe with whisper.cpp.
    Transcribe {
        input: PathBuf,
        #[arg(long)]
        model: PathBuf,
        #[arg(long, default_value = "whisper-cli")]
        whisper_bin: PathBuf,
        #[arg(long, default_value = "ffmpeg")]
        ffmpeg_bin: PathBuf,
        #[arg(long, default_value = "auto")]
        language: String,
        #[arg(long, default_value_t = 4, value_parser = clap::value_parser!(u16).range(1..=64))]
        threads: u16,
        #[arg(long, default_value_t = 3600, value_parser = clap::value_parser!(u64).range(1..))]
        timeout_secs: u64,
        #[command(flatten)]
        output: Output,
    },
}

#[derive(Args)]
struct Recognizer {
    #[arg(long)]
    config: PathBuf,
    /// Override the inference thread count from the model configuration.
    #[arg(long, value_parser = clap::value_parser!(i32).range(1..=64))]
    threads: Option<i32>,
}

impl Recognizer {
    fn load(&self) -> Result<RecognizerConfig> {
        RecognizerConfig::load(&self.config, self.threads)
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Srt,
    Vtt,
    Json,
}

#[derive(Args)]
struct Output {
    #[arg(long, value_enum, default_value = "srt")]
    format: Format,
    /// Omit to write to stdout. Existing files are never overwritten.
    #[arg(long)]
    output: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "substream=info".into()),
        )
        .init();
    match Cli::parse().command {
        Command::Token { output } => {
            create_token(&output)?;
            eprintln!("Token created: {}", output.display());
            Ok(())
        }
        Command::Stream { recognizer } => stream(recognizer.load()?),
        Command::Serve {
            listen,
            token_file,
            origins,
            config,
            threads,
            video_config,
        } => {
            anyhow::ensure!(
                config.is_some() || video_config.is_some(),
                "serve requires --config or --video-config"
            );
            serve(ServerConfig {
                address: listen,
                token: Token::read(&token_file)?,
                allowed_origins: origins,
                recognizer: config
                    .as_deref()
                    .map(|path| {
                        RecognizerConfig::load(path, threads).map(RecognizerConfig::factory)
                    })
                    .transpose()?,
                video: video_config.as_deref().map(VideoConfig::load).transpose()?,
            })
            .await
        }
        Command::Video {
            url,
            config,
            cookies_file,
            cookies_from_browser,
            no_cookies,
        } => {
            let mut config = VideoConfig::load(&config)?;
            if no_cookies || cookies_file.is_some() || cookies_from_browser.is_some() {
                config.cookies_file = cookies_file
                    .map(|path| path.canonicalize())
                    .transpose()
                    .context("open cookies file")?;
                config.cookies_from_browser = cookies_from_browser;
            }
            video(config, url).await
        }
        Command::Transcribe {
            input,
            model,
            whisper_bin,
            ffmpeg_bin,
            language,
            threads,
            timeout_secs,
            output,
        } => {
            let cancellation = Cancellation::default();
            let worker_token = cancellation.clone();
            let worker = tokio::task::spawn_blocking(move || -> Result<Transcript> {
                anyhow::ensure!(
                    model.is_file(),
                    "Whisper model does not exist: {}",
                    model.display()
                );
                let directory = tempfile::tempdir()?;
                let wav = directory.path().join("audio.wav");
                let timeout = Duration::from_secs(timeout_secs);
                Ffmpeg {
                    executable: ffmpeg_bin,
                    timeout,
                }
                .decode(&input, &wav, &worker_token)?;
                let mut transcript = WhisperCpp {
                    executable: whisper_bin,
                    model,
                    language,
                    threads,
                    timeout,
                }
                .transcribe(&wav, &worker_token)?;
                transcript.source = input.display().to_string();
                Ok(transcript)
            });
            let cancel_task = tokio::spawn(async move {
                if tokio::signal::ctrl_c().await.is_ok() {
                    cancellation.cancel();
                }
            });
            let result = worker.await.context("offline worker panicked");
            cancel_task.abort();
            write_transcript(&result??, output)
        }
    }
}

fn stream(config: RecognizerConfig) -> Result<()> {
    let mut pipeline = LivePipeline::new(config.create()?);
    let mut output = io::BufWriter::new(io::stdout().lock());
    write_event(
        &mut output,
        &ServerMessage::Ready {
            version: VERSION,
            backend: pipeline.info(),
        },
    )?;
    let mut reader = PcmReader::new(io::stdin().lock());
    let mut processing = Duration::ZERO;
    while let Some(chunk) = reader.next_chunk()? {
        let started = std::time::Instant::now();
        let updates = pipeline.push(&chunk)?;
        processing += started.elapsed();
        for caption in updates {
            write_event(&mut output, &ServerMessage::Caption { caption })?;
        }
    }
    let started = std::time::Instant::now();
    let updates = pipeline.finish()?;
    processing += started.elapsed();
    for caption in updates {
        write_event(&mut output, &ServerMessage::Caption { caption })?;
    }
    write_event(
        &mut output,
        &ServerMessage::Finished {
            samples_processed: pipeline.samples_processed(),
            processing_ms: processing.as_millis().try_into().unwrap_or(u64::MAX),
        },
    )
}

fn write_event(output: &mut impl Write, event: &ServerMessage) -> Result<()> {
    serde_json::to_writer(&mut *output, event)?;
    writeln!(output)?;
    output.flush()?;
    Ok(())
}

fn write_transcript(transcript: &Transcript, output: Output) -> Result<()> {
    let contents = match output.format {
        Format::Json => format!("{}\n", serde_json::to_string_pretty(transcript)?),
        Format::Srt => subtitle::render(
            transcript,
            subtitle::Format::Srt,
            NonZeroUsize::new(42).expect("positive width"),
        )?,
        Format::Vtt => subtitle::render(
            transcript,
            subtitle::Format::Vtt,
            NonZeroUsize::new(42).expect("positive width"),
        )?,
    };
    if let Some(path) = output.output {
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let mut file = tempfile::NamedTempFile::new_in(parent)?;
        file.write_all(contents.as_bytes())?;
        file.as_file().sync_all()?;
        file.persist_noclobber(&path)
            .with_context(|| format!("save {} without overwriting", path.display()))?;
    } else {
        io::stdout().lock().write_all(contents.as_bytes())?;
    }
    Ok(())
}

async fn video(config: VideoConfig, url: String) -> Result<()> {
    let cancellation = Cancellation::default();
    let worker_token = cancellation.clone();
    let worker = tokio::task::spawn_blocking(move || -> Result<()> {
        let mut job = VideoJob {
            id: new_job_id()?,
            stage: VideoStage::Queued,
            url: url.clone(),
            result: None,
            error: None,
        };
        let mut output = io::BufWriter::new(io::stdout().lock());
        let mut emit = |job: &VideoJob| -> Result<()> {
            serde_json::to_writer(&mut output, job)?;
            writeln!(output)?;
            output.flush()?;
            Ok(())
        };
        emit(&job)?;
        let result = run_video(&config, &url, &worker_token, |stage| {
            job.stage = stage;
            emit(&job)
        });
        match result {
            Ok(result) => {
                job.stage = VideoStage::Completed;
                job.result = Some(result);
            }
            Err(error) => {
                job.stage = if worker_token.is_cancelled() {
                    VideoStage::Cancelled
                } else {
                    VideoStage::Failed
                };
                job.error = Some(format!("{error:#}"));
                emit(&job)?;
                return Err(error);
            }
        }
        emit(&job)
    });
    let cancel_task = tokio::spawn(async move {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("SIGTERM handler");
        tokio::select! { _ = tokio::signal::ctrl_c() => {}, _ = terminate.recv() => {} }
        cancellation.cancel();
    });
    let result = worker.await.context("video worker panicked");
    cancel_task.abort();
    result?
}
