use crate::schema::{
    gm_novel_architecture_checkpoints, gm_novel_architectures, gm_novel_blueprint_chapters,
    gm_novel_blueprints, gm_novel_chapter_prompts, gm_novel_chapters,
    gm_novel_character_state_snapshots, gm_novel_consistency_checks,
    gm_novel_embedding_profiles, gm_novel_global_summary_snapshots, gm_novel_jobs,
    gm_novel_knowledge_chunks, gm_novel_knowledge_imports, gm_novel_llm_profiles,
    gm_novel_memory_chunks, gm_novel_plot_arc_snapshots, gm_novel_project_config_snapshots,
    gm_novel_projects, gm_novel_stage_runs,
};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

// =============================================================================
// NovelProject
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Selectable)]
#[diesel(table_name = gm_novel_projects)]
#[diesel(primary_key(project_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NovelProject {
    pub project_id: String,
    pub user_id: i32,
    pub title: String,
    pub topic: String,
    pub genre: String,
    pub description: String,
    pub num_chapters: i32,
    pub target_words_per_chapter: i32,
    pub default_user_guidance: Option<String>,
    pub status: String,
    pub current_stage: Option<String>,
    pub pending_stage: Option<String>,
    pub current_chapter_number: Option<i32>,
    pub progress_percent: f32,
    pub interaction_version: i32,
    pub last_error_message: Option<String>,
    pub metadata: JsonValue,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = gm_novel_projects)]
pub struct NewNovelProject {
    pub project_id: String,
    pub user_id: i32,
    pub title: String,
    pub topic: String,
    pub genre: String,
    pub description: String,
    pub num_chapters: i32,
    pub target_words_per_chapter: i32,
    pub default_user_guidance: Option<String>,
    pub status: String,
    pub metadata: JsonValue,
}

// =============================================================================
// NovelLlmProfile
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Selectable)]
#[diesel(table_name = gm_novel_llm_profiles)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NovelLlmProfile {
    pub id: i64,
    pub user_id: i32,
    pub name: String,
    pub interface_format: String,
    pub base_url: String,
    pub api_key: String,
    pub model_name: String,
    pub temperature: f64,
    pub max_tokens: i32,
    pub timeout_seconds: i32,
    pub is_default: bool,
    pub is_active: bool,
    pub metadata: JsonValue,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = gm_novel_llm_profiles)]
pub struct NewNovelLlmProfile {
    pub user_id: i32,
    pub name: String,
    pub interface_format: String,
    pub base_url: String,
    pub api_key: String,
    pub model_name: String,
    pub temperature: f64,
    pub max_tokens: i32,
    pub timeout_seconds: i32,
    pub is_default: bool,
    pub is_active: bool,
    pub metadata: JsonValue,
}

// =============================================================================
// NovelEmbeddingProfile
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Selectable)]
#[diesel(table_name = gm_novel_embedding_profiles)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NovelEmbeddingProfile {
    pub id: i64,
    pub user_id: i32,
    pub name: String,
    pub interface_format: String,
    pub base_url: String,
    pub api_key: String,
    pub model_name: String,
    pub retrieval_k: i32,
    pub is_default: bool,
    pub is_active: bool,
    pub metadata: JsonValue,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = gm_novel_embedding_profiles)]
pub struct NewNovelEmbeddingProfile {
    pub user_id: i32,
    pub name: String,
    pub interface_format: String,
    pub base_url: String,
    pub api_key: String,
    pub model_name: String,
    pub retrieval_k: i32,
    pub is_default: bool,
    pub is_active: bool,
    pub metadata: JsonValue,
}

// =============================================================================
// NovelProjectConfigSnapshot
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Selectable)]
#[diesel(table_name = gm_novel_project_config_snapshots)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NovelProjectConfigSnapshot {
    pub id: i64,
    pub project_id: String,
    pub architecture_llm_profile_id: Option<i64>,
    pub chapter_outline_llm_profile_id: Option<i64>,
    pub prompt_draft_llm_profile_id: Option<i64>,
    pub final_chapter_llm_profile_id: Option<i64>,
    pub consistency_review_llm_profile_id: Option<i64>,
    pub embedding_profile_id: Option<i64>,
    pub proxy_setting: JsonValue,
    pub webdav_config: JsonValue,
    pub other_params: JsonValue,
    pub is_current: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = gm_novel_project_config_snapshots)]
pub struct NewNovelProjectConfigSnapshot {
    pub project_id: String,
    pub architecture_llm_profile_id: Option<i64>,
    pub chapter_outline_llm_profile_id: Option<i64>,
    pub prompt_draft_llm_profile_id: Option<i64>,
    pub final_chapter_llm_profile_id: Option<i64>,
    pub consistency_review_llm_profile_id: Option<i64>,
    pub embedding_profile_id: Option<i64>,
    pub proxy_setting: JsonValue,
    pub webdav_config: JsonValue,
    pub other_params: JsonValue,
    pub is_current: bool,
}

// =============================================================================
// NovelJob
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Selectable)]
#[diesel(table_name = gm_novel_jobs)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NovelJob {
    pub id: i64,
    pub project_id: String,
    pub chapter_number: Option<i32>,
    pub stage_code: String,
    pub task_type: String,
    pub status: String,
    pub idempotency_key: Option<String>,
    pub request_payload: JsonValue,
    pub result_payload: JsonValue,
    pub error_payload: JsonValue,
    pub created_by: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = gm_novel_jobs)]
pub struct NewNovelJob {
    pub project_id: String,
    pub chapter_number: Option<i32>,
    pub stage_code: String,
    pub task_type: String,
    pub status: String,
    pub idempotency_key: Option<String>,
    pub request_payload: JsonValue,
    pub created_by: Option<i32>,
}

// =============================================================================
// NovelStageRun
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Selectable)]
#[diesel(table_name = gm_novel_stage_runs)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NovelStageRun {
    pub id: i64,
    pub project_id: String,
    pub job_id: i64,
    pub chapter_number: i32,
    pub stage_code: String,
    pub status: String,
    pub input_hash: String,
    pub input_payload: JsonValue,
    pub output_payload: JsonValue,
    pub error_message: Option<String>,
    pub attempt_no: i32,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// =============================================================================
// NovelArchitectureCheckpoint
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Selectable)]
#[diesel(table_name = gm_novel_architecture_checkpoints)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NovelArchitectureCheckpoint {
    pub id: i64,
    pub project_id: String,
    pub core_seed_result: Option<String>,
    pub character_dynamics_result: Option<String>,
    pub character_state_result: Option<String>,
    pub world_building_result: Option<String>,
    pub plot_arch_result: Option<String>,
    pub status: String,
    pub source_stage_run_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// =============================================================================
// NovelArchitecture
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Selectable)]
#[diesel(table_name = gm_novel_architectures)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NovelArchitecture {
    pub id: i64,
    pub project_id: String,
    pub core_seed_text: String,
    pub character_dynamics_text: String,
    pub world_building_text: String,
    pub plot_architecture_text: String,
    pub full_text: String,
    pub version_no: i32,
    pub is_current: bool,
    pub source_stage_run_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// =============================================================================
// NovelBlueprint
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Selectable)]
#[diesel(table_name = gm_novel_blueprints)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NovelBlueprint {
    pub id: i64,
    pub project_id: String,
    pub raw_text: String,
    pub chunk_size: Option<i32>,
    pub generated_chapter_count: i32,
    pub version_no: i32,
    pub is_current: bool,
    pub source_stage_run_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// =============================================================================
// NovelBlueprintChapter
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Selectable)]
#[diesel(table_name = gm_novel_blueprint_chapters)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NovelBlueprintChapter {
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

// =============================================================================
// NovelChapterPrompt
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Selectable)]
#[diesel(table_name = gm_novel_chapter_prompts)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NovelChapterPrompt {
    pub id: i64,
    pub project_id: String,
    pub chapter_number: i32,
    pub blueprint_chapter_id: Option<i64>,
    pub user_guidance: Option<String>,
    pub characters_involved: Option<String>,
    pub key_items: Option<String>,
    pub scene_location: Option<String>,
    pub time_constraint: Option<String>,
    pub short_summary: Option<String>,
    pub previous_excerpt: Option<String>,
    pub filtered_context: Option<String>,
    pub prompt_text: String,
    pub edited_prompt_text: Option<String>,
    pub is_current: bool,
    pub source_stage_run_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// =============================================================================
// NovelChapter
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Selectable)]
#[diesel(table_name = gm_novel_chapters)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NovelChapter {
    pub id: i64,
    pub project_id: String,
    pub chapter_number: i32,
    pub blueprint_chapter_id: Option<i64>,
    pub prompt_id: Option<i64>,
    pub title: String,
    pub user_guidance: Option<String>,
    pub characters_involved: Option<String>,
    pub key_items: Option<String>,
    pub scene_location: Option<String>,
    pub time_constraint: Option<String>,
    pub draft_text: Option<String>,
    pub final_text: Option<String>,
    pub status: String,
    pub is_enriched: bool,
    pub target_words: Option<i32>,
    pub draft_word_count: i32,
    pub final_word_count: i32,
    pub consistency_status: Option<String>,
    pub source_stage_run_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub finalized_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = gm_novel_chapters)]
pub struct NewNovelChapter {
    pub project_id: String,
    pub chapter_number: i32,
    pub blueprint_chapter_id: Option<i64>,
    pub prompt_id: Option<i64>,
    pub title: String,
    pub user_guidance: Option<String>,
    pub characters_involved: Option<String>,
    pub key_items: Option<String>,
    pub scene_location: Option<String>,
    pub time_constraint: Option<String>,
    pub status: String,
    pub target_words: Option<i32>,
}

// =============================================================================
// NovelCharacterStateSnapshot
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Selectable)]
#[diesel(table_name = gm_novel_character_state_snapshots)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NovelCharacterStateSnapshot {
    pub id: i64,
    pub project_id: String,
    pub chapter_number: Option<i32>,
    pub state_text: String,
    pub version_no: i32,
    pub is_current: bool,
    pub source_stage_run_id: Option<i64>,
    pub created_at: DateTime<Utc>,
}

// =============================================================================
// NovelGlobalSummarySnapshot
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Selectable)]
#[diesel(table_name = gm_novel_global_summary_snapshots)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NovelGlobalSummarySnapshot {
    pub id: i64,
    pub project_id: String,
    pub chapter_number: Option<i32>,
    pub summary_text: String,
    pub version_no: i32,
    pub is_current: bool,
    pub source_stage_run_id: Option<i64>,
    pub created_at: DateTime<Utc>,
}

// =============================================================================
// NovelPlotArcSnapshot
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Selectable)]
#[diesel(table_name = gm_novel_plot_arc_snapshots)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NovelPlotArcSnapshot {
    pub id: i64,
    pub project_id: String,
    pub chapter_number: Option<i32>,
    pub plot_arcs_text: String,
    pub version_no: i32,
    pub is_current: bool,
    pub source_stage_run_id: Option<i64>,
    pub created_at: DateTime<Utc>,
}

// =============================================================================
// NovelConsistencyCheck
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Selectable)]
#[diesel(table_name = gm_novel_consistency_checks)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NovelConsistencyCheck {
    pub id: i64,
    pub project_id: String,
    pub chapter_number: i32,
    pub novel_setting_text: Option<String>,
    pub character_state_text: Option<String>,
    pub global_summary_text: Option<String>,
    pub plot_arcs_text: Option<String>,
    pub chapter_text: String,
    pub result_text: String,
    pub status: String,
    pub source_stage_run_id: Option<i64>,
    pub created_at: DateTime<Utc>,
}

// =============================================================================
// NovelKnowledgeImport
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Selectable)]
#[diesel(table_name = gm_novel_knowledge_imports)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NovelKnowledgeImport {
    pub id: i64,
    pub project_id: String,
    pub source_name: String,
    pub source_type: String,
    pub original_text: String,
    pub segment_count: i32,
    pub status: String,
    pub source_stage_run_id: Option<i64>,
    pub created_at: DateTime<Utc>,
}

// =============================================================================
// NovelKnowledgeChunk
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Selectable)]
#[diesel(table_name = gm_novel_knowledge_chunks)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NovelKnowledgeChunk {
    pub id: i64,
    pub project_id: String,
    pub knowledge_import_id: i64,
    pub chunk_index: i32,
    pub content: String,
    pub metadata: JsonValue,
    pub created_at: DateTime<Utc>,
}

// =============================================================================
// NovelMemoryChunk
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Selectable)]
#[diesel(table_name = gm_novel_memory_chunks)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NovelMemoryChunk {
    pub id: i64,
    pub project_id: String,
    pub source_type: String,
    pub source_ref_id: Option<i64>,
    pub chapter_number: Option<i32>,
    pub chunk_index: i32,
    pub content: String,
    pub embedding_profile_id: Option<i64>,
    pub embedding_json: Option<JsonValue>,
    pub embedding_dim: Option<i32>,
    pub embedding_status: String,
    pub metadata: JsonValue,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
