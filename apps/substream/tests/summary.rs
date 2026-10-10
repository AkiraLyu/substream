use std::sync::{Arc, Mutex};

use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::post,
};
use serde_json::{Value, json};
use substream::summary::{SummaryConfig, summarize};
use substream_protocol::{summary::SummaryRequest, video::VideoDocument};

type Requests = Arc<Mutex<Vec<Value>>>;

async fn provider(
    State(requests): State<Requests>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> axum::response::Response {
    assert_eq!(headers["authorization"], "Bearer fixture-secret");
    assert!(!headers.contains_key("cookie"));
    requests.lock().unwrap().push(body.clone());
    if body["model"] == "denied" {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error":{"message":"invalid key fixture-secret"}})),
        )
            .into_response();
    }
    if body["model"] == "slow" {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
    Json(json!({
        "choices": [{
            "finish_reason": if body["model"] == "truncated" { "length" } else { "stop" },
            "message": {"content": "# Summary\n\nA complete summary [00:00:01]."}
        }]
    }))
    .into_response()
}

#[tokio::test]
async fn summaries_use_configured_models_and_prompts_without_accepting_incomplete_results() {
    let requests = Requests::default();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!(
        "http://{}/v1/chat/completions",
        listener.local_addr().unwrap()
    );
    let app = Router::new()
        .route("/v1/chat/completions", post(provider))
        .with_state(requests.clone());
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let mut config: Value =
        serde_json::from_str(include_str!("../../../configs/summary.example.json")).unwrap();
    config["endpoint"] = json!(endpoint);
    config["api_key"] = json!("fixture-secret");
    config["model"] = json!("fixture-model");
    config["parameters"] = json!({"temperature": 0.4, "max_completion_tokens": 1500});
    let mut config: SummaryConfig = serde_json::from_value(config).unwrap();
    let document: VideoDocument = serde_json::from_value(json!({
        "schema_version": 1,
        "source": {"url": "https://example.com/video", "title": "A {{url}} title", "id": "clip"},
        "subtitle_source": "provided",
        "transcript": {
            "schema_version": 1, "source": "fixture", "language": "en",
            "segments": [{"id": 0, "start_ms": 1000, "end_ms": 2000, "text": "Literal {{title}} caption"}]
        }
    }))
    .unwrap();
    let prompt = SummaryRequest {
        system_prompt: Some("Write concise notes.".into()),
        user_prompt: Some("Title: {{title}}\n{{url}}\n{{transcript}}".into()),
    };
    let result = summarize(&config, &document, &prompt).await.unwrap();
    assert!(result.markdown.contains("# Summary"));
    assert_eq!(result.source.id, "clip");
    let sent = requests.lock().unwrap()[0].clone();
    assert_eq!(sent["model"], "fixture-model");
    assert_eq!(sent["temperature"], 0.4);
    assert_eq!(sent["max_completion_tokens"], 1500);
    assert_eq!(sent["messages"][0]["content"], "Write concise notes.");
    let content = sent["messages"][1]["content"].as_str().unwrap();
    assert!(content.contains("Title: A {{url}} title"));
    assert!(content.contains("[00:00:01.000 --> 00:00:02.000] Literal {{title}} caption"));

    config.max_input_chars = 1;
    assert!(
        summarize(&config, &document, &prompt)
            .await
            .unwrap_err()
            .to_string()
            .contains("no text was sent")
    );
    assert_eq!(
        requests.lock().unwrap().len(),
        1,
        "oversized input must not reach the provider"
    );
    config.max_input_chars = 200000;
    for model in ["truncated", "denied", "slow"] {
        config.model = model.into();
        config.timeout_secs = 1;
        let error = summarize(&config, &document, &prompt).await.unwrap_err();
        assert!(!format!("{error:#}").contains("fixture-secret"));
        if model == "truncated" {
            assert!(error.to_string().contains("truncated"));
        }
    }
    server.abort();
}
