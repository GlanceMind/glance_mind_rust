//! Novel engine HTTP handlers (phase 1 stubs). Service layer wiring comes later.
use axum::{
    extract::{Multipart, Path, Query},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use glance_mind_db::entity::user::User;

use crate::dto::novel_dto::*;

fn not_implemented(user: &User, endpoint: &'static str) -> impl IntoResponse {
    let _ = user.id;
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({
            "code": 501,
            "msg": "not implemented yet",
            "msg_cn": "尚未实现",
            "endpoint": endpoint,
        })),
    )
}

// --- Project & Config (1-9) ---

pub async fn create_project(
    Extension(user): Extension<User>,
    Json(_req): Json<NovelProjectCreateRequest>,
) -> impl IntoResponse {
    not_implemented(&user, "POST /novel/projects")
}

pub async fn list_projects(
    Extension(user): Extension<User>,
    Query(_query): Query<NovelProjectListQuery>,
) -> impl IntoResponse {
    not_implemented(&user, "GET /novel/projects")
}

pub async fn get_project(
    Extension(user): Extension<User>,
    Path(_project_id): Path<String>,
) -> impl IntoResponse {
    not_implemented(&user, "GET /novel/projects/:project_id")
}

pub async fn delete_project(
    Extension(user): Extension<User>,
    Path(_project_id): Path<String>,
) -> impl IntoResponse {
    not_implemented(&user, "DELETE /novel/projects/:project_id")
}

pub async fn cancel_project(
    Extension(user): Extension<User>,
    Path(_project_id): Path<String>,
) -> impl IntoResponse {
    not_implemented(&user, "POST /novel/projects/:project_id/cancel")
}

pub async fn update_project(
    Extension(user): Extension<User>,
    Path(_project_id): Path<String>,
    Json(_req): Json<NovelProjectUpdateRequest>,
) -> impl IntoResponse {
    not_implemented(&user, "PATCH /novel/projects/:project_id")
}

pub async fn list_config_snapshots(
    Extension(user): Extension<User>,
    Path(_project_id): Path<String>,
) -> impl IntoResponse {
    not_implemented(&user, "GET /novel/projects/:project_id/config-snapshots")
}

pub async fn create_config_snapshot(
    Extension(user): Extension<User>,
    Path(_project_id): Path<String>,
    Json(_req): Json<NovelConfigSnapshotCreateRequest>,
) -> impl IntoResponse {
    not_implemented(&user, "POST /novel/projects/:project_id/config-snapshots")
}

pub async fn activate_config_snapshot(
    Extension(user): Extension<User>,
    Path((_project_id, _snapshot_id)): Path<(String, i64)>,
) -> impl IntoResponse {
    not_implemented(&user, "POST /novel/projects/:project_id/config-snapshots/:snapshot_id/activate")
}

// --- Profiles (10-19) ---

pub async fn list_llm_profiles(Extension(user): Extension<User>) -> impl IntoResponse {
    not_implemented(&user, "GET /novel/llm-profiles")
}

pub async fn create_llm_profile(
    Extension(user): Extension<User>,
    Json(_req): Json<NovelLlmProfileCreateRequest>,
) -> impl IntoResponse {
    not_implemented(&user, "POST /novel/llm-profiles")
}

pub async fn update_llm_profile(
    Extension(user): Extension<User>,
    Path(_id): Path<i64>,
    Json(_req): Json<NovelLlmProfileUpdateRequest>,
) -> impl IntoResponse {
    not_implemented(&user, "PUT /novel/llm-profiles/:id")
}

pub async fn delete_llm_profile(
    Extension(user): Extension<User>,
    Path(_id): Path<i64>,
) -> impl IntoResponse {
    not_implemented(&user, "DELETE /novel/llm-profiles/:id")
}

pub async fn test_llm_profile(
    Extension(user): Extension<User>,
    Path(_id): Path<i64>,
    Json(_req): Json<NovelLlmProfileTestRequest>,
) -> impl IntoResponse {
    not_implemented(&user, "POST /novel/llm-profiles/:id/test")
}

pub async fn list_embedding_profiles(Extension(user): Extension<User>) -> impl IntoResponse {
    not_implemented(&user, "GET /novel/embedding-profiles")
}

pub async fn create_embedding_profile(
    Extension(user): Extension<User>,
    Json(_req): Json<NovelEmbeddingProfileCreateRequest>,
) -> impl IntoResponse {
    not_implemented(&user, "POST /novel/embedding-profiles")
}

pub async fn update_embedding_profile(
    Extension(user): Extension<User>,
    Path(_id): Path<i64>,
    Json(_req): Json<NovelEmbeddingProfileUpdateRequest>,
) -> impl IntoResponse {
    not_implemented(&user, "PUT /novel/embedding-profiles/:id")
}

pub async fn delete_embedding_profile(
    Extension(user): Extension<User>,
    Path(_id): Path<i64>,
) -> impl IntoResponse {
    not_implemented(&user, "DELETE /novel/embedding-profiles/:id")
}

pub async fn test_embedding_profile(
    Extension(user): Extension<User>,
    Path(_id): Path<i64>,
    Json(_req): Json<NovelEmbeddingProfileTestRequest>,
) -> impl IntoResponse {
    not_implemented(&user, "POST /novel/embedding-profiles/:id/test")
}

// --- Architecture (20-22) ---

pub async fn generate_architecture(
    Extension(user): Extension<User>,
    Path(_project_id): Path<String>,
    Json(_req): Json<NovelArchitectureGenerateRequest>,
) -> impl IntoResponse {
    not_implemented(&user, "POST /novel/projects/:project_id/architecture/generate")
}

pub async fn get_architecture(
    Extension(user): Extension<User>,
    Path(_project_id): Path<String>,
) -> impl IntoResponse {
    not_implemented(&user, "GET /novel/projects/:project_id/architecture")
}

pub async fn update_architecture(
    Extension(user): Extension<User>,
    Path(_project_id): Path<String>,
    Json(_req): Json<NovelArchitectureUpdateRequest>,
) -> impl IntoResponse {
    not_implemented(&user, "PUT /novel/projects/:project_id/architecture")
}

// --- State docs (23-28) ---

pub async fn get_character_state(
    Extension(user): Extension<User>,
    Path(_project_id): Path<String>,
) -> impl IntoResponse {
    not_implemented(&user, "GET /novel/projects/:project_id/character-state")
}

pub async fn update_character_state(
    Extension(user): Extension<User>,
    Path(_project_id): Path<String>,
    Json(_req): Json<NovelCharacterStateUpdateRequest>,
) -> impl IntoResponse {
    not_implemented(&user, "PUT /novel/projects/:project_id/character-state")
}

pub async fn get_global_summary(
    Extension(user): Extension<User>,
    Path(_project_id): Path<String>,
) -> impl IntoResponse {
    not_implemented(&user, "GET /novel/projects/:project_id/global-summary")
}

pub async fn update_global_summary(
    Extension(user): Extension<User>,
    Path(_project_id): Path<String>,
    Json(_req): Json<NovelGlobalSummaryUpdateRequest>,
) -> impl IntoResponse {
    not_implemented(&user, "PUT /novel/projects/:project_id/global-summary")
}

pub async fn get_plot_arcs(
    Extension(user): Extension<User>,
    Path(_project_id): Path<String>,
) -> impl IntoResponse {
    not_implemented(&user, "GET /novel/projects/:project_id/plot-arcs")
}

pub async fn update_plot_arcs(
    Extension(user): Extension<User>,
    Path(_project_id): Path<String>,
    Json(_req): Json<NovelPlotArcsUpdateRequest>,
) -> impl IntoResponse {
    not_implemented(&user, "PUT /novel/projects/:project_id/plot-arcs")
}

// --- Blueprint (29-34) ---

pub async fn generate_blueprint(
    Extension(user): Extension<User>,
    Path(_project_id): Path<String>,
    Json(_req): Json<NovelBlueprintGenerateRequest>,
) -> impl IntoResponse {
    not_implemented(&user, "POST /novel/projects/:project_id/blueprint/generate")
}

pub async fn get_blueprint(
    Extension(user): Extension<User>,
    Path(_project_id): Path<String>,
) -> impl IntoResponse {
    not_implemented(&user, "GET /novel/projects/:project_id/blueprint")
}

pub async fn update_blueprint(
    Extension(user): Extension<User>,
    Path(_project_id): Path<String>,
    Json(_req): Json<NovelBlueprintUpdateRequest>,
) -> impl IntoResponse {
    not_implemented(&user, "PUT /novel/projects/:project_id/blueprint")
}

pub async fn list_blueprint_chapters(
    Extension(user): Extension<User>,
    Path(_project_id): Path<String>,
) -> impl IntoResponse {
    not_implemented(&user, "GET /novel/projects/:project_id/blueprint/chapters")
}

pub async fn get_blueprint_chapter(
    Extension(user): Extension<User>,
    Path((_project_id, _chapter_number)): Path<(String, i32)>,
) -> impl IntoResponse {
    not_implemented(&user, "GET /novel/projects/:project_id/blueprint/chapters/:chapter_number")
}

pub async fn update_blueprint_chapter(
    Extension(user): Extension<User>,
    Path((_project_id, _chapter_number)): Path<(String, i32)>,
    Json(_req): Json<NovelBlueprintChapterUpdateRequest>,
) -> impl IntoResponse {
    not_implemented(&user, "PUT /novel/projects/:project_id/blueprint/chapters/:chapter_number")
}

// --- Chapter prompt (35-37) ---

pub async fn build_chapter_prompt(
    Extension(user): Extension<User>,
    Path((_project_id, _chapter_number)): Path<(String, i32)>,
    Json(_req): Json<NovelChapterPromptBuildRequest>,
) -> impl IntoResponse {
    not_implemented(&user, "POST /novel/projects/:project_id/chapters/:chapter_number/prompt/build")
}

pub async fn get_chapter_prompt(
    Extension(user): Extension<User>,
    Path((_project_id, _chapter_number)): Path<(String, i32)>,
) -> impl IntoResponse {
    not_implemented(&user, "GET /novel/projects/:project_id/chapters/:chapter_number/prompt")
}

pub async fn update_chapter_prompt(
    Extension(user): Extension<User>,
    Path((_project_id, _chapter_number)): Path<(String, i32)>,
    Json(_req): Json<NovelChapterPromptUpdateRequest>,
) -> impl IntoResponse {
    not_implemented(&user, "PUT /novel/projects/:project_id/chapters/:chapter_number/prompt")
}

// --- Chapter flow (38-44) ---

pub async fn generate_chapter_draft(
    Extension(user): Extension<User>,
    Path((_project_id, _chapter_number)): Path<(String, i32)>,
    Json(_req): Json<NovelChapterDraftGenerateRequest>,
) -> impl IntoResponse {
    not_implemented(&user, "POST /novel/projects/:project_id/chapters/:chapter_number/draft/generate")
}

pub async fn list_chapters(
    Extension(user): Extension<User>,
    Path(_project_id): Path<String>,
    Query(_query): Query<NovelChapterListQuery>,
) -> impl IntoResponse {
    not_implemented(&user, "GET /novel/projects/:project_id/chapters")
}

pub async fn get_chapter(
    Extension(user): Extension<User>,
    Path((_project_id, _chapter_number)): Path<(String, i32)>,
) -> impl IntoResponse {
    not_implemented(&user, "GET /novel/projects/:project_id/chapters/:chapter_number")
}

pub async fn update_chapter(
    Extension(user): Extension<User>,
    Path((_project_id, _chapter_number)): Path<(String, i32)>,
    Json(_req): Json<NovelChapterUpdateRequest>,
) -> impl IntoResponse {
    not_implemented(&user, "PUT /novel/projects/:project_id/chapters/:chapter_number")
}

pub async fn enrich_chapter(
    Extension(user): Extension<User>,
    Path((_project_id, _chapter_number)): Path<(String, i32)>,
    Json(_req): Json<NovelChapterEnrichRequest>,
) -> impl IntoResponse {
    not_implemented(&user, "POST /novel/projects/:project_id/chapters/:chapter_number/enrich")
}

pub async fn finalize_chapter(
    Extension(user): Extension<User>,
    Path((_project_id, _chapter_number)): Path<(String, i32)>,
    Json(_req): Json<NovelChapterFinalizeRequest>,
) -> impl IntoResponse {
    not_implemented(&user, "POST /novel/projects/:project_id/chapters/:chapter_number/finalize")
}

pub async fn batch_generate_chapters(
    Extension(user): Extension<User>,
    Path(_project_id): Path<String>,
    Json(_req): Json<NovelChapterBatchGenerateRequest>,
) -> impl IntoResponse {
    not_implemented(&user, "POST /novel/projects/:project_id/chapters/batch-generate")
}

// --- Consistency (45-47) ---

pub async fn create_consistency_check(
    Extension(user): Extension<User>,
    Path((_project_id, _chapter_number)): Path<(String, i32)>,
    Json(_req): Json<NovelConsistencyCheckRequest>,
) -> impl IntoResponse {
    not_implemented(&user, "POST /novel/projects/:project_id/chapters/:chapter_number/consistency-checks")
}

pub async fn list_consistency_checks(
    Extension(user): Extension<User>,
    Path((_project_id, _chapter_number)): Path<(String, i32)>,
) -> impl IntoResponse {
    not_implemented(&user, "GET /novel/projects/:project_id/chapters/:chapter_number/consistency-checks")
}

pub async fn get_latest_consistency_check(
    Extension(user): Extension<User>,
    Path((_project_id, _chapter_number)): Path<(String, i32)>,
) -> impl IntoResponse {
    not_implemented(&user, "GET /novel/projects/:project_id/chapters/:chapter_number/consistency-checks/latest")
}

// --- Knowledge & memory (48-51) ---

pub async fn import_knowledge(
    Extension(user): Extension<User>,
    Path(_project_id): Path<String>,
    _multipart: Multipart,
) -> impl IntoResponse {
    not_implemented(&user, "POST /novel/projects/:project_id/knowledge/imports")
}

pub async fn list_knowledge_imports(
    Extension(user): Extension<User>,
    Path(_project_id): Path<String>,
) -> impl IntoResponse {
    not_implemented(&user, "GET /novel/projects/:project_id/knowledge/imports")
}

pub async fn list_knowledge_chunks(
    Extension(user): Extension<User>,
    Path(_project_id): Path<String>,
    Query(_query): Query<NovelKnowledgeChunksQuery>,
) -> impl IntoResponse {
    not_implemented(&user, "GET /novel/projects/:project_id/knowledge/chunks")
}

pub async fn clear_memory(
    Extension(user): Extension<User>,
    Path(_project_id): Path<String>,
) -> impl IntoResponse {
    not_implemented(&user, "POST /novel/projects/:project_id/memory/clear")
}

// --- Jobs & events (52-55) ---

pub async fn list_jobs(
    Extension(user): Extension<User>,
    Path(_project_id): Path<String>,
) -> impl IntoResponse {
    not_implemented(&user, "GET /novel/projects/:project_id/jobs")
}

pub async fn get_job(
    Extension(user): Extension<User>,
    Path((_project_id, _job_id)): Path<(String, i64)>,
) -> impl IntoResponse {
    not_implemented(&user, "GET /novel/projects/:project_id/jobs/:job_id")
}

pub async fn retry_job(
    Extension(user): Extension<User>,
    Path((_project_id, _job_id)): Path<(String, i64)>,
) -> impl IntoResponse {
    not_implemented(&user, "POST /novel/projects/:project_id/jobs/:job_id/retry")
}

pub async fn list_events(
    Extension(user): Extension<User>,
    Path(_project_id): Path<String>,
    Query(_query): Query<NovelEventsQuery>,
) -> impl IntoResponse {
    not_implemented(&user, "GET /novel/projects/:project_id/events")
}
