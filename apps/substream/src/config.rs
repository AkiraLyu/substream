use std::path::Path;

use anyhow::{Context, Result, bail};
use clap::ValueEnum;
use substream_backends::{SherpaConfig, demo::DemoRecognizer};
use substream_core::asr::StreamingRecognizer;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Backend {
    Demo,
    Sherpa,
}

#[derive(Debug, Clone)]
pub enum RecognizerConfig {
    Demo,
    Sherpa(SherpaConfig),
}

impl RecognizerConfig {
    pub fn load(backend: Backend, config: Option<&Path>) -> Result<Self> {
        match backend {
            Backend::Demo => {
                anyhow::ensure!(config.is_none(), "--config only applies to sherpa");
                Ok(Self::Demo)
            }
            Backend::Sherpa => {
                if !cfg!(feature = "sherpa") {
                    bail!("rebuild with --features sherpa to use real streaming ASR");
                }
                let path = config
                    .context("sherpa requires --config <model.toml>")?
                    .canonicalize()?;
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
                config.validate()?;
                Ok(Self::Sherpa(config))
            }
        }
    }

    /// Construct on the inference worker; no FFI object crosses the reactor.
    pub fn create(&self) -> Result<Box<dyn StreamingRecognizer>> {
        match self {
            Self::Demo => Ok(Box::<DemoRecognizer>::default()),
            #[cfg(feature = "sherpa")]
            Self::Sherpa(config) => Ok(Box::new(
                substream_backends::sherpa::SherpaRecognizer::new(config)?,
            )),
            #[cfg(not(feature = "sherpa"))]
            Self::Sherpa(_) => bail!("sherpa feature is not enabled"),
        }
    }
}
