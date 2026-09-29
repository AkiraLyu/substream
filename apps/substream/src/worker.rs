use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use substream_core::{audio::AudioChunk, pipeline::LivePipeline};
use substream_protocol::{ServerMessage, VERSION};
use tokio::sync::{OwnedSemaphorePermit, mpsc};

use crate::config::RecognizerConfig;

pub const AUDIO_QUEUE_CAPACITY: usize = 8;
pub const EVENT_QUEUE_CAPACITY: usize = 32;
const MAX_QUEUE_AGE: Duration = Duration::from_millis(250);

pub struct QueuedAudio {
    pub chunk: AudioChunk,
    pub received: Instant,
}

pub fn run(
    config: RecognizerConfig,
    mut input: mpsc::Receiver<QueuedAudio>,
    output: mpsc::Sender<ServerMessage>,
    finish: Arc<AtomicBool>,
    _permit: OwnedSemaphorePermit,
) {
    let result = (|| -> anyhow::Result<()> {
        if output.is_closed() {
            return Ok(());
        }
        let mut pipeline = LivePipeline::new(config.create()?);
        output.blocking_send(ServerMessage::Ready {
            version: VERSION,
            backend: pipeline.info(),
        })?;
        let mut processing = Duration::ZERO;
        while let Some(audio) = input.blocking_recv() {
            if output.is_closed() {
                return Ok(());
            }
            anyhow::ensure!(
                audio.received.elapsed() <= MAX_QUEUE_AGE,
                "audio queue exceeded 250 ms; reconnect to restart the timeline"
            );
            let started = Instant::now();
            let updates = pipeline.push(&audio.chunk)?;
            processing += started.elapsed();
            for caption in updates {
                output.blocking_send(ServerMessage::Caption { caption })?;
            }
        }
        // Disconnect is cancellation. Only explicit EOS is a successful finish.
        if finish.load(Ordering::Acquire) && !output.is_closed() {
            let started = Instant::now();
            let updates = pipeline.finish()?;
            processing += started.elapsed();
            for caption in updates {
                output.blocking_send(ServerMessage::Caption { caption })?;
            }
            output.blocking_send(ServerMessage::Finished {
                samples_processed: pipeline.samples_processed(),
                processing_ms: processing.as_millis().try_into().unwrap_or(u64::MAX),
            })?;
        }
        Ok(())
    })();
    if let Err(error) = result {
        let _ = output.blocking_send(ServerMessage::Error {
            code: "worker_failed".into(),
            message: error.to_string(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::Semaphore;

    #[test]
    fn rejects_stale_audio_and_releases_the_inference_permit() {
        let (input_tx, input_rx) = mpsc::channel(AUDIO_QUEUE_CAPACITY);
        let (output_tx, mut output_rx) = mpsc::channel(EVENT_QUEUE_CAPACITY);
        input_tx
            .try_send(QueuedAudio {
                chunk: AudioChunk::new(0, 0, vec![0.0; 320]).unwrap(),
                received: Instant::now() - Duration::from_secs(1),
            })
            .unwrap();
        drop(input_tx);
        let semaphore = Arc::new(Semaphore::new(1));
        let permit = Arc::clone(&semaphore).try_acquire_owned().unwrap();
        run(
            RecognizerConfig::Demo,
            input_rx,
            output_tx,
            Arc::new(AtomicBool::new(true)),
            permit,
        );
        assert!(matches!(
            output_rx.blocking_recv(),
            Some(ServerMessage::Ready { .. })
        ));
        assert!(
            matches!(output_rx.blocking_recv(), Some(ServerMessage::Error { code, .. }) if code == "worker_failed")
        );
        assert!(output_rx.blocking_recv().is_none());
        assert_eq!(semaphore.available_permits(), 1);
    }

    #[test]
    fn disconnect_does_not_pretend_to_finalize_a_transcript() {
        let (input_tx, input_rx) = mpsc::channel(AUDIO_QUEUE_CAPACITY);
        let (output_tx, mut output_rx) = mpsc::channel(EVENT_QUEUE_CAPACITY);
        drop(input_tx);
        let permit = Arc::new(Semaphore::new(1)).try_acquire_owned().unwrap();
        run(
            RecognizerConfig::Demo,
            input_rx,
            output_tx,
            Arc::new(AtomicBool::new(false)),
            permit,
        );
        assert!(matches!(
            output_rx.blocking_recv(),
            Some(ServerMessage::Ready { .. })
        ));
        assert!(output_rx.blocking_recv().is_none());
    }
}
