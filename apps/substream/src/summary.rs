//! Summary orchestration shared by desktop, command-line, and HTTP callers.

use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use serde_json::{Map, Value};
use std::{fs::File, io::Read, path::Path, time::Duration};
use substream_backends::llm::ChatClient;
use substream_protocol::{
    summary::{SummaryRequest, SummaryResult},
    video::VideoDocument,
};

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SummaryConfig {
    pub endpoint: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub api_key_env: String,
    pub model: String,
    pub timeout_secs: u64,
    pub max_input_chars: usize,
    #[serde(default)]
    pub parameters: Map<String, Value>,
    pub system_prompt: String,
    pub user_prompt: String,
}

impl SummaryConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let mut bytes = Vec::new();
        File::open(path)
            .context("open summary configuration")?
            .take(128 * 1024 + 1)
            .read_to_end(&mut bytes)?;
        ensure!(
            bytes.len() <= 128 * 1024,
            "summary configuration exceeds 128 KiB"
        );
        serde_json::from_slice(&bytes).context("invalid summary configuration")
    }

    fn client(&self) -> Result<ChatClient> {
        ensure!(
            (1..=3600).contains(&self.timeout_secs),
            "summary timeout must be 1..=3600 seconds"
        );
        ensure!(
            (1..=2_000_000).contains(&self.max_input_chars),
            "summary input limit must be 1..=2000000 characters"
        );
        let key = if self.api_key.is_empty() && !self.api_key_env.is_empty() {
            std::env::var(&self.api_key_env).unwrap_or_default()
        } else {
            self.api_key.clone()
        };
        ChatClient::new(
            &self.endpoint,
            key,
            &self.model,
            self.parameters.clone(),
            Duration::from_secs(self.timeout_secs),
        )
    }
}

pub async fn summarize(
    config: &SummaryConfig,
    document: &VideoDocument,
    request: &SummaryRequest,
) -> Result<SummaryResult> {
    let client = config.client()?;
    ensure!(
        document.schema_version == 1,
        "unsupported video document version"
    );
    document.transcript.validate()?;
    ensure!(
        !document.transcript.segments.is_empty(),
        "the subtitle document has no text to summarize"
    );
    let system = request
        .system_prompt
        .as_deref()
        .unwrap_or(&config.system_prompt);
    let template = request
        .user_prompt
        .as_deref()
        .unwrap_or(&config.user_prompt);
    ensure!(
        system.len() <= 32 * 1024 && template.len() <= 32 * 1024,
        "summary prompts exceed 32 KiB"
    );
    ensure!(
        template.contains("{{transcript}}"),
        "user prompt must contain {}",
        "{{transcript}}"
    );
    let mut transcript = String::new();
    for segment in &document.transcript.segments {
        use std::fmt::Write;
        writeln!(
            &mut transcript,
            "[{} --> {}] {}",
            timestamp(segment.start_ms),
            timestamp(segment.end_ms),
            segment.text
        )?;
    }
    let prompt = expand_prompt(
        template,
        &document.source.title,
        &document.source.url,
        &transcript,
        config.max_input_chars * 4,
    )?;
    ensure!(
        system.chars().count() + prompt.chars().count() <= config.max_input_chars,
        "summary input exceeds max_input_chars ({}); no text was sent",
        config.max_input_chars
    );
    let markdown = client.complete(system, &prompt).await?;
    Ok(SummaryResult {
        schema_version: 1,
        source: document.source.clone(),
        model: config.model.clone(),
        markdown,
    })
}

fn timestamp(ms: u64) -> String {
    format!(
        "{:02}:{:02}:{:02}.{:03}",
        ms / 3_600_000,
        ms / 60_000 % 60,
        ms / 1000 % 60,
        ms % 1000
    )
}

/// Substitute once so subtitle text that resembles a placeholder stays literal.
fn expand_prompt(
    template: &str,
    title: &str,
    url: &str,
    transcript: &str,
    max_bytes: usize,
) -> Result<String> {
    let mut result = String::new();
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        result.push_str(&rest[..start]);
        let end = rest[start + 2..]
            .find("}}")
            .context("unclosed prompt placeholder")?
            + start
            + 2;
        let value = match &rest[start + 2..end] {
            "title" => title,
            "url" => url,
            "transcript" => transcript,
            name => anyhow::bail!("unknown prompt placeholder: {name}"),
        };
        ensure!(
            result.len() + value.len() <= max_bytes,
            "summary input exceeds max_input_chars; no text was sent"
        );
        result.push_str(value);
        rest = &rest[end + 2..];
    }
    result.push_str(rest);
    Ok(result)
}
