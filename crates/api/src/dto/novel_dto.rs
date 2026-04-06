//! Novel engine integration DTOs (Phase 1 control-plane API).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

fn empty_json_object() -> Value {
    Value::Object(Default::default())
}

// =============================================================================
// 1. Project DTOs
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovelProjectCreateRequest {
    pub title: String,
    pub topic: String,
    pub genre: String,
    pub description: String,
    pub num_chapters: i32,
    pub target_words_per_chapter: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_user_guidance: Option<String>,
    pub config_snapshot: NovelConfigSnapshotCreateRequest,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NovelProjectUpdateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub genre: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_chapters: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_words_per_chapter: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_user_guidance: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct NovelProjectListQuery {
    pub status: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovelProjectResponse {
    pub project_id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub genre: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_chapters: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_words_per_chapter: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_stage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending_stage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_chapter_number: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress_percent: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interaction_version: Option<i64>,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovelProjectDetailResponse {
    #[serde(flatten)]
    pub project: NovelProjectResponse,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_user_guidance: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_config_snapshot: Option<NovelConfigSnapshotResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_refs: Option<Value>,
}

// =============================================================================
// 2. Config Snapshot DTOs
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovelConfigSnapshotCreateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub architecture_llm_profile_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chapter_outline_llm_profile_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_draft_llm_profile_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_chapter_llm_profile_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consistency_review_llm_profile_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_profile_id: Option<i64>,
    #[serde(default = "empty_json_object")]
    pub proxy_setting: Value,
    #[serde(default = "empty_json_object")]
    pub webdav_config: Value,
    #[serde(default = "empty_json_object")]
    pub other_params: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovelConfigSnapshotResponse {
    pub id: i64,
    pub project_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub architecture_llm_profile_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chapter_outline_llm_profile_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_draft_llm_profile_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_chapter_llm_profile_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consistency_review_llm_profile_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_profile_id: Option<i64>,
    #[serde(default = "empty_json_object")]
    pub proxy_setting: Value,
    #[serde(default = "empty_json_object")]
    pub webdav_config: Value,
    #[serde(default = "empty_json_object")]
    pub other_params: Value,
    pub is_current: bool,
    pub created_at: DateTime<Utc>,
}

// =============================================================================
// 3. LLM Profile DTOs
// =============================================================================

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NovelLlmProfileCreateRequest {
    pub name: String,
    pub interface_format: String,
    pub base_url: String,
    pub api_key: String,
    pub model_name: String,
    pub temperature: f64,
    pub max_tokens: i32,
    pub timeout_seconds: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NovelLlmProfileUpdateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interface_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_seconds: Option<i32>,
}

/// Body for `POST /novel/llm-profiles/:id/test` (provider-specific fields allowed).
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct NovelLlmProfileTestRequest {
    #[serde(flatten)]
    pub extra: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovelLlmProfileResponse {
    pub id: i64,
    pub user_id: i32,
    pub name: String,
    pub interface_format: String,
    pub base_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    pub model_name: String,
    pub temperature: f64,
    pub max_tokens: i32,
    pub timeout_seconds: i32,
    pub is_default: bool,
    pub is_active: bool,
    #[serde(default = "empty_json_object")]
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<DateTime<Utc>>,
}

// =============================================================================
// 4. Embedding Profile DTOs
// =============================================================================

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NovelEmbeddingProfileCreateRequest {
    pub name: String,
    pub interface_format: String,
    pub base_url: String,
    pub api_key: String,
    pub model_name: String,
    pub retrieval_k: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NovelEmbeddingProfileUpdateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interface_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retrieval_k: Option<i32>,
}

/// Body for `POST /novel/embedding-profiles/:id/test`.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct NovelEmbeddingProfileTestRequest {
    #[serde(flatten)]
    pub extra: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovelEmbeddingProfileResponse {
    pub id: i64,
    pub user_id: i32,
    pub name: String,
    pub interface_format: String,
    pub base_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    pub model_name: String,
    pub retrieval_k: i32,
    pub is_default: bool,
    pub is_active: bool,
    #[serde(default = "empty_json_object")]
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<DateTime<Utc>>,
}

// =============================================================================
// 5. Architecture DTOs
// =============================================================================

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NovelArchitectureGenerateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub override_user_guidance: Option<String>,
    pub config_snapshot_id: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NovelArchitectureUpdateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub core_seed_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub character_dynamics_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub world_building_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plot_architecture_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovelArchitectureResponse {
    pub id: i64,
    pub project_id: String,
    pub core_seed_text: String,
    pub character_dynamics_text: String,
    pub world_building_text: String,
    pub plot_architecture_text: String,
    pub full_text: String,
    pub version_no: i32,
    pub is_current: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_stage_run_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// =============================================================================
// 6. State DTOs
// =============================================================================

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NovelCharacterStateUpdateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chapter_number: Option<i32>,
    pub state_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovelCharacterStateResponse {
    pub id: i64,
    pub project_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chapter_number: Option<i32>,
    pub state_text: String,
    pub version_no: i32,
    pub is_current: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_stage_run_id: Option<i64>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NovelGlobalSummaryUpdateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chapter_number: Option<i32>,
    pub summary_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovelGlobalSummaryResponse {
    pub id: i64,
    pub project_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chapter_number: Option<i32>,
    pub summary_text: String,
    pub version_no: i32,
    pub is_current: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_stage_run_id: Option<i64>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NovelPlotArcsUpdateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chapter_number: Option<i32>,
    pub plot_arcs_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovelPlotArcsResponse {
    pub id: i64,
    pub project_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chapter_number: Option<i32>,
    pub plot_arcs_text: String,
    pub version_no: i32,
    pub is_current: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_stage_run_id: Option<i64>,
    pub created_at: DateTime<Utc>,
}

// =============================================================================
// 7. Blueprint DTOs
// =============================================================================

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NovelBlueprintGenerateRequest {
    pub config_snapshot_id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub override_user_guidance: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NovelBlueprintUpdateRequest {
    pub raw_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_size: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovelBlueprintResponse {
    pub id: i64,
    pub project_id: String,
    pub raw_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_size: Option<i32>,
    pub generated_chapter_count: i32,
    pub version_no: i32,
    pub is_current: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_stage_run_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovelBlueprintChapterResponse {
    pub id: i64,
    pub blueprint_id: i64,
    pub project_id: String,
    pub chapter_number: i32,
    pub chapter_title: String,
    pub chapter_role: String,
    pub chapter_purpose: String,
    pub suspense_level: String,
    pub foreshadowing: String,
    pub plot_twist_level: String,
    pub chapter_summary: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NovelBlueprintChapterUpdateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chapter_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chapter_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chapter_purpose: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suspense_level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foreshadowing: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plot_twist_level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chapter_summary: Option<String>,
}

// =============================================================================
// 8. Chapter Prompt DTOs
// =============================================================================

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NovelChapterPromptBuildRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_guidance: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub characters_involved: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_items: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scene_location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_constraint: Option<String>,
    pub config_snapshot_id: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NovelChapterPromptUpdateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_guidance: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub characters_involved: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_items: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scene_location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_constraint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short_summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_excerpt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filtered_context: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edited_prompt_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovelChapterPromptResponse {
    pub id: i64,
    pub project_id: String,
    pub chapter_number: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blueprint_chapter_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_guidance: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub characters_involved: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_items: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scene_location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_constraint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short_summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_excerpt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filtered_context: Option<String>,
    pub prompt_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edited_prompt_text: Option<String>,
    pub is_current: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_stage_run_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// =============================================================================
// 9. Chapter DTOs
// =============================================================================

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NovelChapterDraftGenerateRequest {
    pub prompt_id: Option<i64>,
    pub config_snapshot_id: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NovelChapterUpdateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NovelChapterEnrichRequest {
    pub config_snapshot_id: i64,
    pub target_words: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NovelChapterFinalizeRequest {
    pub config_snapshot_id: i64,
    pub use_current_draft_text: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NovelChapterBatchGenerateRequest {
    pub start_chapter: i32,
    pub end_chapter: i32,
    pub target_words: i32,
    pub minimum_words: i32,
    pub auto_enrich: bool,
    pub config_snapshot_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovelChapterResponse {
    pub id: i64,
    pub project_id: String,
    pub chapter_number: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blueprint_chapter_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_id: Option<i64>,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_guidance: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub characters_involved: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_items: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scene_location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_constraint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_text: Option<String>,
    pub status: String,
    pub is_enriched: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_words: Option<i32>,
    pub draft_word_count: i32,
    pub final_word_count: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consistency_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_stage_run_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finalized_at: Option<DateTime<Utc>>,
}

// =============================================================================
// 10. Consistency DTOs
// =============================================================================

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NovelConsistencyCheckRequest {
    pub config_snapshot_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovelConsistencyCheckResponse {
    pub id: i64,
    pub project_id: String,
    pub chapter_number: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub novel_setting_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub character_state_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub global_summary_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plot_arcs_text: Option<String>,
    pub chapter_text: String,
    pub result_text: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_stage_run_id: Option<i64>,
    pub created_at: DateTime<Utc>,
}

// =============================================================================
// 11. Knowledge DTOs
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovelKnowledgeImportResponse {
    pub id: i64,
    pub project_id: String,
    pub source_name: String,
    pub source_type: String,
    pub original_text: String,
    pub segment_count: i32,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_stage_run_id: Option<i64>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovelKnowledgeChunkResponse {
    pub id: i64,
    pub project_id: String,
    pub knowledge_import_id: i64,
    pub chunk_index: i32,
    pub content: String,
    #[serde(default = "empty_json_object")]
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
}

// =============================================================================
// 12. Job DTOs
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovelJobResponse {
    pub id: i64,
    pub project_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chapter_number: Option<i32>,
    pub stage_code: String,
    pub task_type: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
    #[serde(default = "empty_json_object")]
    pub request_payload: Value,
    #[serde(default = "empty_json_object")]
    pub result_payload: Value,
    #[serde(default = "empty_json_object")]
    pub error_payload: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovelJobDetailResponse {
    #[serde(flatten)]
    pub job: NovelJobResponse,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage_runs: Option<Vec<NovelStageRunResponse>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovelStageRunResponse {
    pub id: i64,
    pub project_id: String,
    pub job_id: i64,
    pub chapter_number: i32,
    pub stage_code: String,
    pub status: String,
    pub input_hash: String,
    #[serde(default = "empty_json_object")]
    pub input_payload: Value,
    #[serde(default = "empty_json_object")]
    pub output_payload: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    pub attempt_no: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// =============================================================================
// 13. Event DTOs
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovelEventResponse {
    pub id: i64,
    pub project_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage_run_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chapter_number: Option<i32>,
    pub sequence: i64,
    pub event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage_code: Option<String>,
    #[serde(default = "empty_json_object")]
    pub payload: Value,
    pub occurred_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovelEventsListResponse {
    pub project_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after_sequence: Option<i64>,
    pub next_sequence: i64,
    pub events: Vec<NovelEventResponse>,
}

// =============================================================================
// 14. Worker Task Envelope
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NovelWorkerTaskType {
    GenerateArchitecture,
    GenerateBlueprint,
    BuildChapterPrompt,
    GenerateChapterDraft,
    EnrichChapterText,
    FinalizeChapter,
    RunConsistencyCheck,
    ImportKnowledge,
    ClearMemory,
    BatchGenerateChapters,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovelWorkerTaskEnvelope {
    pub task_id: String,
    pub project_id: String,
    pub job_id: i64,
    pub stage_code: String,
    pub task_type: NovelWorkerTaskType,
    pub interaction_version: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chapter_number: Option<i32>,
    #[serde(default = "empty_json_object")]
    pub payload: Value,
    pub created_at: DateTime<Utc>,
}

impl NovelWorkerTaskEnvelope {
    pub const QUEUE_KEY: &'static str = "novel_worker_tasks";
}

// =============================================================================
// 15. Common mutation response
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovelMutationResponse {
    pub project_id: String,
    pub status: String,
    pub accepted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interaction_version: Option<i64>,
}

// =============================================================================
// Query helpers (handlers / routes)
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct NovelChapterListQuery {
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct NovelKnowledgeChunksQuery {
    pub knowledge_import_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct NovelEventsQuery {
    pub after_sequence: Option<i64>,
    pub limit: Option<i64>,
}

// =============================================================================
// 16. Worker Callback Event (worker → API)
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovelWorkerCallbackEvent {
    pub project_id: String,
    pub job_id: i64,
    pub event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chapter_number: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage_run_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_hash: Option<String>,
    #[serde(default = "empty_json_object")]
    pub result_payload: Value,
    #[serde(default = "empty_json_object")]
    pub error_payload: Value,
    #[serde(default = "empty_json_object")]
    pub payload: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempt_no: Option<i32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn worker_task_type_serializes_as_snake_case() {
        let value =
            serde_json::to_value(NovelWorkerTaskType::GenerateArchitecture).expect("serialize");
        assert_eq!(value, json!("generate_architecture"));
    }

    #[test]
    fn worker_task_envelope_round_trips() {
        let envelope = NovelWorkerTaskEnvelope {
            task_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            project_id: "660e8400-e29b-41d4-a716-446655440001".to_string(),
            job_id: 123,
            stage_code: "n01_architecture".to_string(),
            task_type: NovelWorkerTaskType::GenerateArchitecture,
            interaction_version: 1,
            chapter_number: None,
            payload: json!({ "config_snapshot_id": 10 }),
            created_at: Utc::now(),
        };

        let serialized = serde_json::to_string(&envelope).expect("serialize envelope");
        let parsed: NovelWorkerTaskEnvelope =
            serde_json::from_str(&serialized).expect("deserialize envelope");

        assert_eq!(parsed.task_id, envelope.task_id);
        assert_eq!(parsed.project_id, envelope.project_id);
        assert_eq!(parsed.job_id, 123);
        assert_eq!(parsed.payload.get("config_snapshot_id"), Some(&json!(10)));
        assert_eq!(NovelWorkerTaskEnvelope::QUEUE_KEY, "novel_worker_tasks");
    }
}
