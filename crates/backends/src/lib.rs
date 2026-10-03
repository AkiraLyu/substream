//! Device and inference adapters. Native dependencies are opt-in.

pub mod batch;
pub mod download;
pub mod pcm;
pub mod process;
#[cfg(feature = "sherpa")]
pub mod sherpa;

use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SherpaConfig {
    pub encoder: PathBuf,
    pub decoder: PathBuf,
    pub joiner: PathBuf,
    pub tokens: PathBuf,
    /// A declaration of this model's languages, not automatic language detection.
    pub languages: Vec<String>,
    #[serde(default = "default_threads")]
    pub threads: i32,
}

fn default_threads() -> i32 {
    2
}

impl SherpaConfig {
    pub fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            (1..=64).contains(&self.threads),
            "sherpa threads must be 1..=64"
        );
        anyhow::ensure!(
            !self.languages.is_empty(),
            "declare this model's supported languages"
        );
        for path in [&self.encoder, &self.decoder, &self.joiner, &self.tokens] {
            anyhow::ensure!(
                path.is_file(),
                "model file does not exist: {}",
                path.display()
            );
            anyhow::ensure!(path.to_str().is_some(), "sherpa model paths must be UTF-8");
        }
        Ok(())
    }
}
