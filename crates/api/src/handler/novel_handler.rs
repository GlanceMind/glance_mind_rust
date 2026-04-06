//! Novel engine HTTP handlers — wired to NovelService + NovelWorkerDispatcher.

use axum::{
    extract::{Multipart, Path, Query},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use chrono::Utc;
use glance_mind_db::entity::user::User;
use std::time::Duration;
use uuid::Uuid;

use crate::dto::novel_dto::*;
use crate::service::novel_service::NovelService;
use crate::service::novel_worker_dispatcher::NovelWorkerDispatcher;

fn ok_response(data: serde_json::Value) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "code": 1000,
            "msg": "success",
            "msg_cn": "成功",
            "data": data,
        })),
    )
}

fn created_response(data: serde_json::Value) -> impl IntoResponse {
    (
        StatusCode::CREATED,
        Json(serde_json::json!({
            "code": 1000,
            "msg": "success",
            "msg_cn": "成功",
            "data": data,
        })),
    )
}

fn err_response(status: StatusCode, msg: &str) -> impl IntoResponse {
    (
        status,
        Json(serde_json::json!({
            "code": status.as_u16(),
            "msg": msg,
            "msg_cn": msg,
        })),
    )
}

// ---------------------------------------------------------------------------
// Helpers for worker-dispatch endpoints
// ---------------------------------------------------------------------------

fn require_dispatcher(
    dispatcher: &Option<NovelWorkerDispatcher>,
) -> Result<&NovelWorkerDispatcher, impl IntoResponse> {
    dispatcher
        .as_ref()
        .ok_or_else(|| err_response(StatusCode::SERVICE_UNAVAILABLE, "novel worker unavailable"))
}

fn build_envelope(
    project_id: &str,
    job: &glance_mind_db::entity::novel::NovelJob,
    stage_code: &str,
    task_type: NovelWorkerTaskType,
    chapter_number: Option<i32>,
    payload: serde_json::Value,
) -> NovelWorkerTaskEnvelope {
    NovelWorkerTaskEnvelope {
        task_id: Uuid::new_v4().to_string(),
        project_id: project_id.to_string(),
        job_id: job.id,
        stage_code: stage_code.to_string(),
        task_type,
        interaction_version: 1,
        chapter_number,
        payload,
        created_at: Utc::now(),
    }
}

fn job_accepted_response(project_id: &str, job_id: i64) -> impl IntoResponse {
    created_response(serde_json::json!({
        "project_id": project_id,
        "job_id": job_id,
        "status": "pending",
        "accepted": true,
    }))
}

fn knowledge_import_accepted_response(
    project_id: &str,
    job_id: i64,
    knowledge_import_id: i64,
) -> impl IntoResponse {
    created_response(serde_json::json!({
        "project_id": project_id,
        "job_id": job_id,
        "knowledge_import_id": knowledge_import_id,
        "status": "pending",
        "accepted": true,
    }))
}

/// OpenAI-compatible chat completions URL from a stored base URL (adds `/v1` when missing).
fn openai_chat_completions_url(base_url: &str) -> String {
    let b = base_url.trim_end_matches('/');
    if b.ends_with("/v1") {
        format!("{b}/chat/completions")
    } else {
        format!("{b}/v1/chat/completions")
    }
}

/// OpenAI-compatible embeddings URL from a stored base URL (adds `/v1` when missing).
fn openai_embeddings_url(base_url: &str) -> String {
    let b = base_url.trim_end_matches('/');
    if b.ends_with("/v1") {
        format!("{b}/embeddings")
    } else {
        format!("{b}/v1/embeddings")
    }
}

// =========================================================================
// Project & Config (1-9)
// =========================================================================

pub async fn create_project(
    Extension(user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Json(req): Json<NovelProjectCreateRequest>,
) -> impl IntoResponse {
    match service.create_project(user.id, &req) {
        Ok(project) => {
            created_response(serde_json::to_value(&project).unwrap_or_default()).into_response()
        }
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response(),
    }
}

pub async fn list_projects(
    Extension(user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Query(query): Query<NovelProjectListQuery>,
) -> impl IntoResponse {
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);
    match service.list_projects(user.id, query.status.as_deref(), page, page_size) {
        Ok((items, total)) => ok_response(serde_json::json!({
            "list": serde_json::to_value(&items).unwrap_or_default(),
            "total": total,
            "page": page,
            "page_size": page_size,
        }))
        .into_response(),
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response(),
    }
}

pub async fn get_project(
    Extension(user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Path(project_id): Path<String>,
) -> impl IntoResponse {
    match service.get_project(&project_id, user.id) {
        Ok(project) => {
            let snapshot = service
                .list_config_snapshots(&project_id)
                .ok()
                .and_then(|snaps| snaps.into_iter().find(|s| s.is_current))
                .map(|s| serde_json::to_value(&s).unwrap_or_default());
            let mut obj = serde_json::to_value(&project).unwrap_or_default();
            if let (Some(map), Some(snap_val)) = (obj.as_object_mut(), snapshot) {
                map.insert("current_config_snapshot".to_string(), snap_val);
            }
            ok_response(obj).into_response()
        }
        Err(e) => err_response(StatusCode::NOT_FOUND, &e).into_response(),
    }
}

pub async fn delete_project(
    Extension(user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Path(project_id): Path<String>,
) -> impl IntoResponse {
    match service.delete_project(&project_id, user.id) {
        Ok(()) => ok_response(serde_json::json!({
            "project_id": project_id,
            "deleted": true,
        }))
        .into_response(),
        Err(e) => err_response(StatusCode::NOT_FOUND, &e).into_response(),
    }
}

pub async fn cancel_project(
    Extension(user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Path(project_id): Path<String>,
) -> impl IntoResponse {
    match service.cancel_project(&project_id, user.id) {
        Ok(()) => ok_response(serde_json::json!({
            "project_id": project_id,
            "status": "cancelled",
            "accepted": true,
        }))
        .into_response(),
        Err(e) => err_response(StatusCode::NOT_FOUND, &e).into_response(),
    }
}

pub async fn update_project(
    Extension(user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Path(project_id): Path<String>,
    Json(req): Json<NovelProjectUpdateRequest>,
) -> impl IntoResponse {
    match service.update_project(&project_id, user.id, &req) {
        Ok(project) => {
            ok_response(serde_json::to_value(&project).unwrap_or_default()).into_response()
        }
        Err(e) => err_response(StatusCode::NOT_FOUND, &e).into_response(),
    }
}

pub async fn list_config_snapshots(
    Extension(_user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Path(project_id): Path<String>,
) -> impl IntoResponse {
    match service.list_config_snapshots(&project_id) {
        Ok(snaps) => ok_response(serde_json::to_value(&snaps).unwrap_or_default()).into_response(),
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response(),
    }
}

pub async fn create_config_snapshot(
    Extension(_user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Path(project_id): Path<String>,
    Json(req): Json<NovelConfigSnapshotCreateRequest>,
) -> impl IntoResponse {
    match service.create_config_snapshot(&project_id, &req) {
        Ok(snap) => {
            created_response(serde_json::to_value(&snap).unwrap_or_default()).into_response()
        }
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response(),
    }
}

pub async fn activate_config_snapshot(
    Extension(_user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Path((project_id, snapshot_id)): Path<(String, i64)>,
) -> impl IntoResponse {
    match service.activate_config_snapshot(&project_id, snapshot_id) {
        Ok(()) => ok_response(serde_json::json!({
            "project_id": project_id,
            "snapshot_id": snapshot_id,
            "activated": true,
        }))
        .into_response(),
        Err(e) => err_response(StatusCode::NOT_FOUND, &e).into_response(),
    }
}

// =========================================================================
// LLM Profiles (10-14)
// =========================================================================

pub async fn list_llm_profiles(
    Extension(user): Extension<User>,
    Extension(service): Extension<NovelService>,
) -> impl IntoResponse {
    match service.list_llm_profiles(user.id) {
        Ok(profiles) => {
            ok_response(serde_json::to_value(&profiles).unwrap_or_default()).into_response()
        }
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response(),
    }
}

pub async fn create_llm_profile(
    Extension(user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Json(req): Json<NovelLlmProfileCreateRequest>,
) -> impl IntoResponse {
    match service.create_llm_profile(user.id, &req) {
        Ok(profile) => {
            created_response(serde_json::to_value(&profile).unwrap_or_default()).into_response()
        }
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response(),
    }
}

pub async fn update_llm_profile(
    Extension(user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Path(id): Path<i64>,
    Json(req): Json<NovelLlmProfileUpdateRequest>,
) -> impl IntoResponse {
    match service.update_llm_profile(id, user.id, &req) {
        Ok(profile) => {
            ok_response(serde_json::to_value(&profile).unwrap_or_default()).into_response()
        }
        Err(e) => err_response(StatusCode::NOT_FOUND, &e).into_response(),
    }
}

pub async fn delete_llm_profile(
    Extension(user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    match service.delete_llm_profile(id, user.id) {
        Ok(()) => ok_response(serde_json::json!({ "id": id, "deleted": true })).into_response(),
        Err(e) => err_response(StatusCode::NOT_FOUND, &e).into_response(),
    }
}

pub async fn test_llm_profile(
    Extension(user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Path(id): Path<i64>,
    Json(_req): Json<NovelLlmProfileTestRequest>,
) -> impl IntoResponse {
    let profile = match service.get_llm_profile(id, user.id) {
        Ok(p) => p,
        Err(e) => return err_response(StatusCode::NOT_FOUND, &e).into_response(),
    };

    let url = openai_chat_completions_url(&profile.base_url);
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return err_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("http client: {e}"),
            )
            .into_response()
        }
    };

    let body = serde_json::json!({
        "model": profile.model_name,
        "messages": [{"role": "user", "content": "Reply OK"}],
        "max_tokens": 10_i32,
    });

    let resp = match client
        .post(url)
        .header("Authorization", format!("Bearer {}", profile.api_key))
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            return err_response(StatusCode::BAD_GATEWAY, &format!("request failed: {e}"))
                .into_response()
        }
    };

    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    let preview: String = text.chars().take(512).collect();

    if status.is_success() {
        ok_response(serde_json::json!({
            "ok": true,
            "http_status": status.as_u16(),
            "preview": preview,
        }))
        .into_response()
    } else {
        let detail = if preview.is_empty() {
            "(empty body)".to_string()
        } else {
            preview
        };
        err_response(
            StatusCode::BAD_GATEWAY,
            &format!("provider returned {}: {}", status.as_u16(), detail),
        )
        .into_response()
    }
}

// =========================================================================
// Embedding Profiles (15-19)
// =========================================================================

pub async fn list_embedding_profiles(
    Extension(user): Extension<User>,
    Extension(service): Extension<NovelService>,
) -> impl IntoResponse {
    match service.list_embedding_profiles(user.id) {
        Ok(profiles) => {
            ok_response(serde_json::to_value(&profiles).unwrap_or_default()).into_response()
        }
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response(),
    }
}

pub async fn create_embedding_profile(
    Extension(user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Json(req): Json<NovelEmbeddingProfileCreateRequest>,
) -> impl IntoResponse {
    match service.create_embedding_profile(user.id, &req) {
        Ok(profile) => {
            created_response(serde_json::to_value(&profile).unwrap_or_default()).into_response()
        }
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response(),
    }
}

pub async fn update_embedding_profile(
    Extension(user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Path(id): Path<i64>,
    Json(req): Json<NovelEmbeddingProfileUpdateRequest>,
) -> impl IntoResponse {
    match service.update_embedding_profile(id, user.id, &req) {
        Ok(profile) => {
            ok_response(serde_json::to_value(&profile).unwrap_or_default()).into_response()
        }
        Err(e) => err_response(StatusCode::NOT_FOUND, &e).into_response(),
    }
}

pub async fn delete_embedding_profile(
    Extension(user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    match service.delete_embedding_profile(id, user.id) {
        Ok(()) => ok_response(serde_json::json!({ "id": id, "deleted": true })).into_response(),
        Err(e) => err_response(StatusCode::NOT_FOUND, &e).into_response(),
    }
}

pub async fn test_embedding_profile(
    Extension(user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Path(id): Path<i64>,
    Json(_req): Json<NovelEmbeddingProfileTestRequest>,
) -> impl IntoResponse {
    let profile = match service.get_embedding_profile(id, user.id) {
        Ok(p) => p,
        Err(e) => return err_response(StatusCode::NOT_FOUND, &e).into_response(),
    };

    let url = openai_embeddings_url(&profile.base_url);
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return err_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("http client: {e}"),
            )
            .into_response()
        }
    };

    let body = serde_json::json!({
        "model": profile.model_name,
        "input": "test",
    });

    let resp = match client
        .post(url)
        .header("Authorization", format!("Bearer {}", profile.api_key))
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            return err_response(StatusCode::BAD_GATEWAY, &format!("request failed: {e}"))
                .into_response()
        }
    };

    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    let preview: String = text.chars().take(512).collect();

    if status.is_success() {
        ok_response(serde_json::json!({
            "ok": true,
            "http_status": status.as_u16(),
            "preview": preview,
        }))
        .into_response()
    } else {
        let detail = if preview.is_empty() {
            "(empty body)".to_string()
        } else {
            preview
        };
        err_response(
            StatusCode::BAD_GATEWAY,
            &format!("provider returned {}: {}", status.as_u16(), detail),
        )
        .into_response()
    }
}

// =========================================================================
// Architecture (20-22)
// =========================================================================

pub async fn generate_architecture(
    Extension(user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Extension(dispatcher): Extension<Option<NovelWorkerDispatcher>>,
    Path(project_id): Path<String>,
    Json(req): Json<NovelArchitectureGenerateRequest>,
) -> impl IntoResponse {
    let dispatcher = match require_dispatcher(&dispatcher) {
        Ok(d) => d,
        Err(e) => return e.into_response(),
    };

    let payload = serde_json::json!({
        "config_snapshot_id": req.config_snapshot_id,
        "override_user_guidance": req.override_user_guidance,
    });
    let job = match service.create_job(
        &project_id,
        user.id,
        "n01_architecture",
        "generate_architecture",
        None,
        payload.clone(),
    ) {
        Ok(j) => j,
        Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response(),
    };

    let envelope = build_envelope(
        &project_id,
        &job,
        "n01_architecture",
        NovelWorkerTaskType::GenerateArchitecture,
        None,
        payload,
    );
    if let Err(e) = dispatcher.enqueue(&envelope).await {
        return err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response();
    }

    let _ = service.mark_project_running(&project_id, "n01_architecture", None);
    job_accepted_response(&project_id, job.id).into_response()
}

pub async fn get_architecture(
    Extension(_user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Path(project_id): Path<String>,
) -> impl IntoResponse {
    match service.get_architecture(&project_id) {
        Ok(Some(arch)) => {
            ok_response(serde_json::to_value(&arch).unwrap_or_default()).into_response()
        }
        Ok(None) => ok_response(serde_json::Value::Null).into_response(),
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response(),
    }
}

pub async fn update_architecture(
    Extension(_user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Path(project_id): Path<String>,
    Json(req): Json<NovelArchitectureUpdateRequest>,
) -> impl IntoResponse {
    match service.update_architecture(&project_id, &req) {
        Ok(arch) => ok_response(serde_json::to_value(&arch).unwrap_or_default()).into_response(),
        Err(e) => err_response(StatusCode::NOT_FOUND, &e).into_response(),
    }
}

// =========================================================================
// State docs (23-28)
// =========================================================================

pub async fn get_character_state(
    Extension(_user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Path(project_id): Path<String>,
) -> impl IntoResponse {
    match service.get_character_state(&project_id) {
        Ok(Some(state)) => {
            ok_response(serde_json::to_value(&state).unwrap_or_default()).into_response()
        }
        Ok(None) => ok_response(serde_json::Value::Null).into_response(),
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response(),
    }
}

pub async fn update_character_state(
    Extension(_user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Path(project_id): Path<String>,
    Json(req): Json<NovelCharacterStateUpdateRequest>,
) -> impl IntoResponse {
    match service.update_character_state(&project_id, &req.state_text) {
        Ok(state) => ok_response(serde_json::to_value(&state).unwrap_or_default()).into_response(),
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response(),
    }
}

pub async fn get_global_summary(
    Extension(_user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Path(project_id): Path<String>,
) -> impl IntoResponse {
    match service.get_global_summary(&project_id) {
        Ok(Some(summary)) => {
            ok_response(serde_json::to_value(&summary).unwrap_or_default()).into_response()
        }
        Ok(None) => ok_response(serde_json::Value::Null).into_response(),
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response(),
    }
}

pub async fn update_global_summary(
    Extension(_user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Path(project_id): Path<String>,
    Json(req): Json<NovelGlobalSummaryUpdateRequest>,
) -> impl IntoResponse {
    match service.update_global_summary(&project_id, &req.summary_text) {
        Ok(summary) => {
            ok_response(serde_json::to_value(&summary).unwrap_or_default()).into_response()
        }
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response(),
    }
}

pub async fn get_plot_arcs(
    Extension(_user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Path(project_id): Path<String>,
) -> impl IntoResponse {
    match service.get_plot_arcs(&project_id) {
        Ok(Some(arcs)) => {
            ok_response(serde_json::to_value(&arcs).unwrap_or_default()).into_response()
        }
        Ok(None) => ok_response(serde_json::Value::Null).into_response(),
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response(),
    }
}

pub async fn update_plot_arcs(
    Extension(_user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Path(project_id): Path<String>,
    Json(req): Json<NovelPlotArcsUpdateRequest>,
) -> impl IntoResponse {
    match service.update_plot_arcs(&project_id, &req.plot_arcs_text) {
        Ok(arcs) => ok_response(serde_json::to_value(&arcs).unwrap_or_default()).into_response(),
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response(),
    }
}

// =========================================================================
// Blueprint (29-34)
// =========================================================================

pub async fn generate_blueprint(
    Extension(user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Extension(dispatcher): Extension<Option<NovelWorkerDispatcher>>,
    Path(project_id): Path<String>,
    Json(req): Json<NovelBlueprintGenerateRequest>,
) -> impl IntoResponse {
    let dispatcher = match require_dispatcher(&dispatcher) {
        Ok(d) => d,
        Err(e) => return e.into_response(),
    };

    let payload = serde_json::json!({
        "config_snapshot_id": req.config_snapshot_id,
        "override_user_guidance": req.override_user_guidance,
    });
    let job = match service.create_job(
        &project_id,
        user.id,
        "n02_blueprint",
        "generate_blueprint",
        None,
        payload.clone(),
    ) {
        Ok(j) => j,
        Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response(),
    };

    let envelope = build_envelope(
        &project_id,
        &job,
        "n02_blueprint",
        NovelWorkerTaskType::GenerateBlueprint,
        None,
        payload,
    );
    if let Err(e) = dispatcher.enqueue(&envelope).await {
        return err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response();
    }

    let _ = service.mark_project_running(&project_id, "n02_blueprint", None);
    job_accepted_response(&project_id, job.id).into_response()
}

pub async fn get_blueprint(
    Extension(_user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Path(project_id): Path<String>,
) -> impl IntoResponse {
    match service.get_blueprint(&project_id) {
        Ok(Some(bp)) => ok_response(serde_json::to_value(&bp).unwrap_or_default()).into_response(),
        Ok(None) => ok_response(serde_json::Value::Null).into_response(),
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response(),
    }
}

pub async fn update_blueprint(
    Extension(_user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Path(project_id): Path<String>,
    Json(req): Json<NovelBlueprintUpdateRequest>,
) -> impl IntoResponse {
    match service.update_blueprint(&project_id, &req.raw_text) {
        Ok(bp) => ok_response(serde_json::to_value(&bp).unwrap_or_default()).into_response(),
        Err(e) => err_response(StatusCode::NOT_FOUND, &e).into_response(),
    }
}

pub async fn list_blueprint_chapters(
    Extension(_user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Path(project_id): Path<String>,
) -> impl IntoResponse {
    match service.list_blueprint_chapters(&project_id) {
        Ok(chapters) => {
            ok_response(serde_json::to_value(&chapters).unwrap_or_default()).into_response()
        }
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response(),
    }
}

pub async fn get_blueprint_chapter(
    Extension(_user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Path((project_id, chapter_number)): Path<(String, i32)>,
) -> impl IntoResponse {
    match service.get_blueprint_chapter(&project_id, chapter_number) {
        Ok(Some(ch)) => ok_response(serde_json::to_value(&ch).unwrap_or_default()).into_response(),
        Ok(None) => ok_response(serde_json::Value::Null).into_response(),
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response(),
    }
}

pub async fn update_blueprint_chapter(
    Extension(_user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Path((project_id, chapter_number)): Path<(String, i32)>,
    Json(req): Json<NovelBlueprintChapterUpdateRequest>,
) -> impl IntoResponse {
    match service.update_blueprint_chapter(&project_id, chapter_number, &req) {
        Ok(ch) => ok_response(serde_json::to_value(&ch).unwrap_or_default()).into_response(),
        Err(e) => err_response(StatusCode::NOT_FOUND, &e).into_response(),
    }
}

// =========================================================================
// Chapter prompt (35-37)
// =========================================================================

pub async fn build_chapter_prompt(
    Extension(user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Extension(dispatcher): Extension<Option<NovelWorkerDispatcher>>,
    Path((project_id, chapter_number)): Path<(String, i32)>,
    Json(req): Json<NovelChapterPromptBuildRequest>,
) -> impl IntoResponse {
    let dispatcher = match require_dispatcher(&dispatcher) {
        Ok(d) => d,
        Err(e) => return e.into_response(),
    };

    let payload = serde_json::to_value(&req).unwrap_or_default();
    let job = match service.create_job(
        &project_id,
        user.id,
        "n03_chapter_prompt",
        "build_chapter_prompt",
        Some(chapter_number),
        payload.clone(),
    ) {
        Ok(j) => j,
        Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response(),
    };

    let envelope = build_envelope(
        &project_id,
        &job,
        "n03_chapter_prompt",
        NovelWorkerTaskType::BuildChapterPrompt,
        Some(chapter_number),
        payload,
    );
    if let Err(e) = dispatcher.enqueue(&envelope).await {
        return err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response();
    }

    let _ = service.mark_project_running(&project_id, "n03_chapter_prompt", Some(chapter_number));
    job_accepted_response(&project_id, job.id).into_response()
}

pub async fn get_chapter_prompt(
    Extension(_user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Path((project_id, chapter_number)): Path<(String, i32)>,
) -> impl IntoResponse {
    match service.get_chapter_prompt(&project_id, chapter_number) {
        Ok(Some(prompt)) => {
            ok_response(serde_json::to_value(&prompt).unwrap_or_default()).into_response()
        }
        Ok(None) => ok_response(serde_json::Value::Null).into_response(),
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response(),
    }
}

pub async fn update_chapter_prompt(
    Extension(_user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Path((project_id, chapter_number)): Path<(String, i32)>,
    Json(req): Json<NovelChapterPromptUpdateRequest>,
) -> impl IntoResponse {
    let text = req.edited_prompt_text.as_deref().unwrap_or_default();
    match service.update_chapter_prompt(&project_id, chapter_number, text) {
        Ok(prompt) => {
            ok_response(serde_json::to_value(&prompt).unwrap_or_default()).into_response()
        }
        Err(e) => err_response(StatusCode::NOT_FOUND, &e).into_response(),
    }
}

// =========================================================================
// Chapter flow (38-44)
// =========================================================================

pub async fn generate_chapter_draft(
    Extension(user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Extension(dispatcher): Extension<Option<NovelWorkerDispatcher>>,
    Path((project_id, chapter_number)): Path<(String, i32)>,
    Json(req): Json<NovelChapterDraftGenerateRequest>,
) -> impl IntoResponse {
    let dispatcher = match require_dispatcher(&dispatcher) {
        Ok(d) => d,
        Err(e) => return e.into_response(),
    };

    let payload = serde_json::json!({
        "prompt_id": req.prompt_id,
        "config_snapshot_id": req.config_snapshot_id,
    });
    let job = match service.create_job(
        &project_id,
        user.id,
        "n04_chapter_draft",
        "generate_chapter_draft",
        Some(chapter_number),
        payload.clone(),
    ) {
        Ok(j) => j,
        Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response(),
    };

    let envelope = build_envelope(
        &project_id,
        &job,
        "n04_chapter_draft",
        NovelWorkerTaskType::GenerateChapterDraft,
        Some(chapter_number),
        payload,
    );
    if let Err(e) = dispatcher.enqueue(&envelope).await {
        return err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response();
    }

    let _ = service.mark_project_running(&project_id, "n04_chapter_draft", Some(chapter_number));
    job_accepted_response(&project_id, job.id).into_response()
}

pub async fn list_chapters(
    Extension(_user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Path(project_id): Path<String>,
    Query(query): Query<NovelChapterListQuery>,
) -> impl IntoResponse {
    match service.list_chapters(&project_id, query.status.as_deref()) {
        Ok(chapters) => {
            ok_response(serde_json::to_value(&chapters).unwrap_or_default()).into_response()
        }
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response(),
    }
}

pub async fn get_chapter(
    Extension(_user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Path((project_id, chapter_number)): Path<(String, i32)>,
) -> impl IntoResponse {
    match service.get_chapter(&project_id, chapter_number) {
        Ok(Some(ch)) => ok_response(serde_json::to_value(&ch).unwrap_or_default()).into_response(),
        Ok(None) => ok_response(serde_json::Value::Null).into_response(),
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response(),
    }
}

pub async fn update_chapter(
    Extension(_user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Path((project_id, chapter_number)): Path<(String, i32)>,
    Json(req): Json<NovelChapterUpdateRequest>,
) -> impl IntoResponse {
    match service.update_chapter(&project_id, chapter_number, &req) {
        Ok(ch) => ok_response(serde_json::to_value(&ch).unwrap_or_default()).into_response(),
        Err(e) => err_response(StatusCode::NOT_FOUND, &e).into_response(),
    }
}

pub async fn enrich_chapter(
    Extension(user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Extension(dispatcher): Extension<Option<NovelWorkerDispatcher>>,
    Path((project_id, chapter_number)): Path<(String, i32)>,
    Json(req): Json<NovelChapterEnrichRequest>,
) -> impl IntoResponse {
    let dispatcher = match require_dispatcher(&dispatcher) {
        Ok(d) => d,
        Err(e) => return e.into_response(),
    };

    let payload = serde_json::json!({
        "config_snapshot_id": req.config_snapshot_id,
        "target_words": req.target_words,
    });
    let job = match service.create_job(
        &project_id,
        user.id,
        "n05_enrich",
        "enrich_chapter",
        Some(chapter_number),
        payload.clone(),
    ) {
        Ok(j) => j,
        Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response(),
    };

    let envelope = build_envelope(
        &project_id,
        &job,
        "n05_enrich",
        NovelWorkerTaskType::EnrichChapterText,
        Some(chapter_number),
        payload,
    );
    if let Err(e) = dispatcher.enqueue(&envelope).await {
        return err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response();
    }

    let _ = service.mark_project_running(&project_id, "n05_enrich", Some(chapter_number));
    job_accepted_response(&project_id, job.id).into_response()
}

pub async fn finalize_chapter(
    Extension(user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Extension(dispatcher): Extension<Option<NovelWorkerDispatcher>>,
    Path((project_id, chapter_number)): Path<(String, i32)>,
    Json(req): Json<NovelChapterFinalizeRequest>,
) -> impl IntoResponse {
    let dispatcher = match require_dispatcher(&dispatcher) {
        Ok(d) => d,
        Err(e) => return e.into_response(),
    };

    let payload = serde_json::json!({
        "config_snapshot_id": req.config_snapshot_id,
        "use_current_draft_text": req.use_current_draft_text,
    });
    let job = match service.create_job(
        &project_id,
        user.id,
        "n06_finalize",
        "finalize_chapter",
        Some(chapter_number),
        payload.clone(),
    ) {
        Ok(j) => j,
        Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response(),
    };

    let envelope = build_envelope(
        &project_id,
        &job,
        "n06_finalize",
        NovelWorkerTaskType::FinalizeChapter,
        Some(chapter_number),
        payload,
    );
    if let Err(e) = dispatcher.enqueue(&envelope).await {
        return err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response();
    }

    let _ = service.mark_project_running(&project_id, "n06_finalize", Some(chapter_number));
    job_accepted_response(&project_id, job.id).into_response()
}

pub async fn batch_generate_chapters(
    Extension(user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Extension(dispatcher): Extension<Option<NovelWorkerDispatcher>>,
    Path(project_id): Path<String>,
    Json(req): Json<NovelChapterBatchGenerateRequest>,
) -> impl IntoResponse {
    let dispatcher = match require_dispatcher(&dispatcher) {
        Ok(d) => d,
        Err(e) => return e.into_response(),
    };

    let payload = serde_json::to_value(&req).unwrap_or_default();
    let job = match service.create_job(
        &project_id,
        user.id,
        "n07_batch",
        "batch_generate_chapters",
        None,
        payload.clone(),
    ) {
        Ok(j) => j,
        Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response(),
    };

    let envelope = build_envelope(
        &project_id,
        &job,
        "n07_batch",
        NovelWorkerTaskType::BatchGenerateChapters,
        None,
        payload,
    );
    if let Err(e) = dispatcher.enqueue(&envelope).await {
        return err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response();
    }

    let _ = service.mark_project_running(&project_id, "n07_batch", None);
    job_accepted_response(&project_id, job.id).into_response()
}

// =========================================================================
// Consistency (45-47)
// =========================================================================

pub async fn create_consistency_check(
    Extension(user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Extension(dispatcher): Extension<Option<NovelWorkerDispatcher>>,
    Path((project_id, chapter_number)): Path<(String, i32)>,
    Json(req): Json<NovelConsistencyCheckRequest>,
) -> impl IntoResponse {
    let dispatcher = match require_dispatcher(&dispatcher) {
        Ok(d) => d,
        Err(e) => return e.into_response(),
    };

    let payload = serde_json::json!({
        "config_snapshot_id": req.config_snapshot_id,
        "chapter_number": chapter_number,
    });
    let job = match service.create_job(
        &project_id,
        user.id,
        "n08_consistency",
        "run_consistency_check",
        Some(chapter_number),
        payload.clone(),
    ) {
        Ok(j) => j,
        Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response(),
    };

    let envelope = build_envelope(
        &project_id,
        &job,
        "n08_consistency",
        NovelWorkerTaskType::RunConsistencyCheck,
        Some(chapter_number),
        payload,
    );
    if let Err(e) = dispatcher.enqueue(&envelope).await {
        return err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response();
    }

    let _ = service.mark_project_running(&project_id, "n08_consistency", Some(chapter_number));
    job_accepted_response(&project_id, job.id).into_response()
}

pub async fn list_consistency_checks(
    Extension(_user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Path((project_id, chapter_number)): Path<(String, i32)>,
) -> impl IntoResponse {
    match service.list_consistency_checks(&project_id, chapter_number) {
        Ok(checks) => {
            ok_response(serde_json::to_value(&checks).unwrap_or_default()).into_response()
        }
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response(),
    }
}

pub async fn get_latest_consistency_check(
    Extension(_user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Path((project_id, chapter_number)): Path<(String, i32)>,
) -> impl IntoResponse {
    match service.get_latest_consistency_check(&project_id, chapter_number) {
        Ok(Some(check)) => {
            ok_response(serde_json::to_value(&check).unwrap_or_default()).into_response()
        }
        Ok(None) => ok_response(serde_json::Value::Null).into_response(),
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response(),
    }
}

// =========================================================================
// Knowledge & memory (48-51)
// =========================================================================

pub async fn import_knowledge(
    Extension(user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Extension(dispatcher): Extension<Option<NovelWorkerDispatcher>>,
    Path(project_id): Path<String>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let dispatcher = match require_dispatcher(&dispatcher) {
        Ok(d) => d,
        Err(e) => return e.into_response(),
    };

    let mut file_content: Option<String> = None;
    let mut source_name: Option<String> = None;
    let mut config_snapshot_id: Option<i64> = None;

    loop {
        let field = match multipart.next_field().await {
            Ok(Some(f)) => f,
            Ok(None) => break,
            Err(e) => {
                return err_response(
                    StatusCode::BAD_REQUEST,
                    &format!("multipart read error: {e}"),
                )
                .into_response()
            }
        };

        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "file" => {
                let filename = field.file_name().unwrap_or("unknown.txt").to_string();
                if source_name.is_none() {
                    source_name = Some(filename);
                }
                let bytes = match field.bytes().await {
                    Ok(b) => b,
                    Err(e) => {
                        return err_response(
                            StatusCode::BAD_REQUEST,
                            &format!("read file field: {e}"),
                        )
                        .into_response()
                    }
                };
                file_content = Some(String::from_utf8_lossy(&bytes).to_string());
            }
            "source_name" => {
                let t = match field.text().await {
                    Ok(s) => s,
                    Err(e) => {
                        return err_response(
                            StatusCode::BAD_REQUEST,
                            &format!("read source_name: {e}"),
                        )
                        .into_response()
                    }
                };
                source_name = Some(t);
            }
            "config_snapshot_id" => {
                let t = match field.text().await {
                    Ok(s) => s,
                    Err(e) => {
                        return err_response(
                            StatusCode::BAD_REQUEST,
                            &format!("read config_snapshot_id: {e}"),
                        )
                        .into_response()
                    }
                };
                config_snapshot_id = t.parse().ok();
            }
            _ => {}
        }
    }

    let Some(text) = file_content else {
        return err_response(StatusCode::BAD_REQUEST, "missing multipart field: file")
            .into_response();
    };
    let Some(snapshot_id) = config_snapshot_id else {
        return err_response(
            StatusCode::BAD_REQUEST,
            "missing or invalid config_snapshot_id",
        )
        .into_response();
    };

    if let Err(e) = service.get_project(&project_id, user.id) {
        return err_response(StatusCode::NOT_FOUND, &e).into_response();
    }
    if let Err(e) = service.get_config_snapshot(&project_id, snapshot_id) {
        return err_response(StatusCode::BAD_REQUEST, &e).into_response();
    }

    let mut name = source_name.unwrap_or_else(|| "upload.txt".to_string());
    if name.trim().is_empty() {
        name = "upload.txt".to_string();
    }

    let import = match service.create_knowledge_import(&project_id, name, text) {
        Ok(i) => i,
        Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response(),
    };

    let payload = serde_json::json!({
        "config_snapshot_id": snapshot_id,
        "knowledge_import_id": import.id,
    });
    let job = match service.create_job(
        &project_id,
        user.id,
        "n10_knowledge",
        "import_knowledge",
        None,
        payload.clone(),
    ) {
        Ok(j) => j,
        Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response(),
    };

    let envelope = build_envelope(
        &project_id,
        &job,
        "n10_knowledge",
        NovelWorkerTaskType::ImportKnowledge,
        None,
        payload,
    );
    if let Err(e) = dispatcher.enqueue(&envelope).await {
        return err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response();
    }

    let _ = service.mark_project_running(&project_id, "n10_knowledge", None);
    knowledge_import_accepted_response(&project_id, job.id, import.id).into_response()
}

pub async fn list_knowledge_imports(
    Extension(_user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Path(project_id): Path<String>,
) -> impl IntoResponse {
    match service.list_knowledge_imports(&project_id) {
        Ok(imports) => {
            ok_response(serde_json::to_value(&imports).unwrap_or_default()).into_response()
        }
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response(),
    }
}

pub async fn list_knowledge_chunks(
    Extension(_user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Path(project_id): Path<String>,
    Query(query): Query<NovelKnowledgeChunksQuery>,
) -> impl IntoResponse {
    match service.list_knowledge_chunks(&project_id, query.knowledge_import_id) {
        Ok(chunks) => {
            ok_response(serde_json::to_value(&chunks).unwrap_or_default()).into_response()
        }
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response(),
    }
}

pub async fn clear_memory(
    Extension(user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Extension(dispatcher): Extension<Option<NovelWorkerDispatcher>>,
    Path(project_id): Path<String>,
) -> impl IntoResponse {
    let dispatcher = match require_dispatcher(&dispatcher) {
        Ok(d) => d,
        Err(e) => return e.into_response(),
    };

    let payload = serde_json::json!({ "project_id": project_id });
    let job = match service.create_job(
        &project_id,
        user.id,
        "n09_memory",
        "clear_memory",
        None,
        payload.clone(),
    ) {
        Ok(j) => j,
        Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response(),
    };

    let envelope = build_envelope(
        &project_id,
        &job,
        "n09_memory",
        NovelWorkerTaskType::ClearMemory,
        None,
        payload,
    );
    if let Err(e) = dispatcher.enqueue(&envelope).await {
        return err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response();
    }

    let _ = service.mark_project_running(&project_id, "n09_memory", None);
    job_accepted_response(&project_id, job.id).into_response()
}

// =========================================================================
// Jobs & events (52-55)
// =========================================================================

pub async fn list_jobs(
    Extension(_user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Path(project_id): Path<String>,
) -> impl IntoResponse {
    match service.list_jobs(&project_id) {
        Ok(jobs) => ok_response(serde_json::to_value(&jobs).unwrap_or_default()).into_response(),
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response(),
    }
}

pub async fn get_job(
    Extension(_user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Path((project_id, job_id)): Path<(String, i64)>,
) -> impl IntoResponse {
    let job = match service.get_job(&project_id, job_id) {
        Ok(Some(j)) => j,
        Ok(None) => return err_response(StatusCode::NOT_FOUND, "job not found").into_response(),
        Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response(),
    };

    let stage_runs = match service.list_stage_runs(job.id) {
        Ok(runs) => runs,
        Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response(),
    };

    let mut job_json = serde_json::to_value(&job).unwrap_or_default();
    if let Some(obj) = job_json.as_object_mut() {
        obj.insert(
            "stage_runs".to_string(),
            serde_json::to_value(&stage_runs).unwrap_or_default(),
        );
    }
    ok_response(job_json).into_response()
}

pub async fn retry_job(
    Extension(user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Extension(dispatcher): Extension<Option<NovelWorkerDispatcher>>,
    Path((project_id, job_id)): Path<(String, i64)>,
) -> impl IntoResponse {
    let dispatcher = match require_dispatcher(&dispatcher) {
        Ok(d) => d,
        Err(e) => return e.into_response(),
    };

    let original_job = match service.get_job(&project_id, job_id) {
        Ok(Some(j)) => j,
        Ok(None) => return err_response(StatusCode::NOT_FOUND, "job not found").into_response(),
        Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response(),
    };

    let retryable = ["failed", "partial_failed", "cancelled"];
    if !retryable.contains(&original_job.status.as_str()) {
        return err_response(
            StatusCode::CONFLICT,
            &format!(
                "job status '{}' is not retryable (must be failed/partial_failed/cancelled)",
                original_job.status
            ),
        )
        .into_response();
    }

    let new_job = match service.create_job(
        &project_id,
        user.id,
        &original_job.stage_code,
        &original_job.task_type,
        original_job.chapter_number,
        original_job.request_payload.clone(),
    ) {
        Ok(j) => j,
        Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response(),
    };

    let task_type = match original_job.task_type.as_str() {
        "generate_architecture" => NovelWorkerTaskType::GenerateArchitecture,
        "generate_blueprint" => NovelWorkerTaskType::GenerateBlueprint,
        "build_chapter_prompt" => NovelWorkerTaskType::BuildChapterPrompt,
        "generate_chapter_draft" => NovelWorkerTaskType::GenerateChapterDraft,
        "enrich_chapter" => NovelWorkerTaskType::EnrichChapterText,
        "finalize_chapter" => NovelWorkerTaskType::FinalizeChapter,
        "run_consistency_check" => NovelWorkerTaskType::RunConsistencyCheck,
        "clear_memory" => NovelWorkerTaskType::ClearMemory,
        "batch_generate_chapters" => NovelWorkerTaskType::BatchGenerateChapters,
        "import_knowledge" => NovelWorkerTaskType::ImportKnowledge,
        other => {
            let msg = format!("unknown task_type for retry: {other}");
            return err_response(StatusCode::BAD_REQUEST, &msg).into_response();
        }
    };

    let envelope = build_envelope(
        &project_id,
        &new_job,
        &original_job.stage_code,
        task_type,
        original_job.chapter_number,
        original_job.request_payload.clone(),
    );
    if let Err(e) = dispatcher.enqueue(&envelope).await {
        return err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response();
    }

    job_accepted_response(&project_id, new_job.id).into_response()
}

pub async fn list_events(
    Extension(_user): Extension<User>,
    Extension(service): Extension<NovelService>,
    Path(project_id): Path<String>,
    Query(query): Query<NovelEventsQuery>,
) -> impl IntoResponse {
    let limit = query.limit.unwrap_or(50);
    match service.list_events(&project_id, query.after_sequence, limit) {
        Ok(events) => {
            let next_seq = events.last().map(|e| e.sequence).unwrap_or(0);
            ok_response(serde_json::json!({
                "project_id": project_id,
                "after_sequence": query.after_sequence,
                "next_sequence": next_seq,
                "events": serde_json::to_value(&events).unwrap_or_default(),
            }))
            .into_response()
        }
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response(),
    }
}

// =========================================================================
// Internal callback (worker → API, no user auth)
// =========================================================================

pub async fn ingest_worker_callback(
    Extension(service): Extension<NovelService>,
    Json(event): Json<crate::dto::novel_dto::NovelWorkerCallbackEvent>,
) -> impl IntoResponse {
    tracing::info!(
        project_id = %event.project_id,
        job_id = event.job_id,
        event_type = %event.event_type,
        "Novel worker callback received"
    );

    match event.event_type.as_str() {
        "job_started" => {
            if let Err(e) = service.update_job_status(event.job_id, "running", None, None) {
                tracing::error!("Failed to update job to running: {e}");
                return err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response();
            }
            if let Err(e) = service.mark_project_running(
                &event.project_id,
                event.stage_code.as_deref().unwrap_or("unknown"),
                event.chapter_number,
            ) {
                tracing::error!("Failed to mark project running: {e}");
            }
        }
        "job_completed" => {
            let result = if event.result_payload.is_object()
                && event
                    .result_payload
                    .as_object()
                    .is_some_and(|m| !m.is_empty())
            {
                Some(event.result_payload.clone())
            } else {
                None
            };
            match service.update_job_status(event.job_id, "completed", result, None) {
                Ok(job) => {
                    if let Err(e) = service.sync_project_from_job(&job) {
                        tracing::error!("Failed to sync project after job_completed: {e}");
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to update job to completed: {e}");
                    return err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response();
                }
            }
        }
        "job_failed" => {
            let err_payload = if event.error_payload.is_object()
                && event
                    .error_payload
                    .as_object()
                    .is_some_and(|m| !m.is_empty())
            {
                Some(event.error_payload.clone())
            } else {
                Some(
                    serde_json::json!({"error": event.error_message.as_deref().unwrap_or("unknown")}),
                )
            };
            match service.update_job_status(event.job_id, "failed", None, err_payload) {
                Ok(job) => {
                    if let Err(e) = service.sync_project_from_job(&job) {
                        tracing::error!("Failed to sync project after job_failed: {e}");
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to update job to failed: {e}");
                    return err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response();
                }
            }
        }
        "job_partial_failed" => {
            let result = if event.result_payload.is_object()
                && event
                    .result_payload
                    .as_object()
                    .is_some_and(|m| !m.is_empty())
            {
                Some(event.result_payload.clone())
            } else {
                None
            };
            let err_payload = if event.error_payload.is_object()
                && event
                    .error_payload
                    .as_object()
                    .is_some_and(|m| !m.is_empty())
            {
                Some(event.error_payload.clone())
            } else {
                None
            };
            match service.update_job_status(event.job_id, "partial_failed", result, err_payload) {
                Ok(job) => {
                    if let Err(e) = service.sync_project_from_job(&job) {
                        tracing::error!("Failed to sync project after job_partial_failed: {e}");
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to update job to partial_failed: {e}");
                    return err_response(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response();
                }
            }
        }
        "stage_run_started" | "stage_run_completed" | "stage_run_failed" => {
            let sr_status = match event.event_type.as_str() {
                "stage_run_started" => "running",
                "stage_run_completed" => "completed",
                "stage_run_failed" => "failed",
                _ => "unknown",
            };
            let output = if event.result_payload.is_object()
                && event
                    .result_payload
                    .as_object()
                    .is_some_and(|m| !m.is_empty())
            {
                Some(event.result_payload.clone())
            } else {
                None
            };
            if let Err(e) = service.upsert_stage_run(
                &event.project_id,
                event.job_id,
                event.chapter_number.unwrap_or(0),
                event.stage_code.as_deref().unwrap_or("unknown"),
                sr_status,
                event.input_hash.as_deref().unwrap_or(""),
                event.payload.clone(),
                output,
                event.error_message.as_deref(),
                event.attempt_no.unwrap_or(1),
            ) {
                tracing::error!("Failed to upsert stage run: {e}");
            }
        }
        _ => {}
    }

    if let Err(e) = service.record_stage_event(
        &event.project_id,
        Some(event.job_id),
        event.stage_run_id,
        event.chapter_number,
        &event.event_type,
        event.stage_code.as_deref(),
        event.payload.clone(),
    ) {
        tracing::error!("Failed to record stage event: {e}");
    }

    ok_response(serde_json::json!({
        "accepted": true,
        "project_id": event.project_id,
        "job_id": event.job_id,
        "event_type": event.event_type,
    }))
    .into_response()
}
