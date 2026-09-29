use sherpa_onnx::{OnlineRecognizer, OnlineRecognizerConfig, OnlineStream};
use substream_core::{
    Error, Result,
    asr::{BackendInfo, Hypothesis, StreamingRecognizer},
    audio::{AudioChunk, SAMPLE_RATE, samples_to_ms},
    transcript::Segment,
};

use crate::SherpaConfig;

pub struct SherpaRecognizer {
    // Rust drops fields in declaration order. The stream must die before its recognizer.
    stream: OnlineStream,
    recognizer: OnlineRecognizer,
    languages: Vec<String>,
    received: u64,
    segment_start: u64,
    segment_id: u64,
    last_text: String,
}

impl SherpaRecognizer {
    pub fn new(config: &SherpaConfig) -> anyhow::Result<Self> {
        config.validate()?;
        let mut native = OnlineRecognizerConfig::default();
        native.model_config.transducer.encoder = config.encoder.to_str().map(str::to_owned);
        native.model_config.transducer.decoder = config.decoder.to_str().map(str::to_owned);
        native.model_config.transducer.joiner = config.joiner.to_str().map(str::to_owned);
        native.model_config.tokens = config.tokens.to_str().map(str::to_owned);
        native.model_config.num_threads = config.threads;
        // The opt-in prebuilt library is CPU-only. GPU providers need separate builds.
        native.model_config.provider = Some("cpu".into());
        native.enable_endpoint = true;
        native.decoding_method = Some("greedy_search".into());
        native.rule1_min_trailing_silence = 2.4;
        native.rule2_min_trailing_silence = 0.8;
        native.rule3_min_utterance_length = 15.0;
        let recognizer = OnlineRecognizer::create(&native)
            .ok_or_else(|| anyhow::anyhow!("sherpa-onnx could not load this model"))?;
        let stream = recognizer.create_stream();
        Ok(Self {
            stream,
            recognizer,
            languages: config.languages.clone(),
            received: 0,
            segment_start: 0,
            segment_id: 0,
            last_text: String::new(),
        })
    }

    fn hypothesis(&mut self, final_result: bool) -> Result<Option<Hypothesis>> {
        let result = self
            .recognizer
            .get_result(&self.stream)
            .ok_or_else(|| Error::Backend("sherpa returned an invalid result".into()))?;
        let text = result.text.trim();
        if !text.is_empty() {
            self.last_text = text.to_owned();
        }
        if self.last_text.is_empty() || self.received <= self.segment_start {
            return Ok(None);
        }
        Ok(Some(Hypothesis {
            segment: Segment {
                id: self.segment_id,
                start_ms: samples_to_ms(self.segment_start),
                end_ms: samples_to_ms(self.received),
                text: self.last_text.clone(),
            },
            is_final: final_result,
        }))
    }

    fn reset_segment(&mut self) {
        self.recognizer.reset(&self.stream);
        self.segment_id += 1;
        self.segment_start = self.received;
        self.last_text.clear();
    }
}

impl StreamingRecognizer for SherpaRecognizer {
    fn info(&self) -> BackendInfo {
        BackendInfo {
            name: "sherpa-onnx / streaming transducer / CPU".into(),
            synthetic: false,
            languages: self.languages.clone(),
        }
    }

    fn push(&mut self, chunk: &AudioChunk) -> Result<Vec<Hypothesis>> {
        self.received = chunk.end_sample();
        self.stream
            .accept_waveform(SAMPLE_RATE as i32, chunk.samples());
        let mut decoded = false;
        while self.recognizer.is_ready(&self.stream) {
            self.recognizer.decode(&self.stream);
            decoded = true;
        }
        if !decoded {
            return Ok(vec![]);
        }
        let endpoint = self.recognizer.is_endpoint(&self.stream);
        let result = self.hypothesis(endpoint)?.into_iter().collect();
        if endpoint {
            self.reset_segment();
        }
        Ok(result)
    }

    fn finish(&mut self) -> Result<Vec<Hypothesis>> {
        if self.received == 0 {
            return Ok(vec![]);
        }
        // Acoustic lookahead only: padding never advances the public sample clock.
        self.stream
            .accept_waveform(SAMPLE_RATE as i32, &[0.0; 4_800]);
        self.stream.input_finished();
        while self.recognizer.is_ready(&self.stream) {
            self.recognizer.decode(&self.stream);
        }
        Ok(self.hypothesis(true)?.into_iter().collect())
    }
}
