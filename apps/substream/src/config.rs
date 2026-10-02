use std::{path::Path, sync::Arc};

use anyhow::{Context, Result};
use substream_backends::SherpaConfig;
use substream_core::asr::StreamingRecognizer;

/// Each session constructs its own recognizer on the inference worker.
pub type RecognizerFactory = Arc<dyn Fn() -> Result<Box<dyn StreamingRecognizer>> + Send + Sync>;

#[derive(Debug, Clone)]
pub struct RecognizerConfig(SherpaConfig);

impl RecognizerConfig {
    pub fn load(path: &Path, threads: Option<i32>) -> Result<Self> {
        let path = path.canonicalize().context("open model configuration")?;
        let mut config: SherpaConfig = toml::from_str(&std::fs::read_to_string(&path)?)
            .context("parse sherpa model configuration")?;
        let base = path.parent().context("configuration has no parent")?;
        for model in [
            &mut config.encoder,
            &mut config.decoder,
            &mut config.joiner,
            &mut config.tokens,
        ] {
            if model.is_relative() {
                *model = base.join(&*model);
            }
        }
        if let Some(threads) = threads {
            config.threads = threads;
        }
        config.validate()?;
        Ok(Self(config))
    }

    pub fn factory(self) -> RecognizerFactory {
        Arc::new(move || self.create())
    }

    /// Construct on the inference worker; no FFI object crosses the reactor.
    pub fn create(&self) -> Result<Box<dyn StreamingRecognizer>> {
        self.0.validate()?;
        #[cfg(feature = "sherpa")]
        {
            Ok(Box::new(substream_backends::sherpa::SherpaRecognizer::new(
                &self.0,
            )?))
        }
        #[cfg(not(feature = "sherpa"))]
        anyhow::bail!("rebuild substream with --features sherpa to enable speech recognition")
    }
}
