//! OpenMontage HTTP Handlers
//!
//! E1-E8 (user routes) + I1 (internal callback).

use axum::{
    extract::{Path, Query},
    response::sse::{Event, Sse},
    Extension, Json,
};
use futures::stream::Stream;
use glance_mind_db::entity::user::User;
use serde::Deserialize;
use std::convert::Infallible;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;

use crate::dto::openmontage_dto::{
    ApprovalDto, CreateJobDto, JobEventDto, JobEventsDto, PipelineInfoDto, PipelinesDto,
    PreflightDto,
};
use crate::error::api_error::ApiError;
use crate::repository::openmontage_repository::OpenMontageJobStore;
use crate::response::unified_response::ApiResponse;
use crate::service::openmontage_client::OpenMontageClient;
use crate::service::openmontage_service::OpenMontageService;
use crate::service::openmontage_stream_hub::{OpenMontageSseEvent, OpenMontageStreamHub};

// ============================================================================
// E1: GET /openmontage/preflight
// ============================================================================

pub async fn get_preflight(
    Extension(client): Extension<std::sync::Arc<dyn OpenMontageClient>>,
) -> Result<Json<ApiResponse<PreflightDto>>, ApiError> {
    let preflight = client
        .read_preflight()
        .map_err(|e| ApiError::InternalServerError(format!("preflight read failed: {}", e)))?;

    let dto = preflight.unwrap_or_else(|| PreflightDto {
        passed: false,
        status: "warming_up".to_string(),
        blocking: vec![],
        warnings: vec![],
        estimated_cost_cents: None,
    });

    Ok(Json(ApiResponse::success(dto)))
}

// ============================================================================
// E2: GET /openmontage/pipelines
// ============================================================================

pub async fn get_pipelines(
    Extension(client): Extension<std::sync::Arc<dyn OpenMontageClient>>,
) -> Result<Json<ApiResponse<PipelinesDto>>, ApiError> {
    let pipelines = client
        .read_pipelines()
        .map_err(|e| ApiError::InternalServerError(format!("pipelines read failed: {}", e)))?;

    let dto = pipelines.unwrap_or_else(|| PipelinesDto {
        pipelines: vec![PipelineInfoDto {
            name: "animated-explainer".to_string(),
            description: "Topic to fully generated explainer".to_string(),
            stability: "production".to_string(),
        }],
    });

    Ok(Json(ApiResponse::success(dto)))
}

// ============================================================================
// E3: POST /openmontage/jobs
// ============================================================================

pub async fn create_job(
    Extension(user): Extension<User>,
    Extension(service): Extension<OpenMontageService>,
    Json(dto): Json<CreateJobDto>,
) -> Result<Json<ApiResponse<crate::dto::openmontage_dto::JobSnapshotDto>>, ApiError> {
    let snapshot = service
        .create_job(user.id, "default-tenant", dto)
        .map_err(|e| ApiError::BadRequest(format!("create job failed: {}", e)))?;

    Ok(Json(ApiResponse::success(snapshot)))
}

// ============================================================================
// E4: GET /openmontage/jobs/:job_id
// ============================================================================

pub async fn get_job(
    Extension(_user): Extension<User>,
    Extension(service): Extension<OpenMontageService>,
    Path(job_id): Path<String>,
) -> Result<Json<ApiResponse<crate::dto::openmontage_dto::JobSnapshotDto>>, ApiError> {
    let snapshot = service
        .get_job(&job_id)
        .map_err(|e| ApiError::InternalServerError(format!("get job failed: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("Job not found".to_string()))?;

    Ok(Json(ApiResponse::success(snapshot)))
}

// ============================================================================
// E5: GET /openmontage/jobs/:job_id/events?after=<seq>&limit=<n>
// ============================================================================

#[derive(Deserialize)]
pub struct EventsQuery {
    #[serde(default)]
    after: i64,
    #[serde(default = "default_limit")]
    limit: i64,
}

fn default_limit() -> i64 {
    50
}

pub async fn get_events(
    Extension(_user): Extension<User>,
    Extension(store): Extension<std::sync::Arc<dyn OpenMontageJobStore>>,
    Path(job_id): Path<String>,
    Query(query): Query<EventsQuery>,
) -> Result<Json<ApiResponse<JobEventsDto>>, ApiError> {
    let events = store
        .list_events(&job_id, query.after, query.limit)
        .map_err(|e| ApiError::InternalServerError(format!("list events failed: {}", e)))?;

    let next_seq = events.last().map(|e| e.sequence as u64 + 1).unwrap_or(1);

    let dtos: Vec<JobEventDto> = events
        .iter()
        .map(|e| JobEventDto {
            sequence: e.sequence,
            event_id: e.event_id.clone(),
            event_type: e.event_type.clone(),
            status: e.status.clone(),
            stage: e.stage.clone(),
            progress_pct: e.progress_pct,
            event_json: e.event_json.clone(),
            emitted_at: e.emitted_at.map(|t| t.to_rfc3339()).unwrap_or_default(),
        })
        .collect();

    Ok(Json(ApiResponse::success(JobEventsDto {
        events: dtos,
        next_sequence: next_seq,
    })))
}

// ============================================================================
// E6: GET /openmontage/jobs/:job_id/stream
// ============================================================================

#[derive(Deserialize)]
pub struct StreamQuery {
    #[serde(default)]
    after: i64,
}

pub async fn stream_job(
    Extension(_user): Extension<User>,
    Extension(service): Extension<OpenMontageService>,
    Extension(store): Extension<std::sync::Arc<dyn OpenMontageJobStore>>,
    Extension(hub): Extension<OpenMontageStreamHub>,
    Path(job_id): Path<String>,
    Query(query): Query<StreamQuery>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
    let snapshot = service
        .get_job(&job_id)
        .map_err(|e| ApiError::InternalServerError(format!("get job failed: {}", e)))?;

    let rx = hub.subscribe(&job_id).await;
    let (tx_out, rx_out) = tokio::sync::mpsc::channel::<OpenMontageSseEvent>(64);

    // Send snapshot
    if let Some(snap) = snapshot {
        let snap_event = OpenMontageSseEvent {
            event_type: "job_snapshot".to_string(),
            job_id: snap.job_id.clone(),
            project_id: snap.project_id.clone(),
            sequence: snap.last_event_sequence,
            status: snap.status.clone(),
            stage: snap.current_stage.clone(),
            progress_pct: snap.progress_pct,
            payload: snap.snapshot_json.clone(),
        };
        let _ = tx_out.send(snap_event).await;

        // Send backlog events after query.after
        let backlog = store
            .list_events(&job_id, query.after, 100)
            .map_err(|e| ApiError::InternalServerError(format!("backlog failed: {}", e)))?;

        for event in backlog {
            let evt = OpenMontageSseEvent {
                event_type: event.event_type.clone(),
                job_id: job_id.clone(),
                project_id: snap.project_id.clone(),
                sequence: event.sequence,
                status: event.status.clone().unwrap_or_default(),
                stage: event.stage.clone(),
                progress_pct: event.progress_pct.unwrap_or(0),
                payload: event.event_json.clone(),
            };
            let _ = tx_out.send(evt).await;
        }
    }

    let heartbeat_tx = tx_out.clone();

    // Forward live events
    tokio::spawn(async move {
        let mut stream = ReceiverStream::new(rx);
        while let Some(event) = stream.next().await {
            if tx_out.send(event).await.is_err() {
                break;
            }
        }
    });

    // Heartbeat every 15s
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(15));
        loop {
            interval.tick().await;
            let hb = OpenMontageSseEvent {
                event_type: "heartbeat".to_string(),
                job_id: String::new(),
                project_id: String::new(),
                sequence: 0,
                status: String::new(),
                stage: None,
                progress_pct: 0,
                payload: serde_json::json!({}),
            };
            if heartbeat_tx.send(hb).await.is_err() {
                break;
            }
        }
    });

    let sse_stream = ReceiverStream::new(rx_out).map(|event| {
        let data = serde_json::to_string(&event).unwrap_or_default();
        Ok(Event::default().event(&event.event_type).data(data))
    });

    Ok(Sse::new(sse_stream))
}

// ============================================================================
// E7: POST /openmontage/jobs/:job_id/approvals
// ============================================================================

pub async fn submit_approval(
    Extension(_user): Extension<User>,
    Extension(service): Extension<OpenMontageService>,
    Path(job_id): Path<String>,
    Json(approval): Json<ApprovalDto>,
) -> Result<Json<ApiResponse<crate::dto::openmontage_dto::JobSnapshotDto>>, ApiError> {
    let snapshot = service
        .submit_approval(&job_id, approval)
        .map_err(|e| ApiError::BadRequest(format!("approval failed: {}", e)))?;

    Ok(Json(ApiResponse::success(snapshot)))
}

// ============================================================================
// E8: POST /openmontage/jobs/:job_id/cancel
// ============================================================================

pub async fn cancel_job(
    Extension(_user): Extension<User>,
    Extension(service): Extension<OpenMontageService>,
    Path(job_id): Path<String>,
) -> Result<Json<ApiResponse<crate::dto::openmontage_dto::CancelResultDto>>, ApiError> {
    let result = service
        .cancel_job(&job_id)
        .map_err(|e| ApiError::InternalServerError(format!("cancel failed: {}", e)))?;

    Ok(Json(ApiResponse::success(result)))
}

// ============================================================================
// I1: POST /internal/openmontage/callback
// ============================================================================

#[derive(serde::Deserialize, serde::Serialize)]
pub struct OpenMontageJobEvent {
    pub version: String,
    pub event_id: String,
    pub sequence: i64,
    pub job: JobIdentifier,
    pub event_type: String,
    pub status: String,
    #[serde(default)]
    pub stage: String,
    pub progress_pct: i32,
    pub emitted_at: String,
    #[serde(default)]
    pub artifacts: Vec<ArtifactDto>,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct JobIdentifier {
    pub job_id: String,
    pub project_id: String,
    #[serde(default)]
    pub request_id: String,
    #[serde(default)]
    pub correlation_id: String,
    #[serde(default)]
    pub idempotency_key: String,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct ArtifactDto {
    pub artifact_id: String,
    pub kind: String,
    pub role: String,
    #[serde(default)]
    pub uri: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub mime_type: String,
    #[serde(default)]
    pub width_px: i32,
    #[serde(default)]
    pub height_px: i32,
    #[serde(default)]
    pub duration_ms: i32,
    #[serde(default)]
    pub bytes: i64,
}

#[derive(serde::Serialize)]
pub struct CallbackAck {
    pub received: bool,
    pub event_id: String,
    pub next_expected_sequence: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sync_required: Option<bool>,
}

pub async fn ingest_callback(
    Extension(store): Extension<std::sync::Arc<dyn OpenMontageJobStore>>,
    Extension(hub): Extension<OpenMontageStreamHub>,
    Json(event): Json<OpenMontageJobEvent>,
) -> Result<Json<ApiResponse<CallbackAck>>, ApiError> {
    use crate::repository::openmontage_repository::NewJobEvent;

    let job_id = &event.job.job_id;

    let new_event = NewJobEvent {
        job_id: job_id.to_string(),
        sequence: event.sequence,
        event_id: event.event_id.clone(),
        event_type: event.event_type.clone(),
        status: Some(event.status.clone()),
        event_json: serde_json::to_value(&event).unwrap_or_default(),
    };

    let append_result = store
        .append_event(new_event.clone())
        .map_err(|e| ApiError::InternalServerError(format!("append event failed: {}", e)))?;

    if append_result.inserted {
        store.update_from_event(&new_event).map_err(|e| {
            ApiError::InternalServerError(format!("update from event failed: {}", e))
        })?;

        // Publish to SSE hub
        let sse_event = OpenMontageSseEvent {
            event_type: event.event_type.clone(),
            job_id: job_id.to_string(),
            project_id: event.job.project_id.clone(),
            sequence: event.sequence,
            status: event.status.clone(),
            stage: Some(event.stage.clone()),
            progress_pct: event.progress_pct,
            payload: serde_json::to_value(&event).unwrap_or_default(),
        };
        hub.publish(sse_event).await;
    }

    let job = store
        .get_job(job_id)
        .map_err(|e| ApiError::InternalServerError(format!("get job failed: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("Job not found".to_string()))?;

    let ack = CallbackAck {
        received: true,
        event_id: event.event_id,
        next_expected_sequence: job.next_event_sequence,
        sync_required: if append_result.gap { Some(true) } else { None },
    };

    Ok(Json(ApiResponse::success(ack)))
}
