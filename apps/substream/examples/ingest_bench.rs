//! A reproducible baseline for framework overhead; it deliberately excludes ASR.

use std::{hint::black_box, time::Instant};

use substream::config::RecognizerConfig;
use substream_core::{
    audio::{AudioChunk, FRAME_SAMPLES},
    pipeline::LivePipeline,
};
use substream_protocol::{decode_audio, encode_audio};

fn main() -> anyhow::Result<()> {
    let mut pipeline = LivePipeline::new(RecognizerConfig::Demo.create()?);
    let mut times = Vec::with_capacity(50_000);
    for sequence in 0..51_000 {
        let chunk = AudioChunk::new(
            sequence,
            u64::from(sequence) * FRAME_SAMPLES as u64,
            vec![0.0; FRAME_SAMPLES],
        )?;
        let bytes = encode_audio(&chunk);
        let start = Instant::now();
        let decoded = decode_audio(black_box(&bytes))?;
        black_box(pipeline.push(&decoded)?);
        if sequence >= 1_000 {
            times.push(start.elapsed().as_nanos());
        }
    }
    times.sort_unstable();
    println!(
        "Synthetic baseline only: decode + demo pipeline, 20 ms frames, 1,000 warmup + 50,000 samples"
    );
    for (label, percentile) in [("p50", 50), ("p95", 95), ("p99", 99)] {
        println!(
            "{label}: {} ns/frame",
            times[(times.len() - 1) * percentile / 100]
        );
    }
    println!("Excludes capture, browser, queues, native ASR, model loading, and rendering.");
    Ok(())
}
