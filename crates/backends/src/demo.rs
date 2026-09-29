use substream_core::{
    Result,
    asr::{BackendInfo, Hypothesis, StreamingRecognizer},
    audio::{AudioChunk, samples_to_ms},
    transcript::Segment,
};

const STEPS: [&str; 5] = [
    "这是",
    "这是 Substream",
    "这是 Substream 的",
    "这是 Substream 的演示",
    "这是 Substream 的演示字幕。",
];
const STEP_SAMPLES: u64 = 3_200;
const SEGMENT_SAMPLES: u64 = STEP_SAMPLES * STEPS.len() as u64;

/// Deterministic synthetic captions. This backend does not recognize speech.
#[derive(Default)]
pub struct DemoRecognizer {
    received: u64,
    emitted_steps: u64,
}

impl StreamingRecognizer for DemoRecognizer {
    fn info(&self) -> BackendInfo {
        BackendInfo {
            name: "demo (synthetic, no speech recognition)".into(),
            synthetic: true,
            languages: vec!["zh".into(), "en".into()],
        }
    }

    fn push(&mut self, chunk: &AudioChunk) -> Result<Vec<Hypothesis>> {
        self.received = chunk.end_sample();
        let mut output = Vec::new();
        while (self.emitted_steps + 1) * STEP_SAMPLES <= self.received {
            let step = (self.emitted_steps % STEPS.len() as u64) as usize;
            let id = self.emitted_steps / STEPS.len() as u64;
            output.push(Hypothesis {
                segment: Segment {
                    id,
                    start_ms: samples_to_ms(id * SEGMENT_SAMPLES),
                    end_ms: samples_to_ms((self.emitted_steps + 1) * STEP_SAMPLES),
                    text: STEPS[step].into(),
                },
                is_final: step == STEPS.len() - 1,
            });
            self.emitted_steps += 1;
        }
        Ok(output)
    }

    fn finish(&mut self) -> Result<Vec<Hypothesis>> {
        let id = self.received / SEGMENT_SAMPLES;
        let start = id * SEGMENT_SAMPLES;
        if samples_to_ms(self.received) == samples_to_ms(start) {
            return Ok(vec![]);
        }
        let step = ((self.received - start) / STEP_SAMPLES).saturating_sub(1) as usize;
        Ok(vec![Hypothesis {
            segment: Segment {
                id,
                start_ms: samples_to_ms(start),
                end_ms: samples_to_ms(self.received),
                text: STEPS[step].into(),
            },
            is_final: true,
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use substream_core::{audio::AudioChunk, pipeline::LivePipeline};

    #[test]
    fn pipeline_flushes_tail_and_rejects_discontinuities_and_duplicate_eos() {
        let mut pipeline = LivePipeline::new(Box::<DemoRecognizer>::default());
        let mut finals = Vec::new();
        for i in 0..53 {
            for update in pipeline
                .push(&AudioChunk::new(i, u64::from(i) * 320, vec![0.0; 320]).unwrap())
                .unwrap()
            {
                if update.is_final {
                    finals.push(update.segment());
                }
            }
        }
        finals.extend(pipeline.finish().unwrap().iter().map(|u| u.segment()));
        assert_eq!(finals.len(), 2);
        assert_eq!((finals[1].start_ms, finals[1].end_ms), (1000, 1060));
        assert!(pipeline.finish().is_err());
        assert!(
            pipeline
                .push(&AudioChunk::new(53, 16960, vec![0.0]).unwrap())
                .is_err()
        );

        let mut pipeline = LivePipeline::new(Box::<DemoRecognizer>::default());
        assert!(
            pipeline
                .push(&AudioChunk::new(1, 0, vec![0.0]).unwrap())
                .is_err()
        );
        assert!(
            pipeline
                .push(&AudioChunk::new(0, 1, vec![0.0]).unwrap())
                .is_err()
        );
    }
}
