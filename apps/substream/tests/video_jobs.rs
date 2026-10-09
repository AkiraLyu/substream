use std::{fs, os::unix::fs::PermissionsExt, time::Duration};

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use substream::{
    auth::{Token, create_token},
    server::{ServerConfig, router},
    video::VideoConfig,
};
use tokio::{sync::watch, time::timeout};
use tower::ServiceExt;

async fn request(
    app: &Router,
    method: &str,
    path: &str,
    token: &str,
    origin: &str,
    body: Value,
) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("Authorization", format!("Bearer {token}"))
                .header("Origin", origin)
                .header("Content-Type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn finished(app: &Router, path: &str, token: &str, origin: &str) -> Value {
    timeout(Duration::from_secs(5), async {
        loop {
            let (status, job) = request(app, "GET", path, token, origin, Value::Null).await;
            assert_eq!(status, StatusCode::OK);
            if matches!(
                job["stage"].as_str(),
                Some("completed" | "failed" | "cancelled")
            ) {
                return job;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("video job did not stop")
}

#[tokio::test]
async fn video_jobs_require_pairing_and_can_be_cancelled_retried_and_stopped() {
    let directory = tempfile::tempdir().unwrap();
    let token_path = directory.path().join("token");
    create_token(&token_path).unwrap();
    let token = fs::read_to_string(&token_path).unwrap().trim().to_owned();
    let model = directory.path().join("model.bin");
    fs::write(&model, b"fixture").unwrap();
    let downloader = directory.path().join("downloader");
    // A slow external download leaves a partial file, as an interrupted transfer would.
    fs::write(
        &downloader,
        "#!/bin/sh\nprintf partial > media.part\nsleep 30\n",
    )
    .unwrap();
    fs::set_permissions(&downloader, fs::Permissions::from_mode(0o755)).unwrap();
    let output_dir = directory.path().join("output");
    let origin = "chrome-extension://test";
    let (shutdown, receiver) = watch::channel(false);
    let app = router(
        ServerConfig {
            address: "127.0.0.1:0".parse().unwrap(),
            token: Token::read(&token_path).unwrap(),
            allowed_origins: vec![origin.into()],
            recognizer: None,
            video: Some(VideoConfig {
                output_dir: output_dir.clone(),
                model: Some(model),
                cookies_from_browser: None,
                cookies_file: None,
                yt_dlp_bin: downloader.clone(),
                ffmpeg_bin: "ffmpeg".into(),
                whisper_bin: "whisper-cli".into(),
                language: "auto".into(),
                threads: 1,
                timeout_secs: 60,
            }),
        },
        receiver,
    );
    let endpoint = "/v1/video/jobs";
    let body = json!({"url":"https://example.com/video"});
    assert_eq!(
        request(&app, "POST", endpoint, "wrong", origin, body.clone())
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        request(
            &app,
            "POST",
            endpoint,
            &token,
            "https://untrusted.example",
            body.clone()
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        request(&app, "OPTIONS", endpoint, "", origin, Value::Null)
            .await
            .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        request(
            &app,
            "POST",
            endpoint,
            &token,
            origin,
            json!({"url":"file:///etc/passwd"})
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );

    let (status, job) = request(&app, "POST", endpoint, &token, origin, body.clone()).await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let path = format!("{endpoint}/{}", job["id"].as_str().unwrap());
    assert_eq!(
        request(&app, "POST", endpoint, &token, origin, body.clone())
            .await
            .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        request(
            &app,
            "GET",
            &format!("{path}/document"),
            &token,
            origin,
            Value::Null
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    // Wait for an actual partial download before cancelling it.
    timeout(Duration::from_secs(5), async {
        while !output_dir.exists()
            || !fs::read_dir(&output_dir)
                .unwrap()
                .any(|entry| entry.unwrap().path().join("media.part").exists())
        {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    request(&app, "DELETE", &path, &token, origin, Value::Null).await;
    assert_eq!(
        finished(&app, &path, &token, origin).await["stage"],
        "cancelled"
    );
    assert_eq!(fs::read_dir(&output_dir).unwrap().count(), 0);

    // Failure remains visible and releases the slot for another request.
    fs::write(
        &downloader,
        "#!/bin/sh\necho 'download failed' >&2\nexit 1\n",
    )
    .unwrap();
    let (status, job) = request(&app, "POST", endpoint, &token, origin, body.clone()).await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let path = format!("{endpoint}/{}", job["id"].as_str().unwrap());
    let failed = finished(&app, &path, &token, origin).await;
    assert_eq!(failed["stage"], "failed");
    assert!(
        failed["error"]
            .as_str()
            .unwrap()
            .contains("download failed")
    );
    assert!(failed["result"].is_null());
    assert_eq!(fs::read_dir(&output_dir).unwrap().count(), 0);

    fs::write(&downloader, "#!/bin/sh\nsleep 30\n").unwrap();
    let (status, job) = request(&app, "POST", endpoint, &token, origin, body).await;
    assert_eq!(status, StatusCode::ACCEPTED);
    shutdown.send(true).unwrap();
    let path = format!("{endpoint}/{}", job["id"].as_str().unwrap());
    assert_eq!(
        finished(&app, &path, &token, origin).await["stage"],
        "cancelled"
    );
    assert_eq!(fs::read_dir(&output_dir).unwrap().count(), 0);
}
