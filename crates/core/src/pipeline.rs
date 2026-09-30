use std::num::NonZeroUsize;

use serde::{Deserialize, Serialize};

use crate::{
    Error, Result,
    asr::{BackendInfo, Hypothesis, StreamingRecognizer},
    audio::{AudioChunk, samples_to_ms},
    stabilizer::Stabilizer,
    transcript::Segment,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptionUpdate {
    pub segment_id: u64,
    pub revision: u64,
    pub start_ms: u64,
    pub end_ms: u64,
    pub stable_text: String,
    pub unstable_text: String,
    pub is_final: bool,
}

impl CaptionUpdate {
    pub fn segment(&self) -> Segment {
        Segment {
            id: self.segment_id,
            start_ms: self.start_ms,
            end_ms: self.end_ms,
            text: format!("{}{}", self.stable_text, self.unstable_text),
        }
    }
}

/// Bounded session state: finalized history belongs to a transcript sink.
pub struct LivePipeline {
    recognizer: Box<dyn StreamingRecognizer>,
    stabilizer: Stabilizer,
    next_sequence: u32,
    next_sample: u64,
    active: Option<Segment>,
    last_final: Option<(u64, u64)>,
    revision: u64,
    finished: bool,
}

impl LivePipeline {
    pub fn new(recognizer: Box<dyn StreamingRecognizer>) -> Self {
        Self {
            recognizer,
            stabilizer: Stabilizer::new(NonZeroUsize::new(2).expect("two is nonzero")),
            next_sequence: 0,
            next_sample: 0,
            active: None,
            last_final: None,
            revision: 0,
            finished: false,
        }
    }

    pub fn info(&self) -> BackendInfo {
        self.recognizer.info()
    }
    pub fn samples_processed(&self) -> u64 {
        self.next_sample
    }

    pub fn push(&mut self, chunk: &AudioChunk) -> Result<Vec<CaptionUpdate>> {
        if self.finished {
            return Err(Error::State("audio after EOS".into()));
        }
        if chunk.sequence() != self.next_sequence || chunk.start_sample() != self.next_sample {
            return Err(Error::State(
                "audio discontinuity; start a new session".into(),
            ));
        }
        self.next_sequence = self.next_sequence.wrapping_add(1);
        self.next_sample = chunk.end_sample();
        let hypotheses = self.recognizer.push(chunk)?;
        self.apply(hypotheses)
    }

    pub fn finish(&mut self) -> Result<Vec<CaptionUpdate>> {
        if self.finished {
            return Err(Error::State("duplicate EOS".into()));
        }
        self.finished = true;
        let hypotheses = self.recognizer.finish()?;
        let updates = self.apply(hypotheses)?;
        if self.active.is_some() {
            return Err(Error::Backend(
                "backend did not finalize pending speech at EOS".into(),
            ));
        }
        Ok(updates)
    }

    fn apply(&mut self, hypotheses: Vec<Hypothesis>) -> Result<Vec<CaptionUpdate>> {
        let mut updates = Vec::with_capacity(hypotheses.len());
        for hypothesis in hypotheses {
            let segment = hypothesis.segment;
            segment.validate()?;
            if segment.end_ms > samples_to_ms(self.next_sample) {
                return Err(Error::Backend(
                    "hypothesis extends beyond received audio".into(),
                ));
            }
            if let Some((id, end_ms)) = self.last_final
                && (segment.id <= id || segment.start_ms < end_ms)
            {
                return Err(Error::Backend("backend revised finalized speech".into()));
            }
            if let Some(active) = &self.active {
                if segment.id != active.id
                    || segment.start_ms != active.start_ms
                    || segment.end_ms < active.end_ms
                {
                    return Err(Error::Backend(
                        "backend changed the active segment timeline".into(),
                    ));
                }
            } else {
                self.revision = 0;
                self.stabilizer.reset();
            }
            self.revision += 1;
            let (stable_text, unstable_text) =
                self.stabilizer.update(&segment.text, hypothesis.is_final);
            updates.push(CaptionUpdate {
                segment_id: segment.id,
                revision: self.revision,
                start_ms: segment.start_ms,
                end_ms: segment.end_ms,
                stable_text,
                unstable_text,
                is_final: hypothesis.is_final,
            });
            if hypothesis.is_final {
                self.last_final = Some((segment.id, segment.end_ms));
                self.active = None;
            } else {
                self.active = Some(segment);
            }
        }
        Ok(updates)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    struct Script(VecDeque<Vec<Hypothesis>>);

    impl StreamingRecognizer for Script {
        fn info(&self) -> BackendInfo {
            BackendInfo {
                name: "test".into(),
                synthetic: true,
                languages: vec![],
            }
        }
        fn push(&mut self, _: &AudioChunk) -> Result<Vec<Hypothesis>> {
            Ok(self.0.pop_front().unwrap_or_default())
        }
        fn finish(&mut self) -> Result<Vec<Hypothesis>> {
            Ok(self.0.pop_front().unwrap_or_default())
        }
    }

    fn hypothesis(id: u64, end_ms: u64, text: &str, is_final: bool) -> Hypothesis {
        Hypothesis {
            segment: Segment {
                id,
                start_ms: 0,
                end_ms,
                text: text.into(),
            },
            is_final,
        }
    }

    #[test]
    fn final_corrections_replace_partial_text_without_duplicating_segments() {
        let draft_text = "今天用 Ruby 编写字幕 👩‍💻。";
        let final_text = "今天用 Rust 编写字幕 👩‍💻。";
        let script = Script(VecDeque::from([
            vec![hypothesis(0, 20, draft_text, false)],
            vec![hypothesis(0, 40, draft_text, false)],
            vec![hypothesis(0, 40, final_text, true)],
        ]));
        let mut pipeline = LivePipeline::new(Box::new(script));
        let first = pipeline
            .push(&AudioChunk::new(0, 0, vec![0.0; 320]).unwrap())
            .unwrap();
        let mut updates = pipeline
            .push(&AudioChunk::new(1, 320, vec![0.0; 320]).unwrap())
            .unwrap();
        updates.extend(pipeline.finish().unwrap());
        let partial = first.iter().find(|update| !update.is_final).unwrap();
        let finals: Vec<_> = updates.iter().filter(|update| update.is_final).collect();
        assert_eq!(finals.len(), 1, "one finalized subtitle per utterance");
        let final_result = finals[0];
        assert_eq!(final_result.segment_id, partial.segment_id);
        assert!(final_result.revision > partial.revision);
        assert_eq!(final_result.segment().text, final_text);
        assert_eq!((final_result.start_ms, final_result.end_ms), (0, 40));
    }
}
