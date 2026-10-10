//! Text-only Chat Completions client for configurable LLM providers.

use std::time::Duration;

use anyhow::{Context, Result, bail, ensure};
use reqwest::{Client, Url, redirect::Policy};
use serde::Deserialize;
use serde_json::{Map, Value, json};

pub struct ChatClient {
    client: Client,
    endpoint: Url,
    api_key: String,
    model: String,
    parameters: Map<String, Value>,
}

impl ChatClient {
    pub fn new(
        endpoint: &str,
        api_key: String,
        model: &str,
        parameters: Map<String, Value>,
        timeout: Duration,
    ) -> Result<Self> {
        let endpoint = Url::parse(endpoint).context("invalid LLM endpoint")?;
        let local = endpoint.host_str().is_some_and(|host| {
            host == "localhost"
                || host
                    .trim_matches(['[', ']'])
                    .parse::<std::net::IpAddr>()
                    .is_ok_and(|ip| ip.is_loopback())
        });
        ensure!(
            endpoint.scheme() == "https" || (endpoint.scheme() == "http" && local),
            "LLM endpoint must use HTTPS, or HTTP on loopback"
        );
        ensure!(
            endpoint.has_host()
                && endpoint.username().is_empty()
                && endpoint.password().is_none()
                && endpoint.query().is_none()
                && endpoint.fragment().is_none(),
            "LLM endpoint must not contain credentials, a query, or a fragment"
        );
        ensure!(!model.trim().is_empty(), "choose an LLM model");
        for key in [
            "model",
            "messages",
            "stream",
            "stream_options",
            "n",
            "tools",
            "tool_choice",
            "functions",
            "function_call",
        ] {
            ensure!(
                !parameters.contains_key(key),
                "request parameter {key} is managed by Substream or not supported"
            );
        }
        let client = Client::builder()
            .timeout(timeout)
            .connect_timeout(Duration::from_secs(15))
            .redirect(Policy::none())
            .build()
            .context("create LLM HTTP client")?;
        Ok(Self {
            client,
            endpoint,
            api_key,
            model: model.to_owned(),
            parameters,
        })
    }

    pub async fn complete(&self, system_prompt: &str, user_prompt: &str) -> Result<String> {
        let mut body = self.parameters.clone();
        body.insert("model".into(), json!(self.model));
        body.insert("stream".into(), json!(false));
        body.insert(
            "messages".into(),
            json!([
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_prompt}
            ]),
        );
        let mut request = self.client.post(self.endpoint.clone()).json(&body);
        if !self.api_key.is_empty() {
            request = request.bearer_auth(&self.api_key);
        }
        let mut response = request
            .send()
            .await
            .map_err(|error| error.without_url())
            .context("LLM request failed")?;
        let status = response.status();
        let mut bytes = Vec::new();
        const MAX_RESPONSE: usize = 2 * 1024 * 1024;
        while let Some(chunk) = response.chunk().await.context("read LLM response")? {
            ensure!(
                bytes.len() + chunk.len() <= MAX_RESPONSE,
                "LLM response exceeds 2 MiB"
            );
            bytes.extend_from_slice(&chunk);
        }
        if !status.is_success() {
            let detail = serde_json::from_slice::<Value>(&bytes)
                .ok()
                .and_then(|body| {
                    body.get("error")?
                        .get("message")?
                        .as_str()
                        .map(str::to_owned)
                })
                .unwrap_or_else(|| {
                    "check the endpoint, credentials, model, and request parameters".into()
                });
            let detail = if self.api_key.is_empty() {
                detail
            } else {
                detail.replace(&self.api_key, "[redacted]")
            };
            bail!(
                "LLM HTTP {}: {}",
                status.as_u16(),
                detail.chars().take(1000).collect::<String>()
            );
        }
        let output: ChatResponse =
            serde_json::from_slice(&bytes).context("invalid Chat Completions response")?;
        let choice = output
            .choices
            .into_iter()
            .next()
            .context("LLM returned no choices")?;
        match choice.finish_reason.as_str() {
            "stop" => {}
            "length" => bail!(
                "LLM output was truncated; increase the output token limit or shorten the prompt"
            ),
            reason => {
                bail!("LLM did not return a complete text response (finish_reason: {reason})")
            }
        }
        ensure!(
            choice
                .message
                .refusal
                .as_deref()
                .is_none_or(|value| value.is_empty()),
            "LLM refused the summary request"
        );
        let text = choice.message.content.context("LLM returned no text")?;
        ensure!(!text.trim().is_empty(), "LLM returned empty text");
        Ok(text)
    }
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}
#[derive(Deserialize)]
struct Choice {
    finish_reason: String,
    message: Message,
}
#[derive(Deserialize)]
struct Message {
    content: Option<String>,
    refusal: Option<String>,
}
