use std::{
    collections::VecDeque,
    io::Read,
    sync::{Arc, Mutex},
};

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, Request, State},
    http::{HeaderMap, HeaderValue, Method, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use substream_backends::{download::validate_video_url, process::Cancellation};
use substream_protocol::video::{VideoDocument, VideoJob, VideoRequest, VideoStage};
use tokio::sync::{Semaphore, watch};

use crate::{
    auth::Token,
    video::{VideoConfig, new_job_id, run_video},
};

#[derive(Clone)]
struct Access {
    token: Token,
    origins: Vec<String>,
}

#[derive(Clone)]
struct JobRecord {
    job: Arc<Mutex<VideoJob>>,
    cancellation: Cancellation,
}

#[derive(Clone)]
struct Jobs {
    config: Option<VideoConfig>,
    records: Arc<Mutex<VecDeque<JobRecord>>>,
    slot: Arc<Semaphore>,
    shutdown: watch::Receiver<bool>,
}

struct ApiError(StatusCode, &'static str, String);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.0,
            Json(serde_json::json!({"code":self.1, "message":self.2})),
        )
            .into_response()
    }
}

pub(crate) fn router(
    config: Option<VideoConfig>,
    token: Token,
    origins: Vec<String>,
    shutdown: watch::Receiver<bool>,
) -> Router {
    Router::new()
        .route("/v1/video/jobs", post(submit).options(preflight))
        .route(
            "/v1/video/jobs/{id}",
            get(status).delete(cancel).options(preflight),
        )
        .route(
            "/v1/video/jobs/{id}/document",
            get(document).options(preflight),
        )
        .layer(DefaultBodyLimit::max(16 * 1024))
        .route_layer(middleware::from_fn_with_state(
            Access { token, origins },
            authorize,
        ))
        .with_state(Jobs {
            config,
            records: Arc::default(),
            slot: Arc::new(Semaphore::new(1)),
            shutdown,
        })
}

async fn preflight() -> StatusCode {
    StatusCode::NO_CONTENT
}

async fn authorize(State(access): State<Access>, request: Request, next: Next) -> Response {
    let origin = request.headers().get(header::ORIGIN).cloned();
    if origin.as_ref().is_some_and(|origin| {
        origin
            .to_str()
            .ok()
            .is_none_or(|value| !access.origins.iter().any(|s| s == value))
    }) {
        return ApiError(
            StatusCode::FORBIDDEN,
            "forbidden_origin",
            "origin is not allowed".into(),
        )
        .into_response();
    }
    let mut response =
        if request.method() != Method::OPTIONS && !authorized(request.headers(), &access.token) {
            ApiError(
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "a valid bearer token is required".into(),
            )
            .into_response()
        } else {
            next.run(request).await
        };
    if let Some(origin) = origin {
        response
            .headers_mut()
            .insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin);
        response
            .headers_mut()
            .insert(header::VARY, HeaderValue::from_static("Origin"));
        response.headers_mut().insert(
            header::ACCESS_CONTROL_ALLOW_METHODS,
            HeaderValue::from_static("GET, POST, DELETE, OPTIONS"),
        );
        response.headers_mut().insert(
            header::ACCESS_CONTROL_ALLOW_HEADERS,
            HeaderValue::from_static("Authorization, Content-Type"),
        );
    }
    response
}

fn authorized(headers: &HeaderMap, token: &Token) -> bool {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .is_some_and(|value| token.matches(value))
}

impl Jobs {
    fn find(&self, id: &str) -> Result<JobRecord, ApiError> {
        self.records
            .lock()
            .expect("job records lock")
            .iter()
            .find(|record| record.job.lock().expect("job lock").id == id)
            .cloned()
            .ok_or_else(|| {
                ApiError(
                    StatusCode::NOT_FOUND,
                    "job_not_found",
                    "video job is not available in this server session".into(),
                )
            })
    }
}

async fn submit(
    State(jobs): State<Jobs>,
    Json(request): Json<VideoRequest>,
) -> Result<(StatusCode, Json<VideoJob>), ApiError> {
    let config = jobs.config.clone().ok_or_else(|| {
        ApiError(
            StatusCode::SERVICE_UNAVAILABLE,
            "video_disabled",
            "start the service with --video-config to enable video jobs".into(),
        )
    })?;
    if *jobs.shutdown.borrow() {
        return Err(ApiError(
            StatusCode::SERVICE_UNAVAILABLE,
            "shutting_down",
            "the service is stopping".into(),
        ));
    }
    validate_video_url(&request.url)
        .map_err(|e| ApiError(StatusCode::BAD_REQUEST, "invalid_url", e.to_string()))?;
    let permit = Arc::clone(&jobs.slot).try_acquire_owned().map_err(|_| {
        ApiError(
            StatusCode::CONFLICT,
            "video_busy",
            "another video job is active".into(),
        )
    })?;
    let job = VideoJob {
        id: new_job_id().map_err(|e| {
            ApiError(
                StatusCode::INTERNAL_SERVER_ERROR,
                "create_job",
                e.to_string(),
            )
        })?,
        stage: VideoStage::Queued,
        url: request.url.clone(),
        result: None,
        error: None,
    };
    let record = JobRecord {
        job: Arc::new(Mutex::new(job.clone())),
        cancellation: Cancellation::default(),
    };
    {
        let mut records = jobs.records.lock().expect("job records lock");
        if records.len() >= 32 {
            records.pop_front();
        }
        records.push_back(record.clone());
    }
    let worker_record = record.clone();
    let mut shutdown = jobs.shutdown.clone();
    let worker = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        run_video(
            &config,
            &request.url,
            &worker_record.cancellation,
            |stage| {
                let mut job = worker_record.job.lock().expect("job lock");
                if job.stage != VideoStage::Cancelling {
                    job.stage = stage;
                }
                Ok(())
            },
        )
    });
    tokio::spawn(async move {
        tokio::pin!(worker);
        let result = tokio::select! {
            result = &mut worker => result,
            _ = shutdown.changed() => {
                record.cancellation.cancel();
                worker.await
            }
        };
        let mut job = record.job.lock().expect("job lock");
        match result {
            Ok(Ok(result)) => {
                job.stage = VideoStage::Completed;
                job.result = Some(result);
            }
            error => {
                job.stage = if record.cancellation.is_cancelled() {
                    VideoStage::Cancelled
                } else {
                    VideoStage::Failed
                };
                job.error = Some(match error {
                    Ok(Err(error)) => format!("{error:#}"),
                    Err(error) => format!("video worker stopped: {error}"),
                    Ok(Ok(_)) => unreachable!(),
                });
            }
        }
    });
    Ok((StatusCode::ACCEPTED, Json(job)))
}

async fn status(
    State(jobs): State<Jobs>,
    Path(id): Path<String>,
) -> Result<Json<VideoJob>, ApiError> {
    let record = jobs.find(&id)?;
    Ok(Json(record.job.lock().expect("job lock").clone()))
}

async fn cancel(
    State(jobs): State<Jobs>,
    Path(id): Path<String>,
) -> Result<Json<VideoJob>, ApiError> {
    let record = jobs.find(&id)?;
    let mut job = record.job.lock().expect("job lock");
    if !job.stage.is_terminal() {
        record.cancellation.cancel();
        job.stage = VideoStage::Cancelling;
    }
    Ok(Json(job.clone()))
}

async fn document(
    State(jobs): State<Jobs>,
    Path(id): Path<String>,
) -> Result<Json<VideoDocument>, ApiError> {
    let record = jobs.find(&id)?;
    let path = record
        .job
        .lock()
        .expect("job lock")
        .result
        .as_ref()
        .map(|result| result.document.clone())
        .ok_or_else(|| {
            ApiError(
                StatusCode::CONFLICT,
                "not_complete",
                "the video document is not ready".into(),
            )
        })?;
    let document = tokio::task::spawn_blocking(move || -> anyhow::Result<VideoDocument> {
        let mut bytes = Vec::new();
        std::fs::File::open(path)?
            .take(32 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)?;
        anyhow::ensure!(
            bytes.len() <= 32 * 1024 * 1024,
            "video document exceeds 32 MiB"
        );
        Ok(serde_json::from_slice(&bytes)?)
    })
    .await
    .map_err(|e| {
        ApiError(
            StatusCode::INTERNAL_SERVER_ERROR,
            "document_failed",
            e.to_string(),
        )
    })?
    .map_err(|e| {
        ApiError(
            StatusCode::INTERNAL_SERVER_ERROR,
            "document_failed",
            e.to_string(),
        )
    })?;
    Ok(Json(document))
}
