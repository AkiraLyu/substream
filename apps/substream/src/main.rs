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
    config::{Backend, RecognizerConfig},
    server::{ServerConfig, serve},
};
use substream_backends::{
    batch::{BatchRecognizer, Ffmpeg, WhisperCpp},
    pcm::PcmReader,
    process::Cancellation,
};
use substream_core::{
    audio::{AudioChunk, FRAME_SAMPLES},
    pipeline::LivePipeline,
    subtitle,
    transcript::Transcript,
};
use substream_protocol::{ServerMessage, VERSION};

#[derive(Parser)]
#[command(version, about = "Local speech recognition and subtitles")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run a deterministic synthetic pipeline. No model or speech recognition.
    Demo {
        #[command(flatten)]
        output: Output,
    },
    /// Read raw mono 16 kHz PCM16LE from stdin; emit NDJSON caption events.
    Stream {
        #[command(flatten)]
        recognizer: Recognizer,
    },
    /// Serve an authenticated binary WebSocket on loopback.
    Serve {
        #[arg(long, default_value = "127.0.0.1:9743")]
        listen: SocketAddr,
        #[arg(long)]
        token_file: PathBuf,
        /// Exact browser extension origin; repeat for more than one extension.
        #[arg(long = "allow-origin")]
        origins: Vec<String>,
        #[command(flatten)]
        recognizer: Recognizer,
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
    /// Explicit selection prevents synthetic output being mistaken for real ASR.
    #[arg(long, value_enum)]
    backend: Backend,
    #[arg(long)]
    config: Option<PathBuf>,
}

impl Recognizer {
    fn load(&self) -> Result<RecognizerConfig> {
        RecognizerConfig::load(self.backend, self.config.as_deref())
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
        Command::Demo { output } => write_transcript(&demo()?, output),
        Command::Stream { recognizer } => stream(recognizer.load()?),
        Command::Serve {
            listen,
            token_file,
            origins,
            recognizer,
        } => {
            serve(ServerConfig {
                address: listen,
                token: Token::read(&token_file)?,
                allowed_origins: origins,
                recognizer: recognizer.load()?,
            })
            .await
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

fn demo() -> Result<Transcript> {
    eprintln!("DEMO: synthetic captions; no speech recognition is performed.");
    let mut pipeline = LivePipeline::new(RecognizerConfig::Demo.create()?);
    let mut transcript = Transcript {
        schema_version: 1,
        source: "synthetic demo".into(),
        language: Some("zh".into()),
        synthetic: true,
        segments: vec![],
    };
    for sequence in 0..125 {
        let chunk = AudioChunk::new(
            sequence,
            u64::from(sequence) * FRAME_SAMPLES as u64,
            vec![0.0; FRAME_SAMPLES],
        )?;
        transcript.segments.extend(
            pipeline
                .push(&chunk)?
                .into_iter()
                .filter(|u| u.is_final)
                .map(|u| u.segment()),
        );
    }
    transcript
        .segments
        .extend(pipeline.finish()?.iter().map(|u| u.segment()));
    transcript.validate()?;
    Ok(transcript)
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
