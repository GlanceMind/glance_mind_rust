use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
pub struct DramaPreflightRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub content_type: Option<String>,
    pub target_duration_seconds: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DramaPreflightResponse {
    pub passed: bool,
    pub status: String,
    pub blocking: Vec<String>,
    pub warnings: Vec<String>,
    #[serde(default)]
    pub clarification_requests: Vec<Value>,
    pub estimated_cost_cents: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DramaProjectCreateRequest {
    pub title: String,
    pub description: String,
    #[serde(flatten)]
    pub extra: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DramaProjectCreateResponse {
    pub project_id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interaction_version: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reserve_cents: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DramaProjectDetailResponse {
    pub project_id: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_stage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending_stage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress_percent: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interaction_version: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_summary: Option<DramaCostSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interaction: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DramaProjectMetaRequest {
    #[serde(default)]
    pub characters: Vec<Value>,
    #[serde(default)]
    pub style_references: Vec<Value>,
    #[serde(default)]
    pub text_materials: Vec<Value>,
    #[serde(default)]
    pub visual_settings: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DramaProjectMetaResponse {
    pub project_id: String,
    #[serde(default)]
    pub characters: Vec<Value>,
    #[serde(default)]
    pub style_references: Vec<Value>,
    #[serde(default)]
    pub text_materials: Vec<Value>,
    #[serde(default)]
    pub visual_settings: Value,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DramaPrivateCharacterRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gender: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub appearance: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub personality: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_image_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DramaPrivateCharacterUpdateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gender: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub appearance: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub personality: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_image_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DramaPrivateCharacterResponse {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gender: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub appearance: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub personality: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_image_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DramaPrivateCharacterListResponse {
    #[serde(default)]
    pub items: Vec<DramaPrivateCharacterResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DramaPrivateSceneAssetRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location_description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_of_day: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mood: Option<String>,
    #[serde(default)]
    pub reference_image_urls: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub camera_notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DramaPrivateSceneAssetUpdateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location_description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_of_day: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mood: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_image_urls: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub camera_notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DramaPrivateSceneAssetResponse {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location_description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_of_day: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mood: Option<String>,
    #[serde(default)]
    pub reference_image_urls: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub camera_notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DramaPrivateSceneAssetListResponse {
    #[serde(default)]
    pub items: Vec<DramaPrivateSceneAssetResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DramaPrivateStyleAssetRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visual_style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_tone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lighting_mood: Option<String>,
    #[serde(default)]
    pub reference_image_urls: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DramaPrivateStyleAssetUpdateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visual_style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_tone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lighting_mood: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_image_urls: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DramaPrivateStyleAssetResponse {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visual_style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_tone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lighting_mood: Option<String>,
    #[serde(default)]
    pub reference_image_urls: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DramaPrivateStyleAssetListResponse {
    #[serde(default)]
    pub items: Vec<DramaPrivateStyleAssetResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DramaProjectResourcesRequest {
    #[serde(default)]
    pub character_ids: Vec<String>,
    #[serde(default)]
    pub scene_asset_ids: Vec<String>,
    #[serde(default)]
    pub style_asset_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_style_asset_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DramaProjectResourcesResponse {
    pub project_id: String,
    #[serde(default)]
    pub character_ids: Vec<String>,
    #[serde(default)]
    pub scene_asset_ids: Vec<String>,
    #[serde(default)]
    pub style_asset_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_style_asset_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DramaChapterSceneAssetsRequest {
    #[serde(default)]
    pub scene_asset_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DramaChapterSceneAssetsResponse {
    pub project_id: String,
    pub chapter_id: String,
    #[serde(default)]
    pub scene_asset_ids: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DramaCostSummary {
    pub reserve_cents: i64,
    pub consumed_cents: i64,
}

#[derive(Debug, Deserialize)]
pub struct DramaProjectListQuery {
    pub status: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DramaProjectListResponse {
    pub items: Vec<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    pub limit: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DramaClarifyRequest {
    pub session_id: Option<String>,
    pub answers: Vec<Value>,
    pub interaction_version: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DramaStrategySelectRequest {
    pub selected_option_id: String,
    pub interaction_version: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DramaApproveRequest {
    pub approved: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision_notes: Option<String>,
    pub interaction_version: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DramaMutationResponse {
    pub project_id: String,
    pub status: String,
    pub accepted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interaction_version: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DramaShotCameraDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub angle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub movement: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lens_mm: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth_of_field: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DramaShotDto {
    pub shot_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scene_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequence: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shot_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub camera: Option<DramaShotCameraDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asset_pack_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DramaShotsResponse {
    pub project_id: String,
    #[serde(default)]
    pub shots: Vec<DramaShotDto>,
    #[serde(default)]
    pub visual_style: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DramaFallbackEventDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason_category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason_detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(default)]
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DramaFallbacksResponse {
    pub project_id: String,
    #[serde(default)]
    pub fallback_events: Vec<DramaFallbackEventDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DramaWorkerTaskType {
    GenerateOutline,
    GenerateEpisodeScripts,
    ExtractCharacters,
    ExtractScenes,
    GenerateStoryboards,
    GenerateCharacterImages,
    GenerateSceneImages,
    GenerateVideos,
    MergeEpisode,
    PublishArtifacts,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DramaWorkerTaskTarget {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub episode_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scene_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storyboard_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub character_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prop_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DramaWorkerTaskEnvelope {
    pub task_id: String,
    pub project_id: String,
    pub task_type: DramaWorkerTaskType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interaction_version: Option<i64>,
    #[serde(default)]
    pub target: DramaWorkerTaskTarget,
    #[serde(default)]
    pub payload: Value,
    pub created_at: DateTime<Utc>,
}

impl DramaWorkerTaskEnvelope {
    pub const QUEUE_KEY: &'static str = "drama_worker_tasks";
}

/// Internal callback event envelope from gm_agent_hub.
/// Must stay field-compatible with Python DramaCallbackEventV1.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DramaCallbackEvent {
    pub event_id: String,
    pub project_id: String,
    pub run_id: String,
    pub sequence: i64,
    pub stage_code: Option<String>,
    pub event_type: String,
    pub occurred_at: DateTime<Utc>,
    #[serde(default)]
    pub payload: Value,
}

impl DramaCallbackEvent {
    pub fn idempotency_key(&self) -> String {
        format!("{}:{}:{}", self.project_id, self.run_id, self.event_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn worker_task_type_serializes_as_snake_case() {
        let value = serde_json::to_value(DramaWorkerTaskType::GenerateStoryboards)
            .expect("serialize worker task type");
        assert_eq!(value, json!("generate_storyboards"));
    }

    #[test]
    fn worker_task_envelope_round_trips() {
        let envelope = DramaWorkerTaskEnvelope {
            task_id: "task_001".to_string(),
            project_id: "proj_001".to_string(),
            task_type: DramaWorkerTaskType::GenerateSceneImages,
            stage_code: Some("s04_execution".to_string()),
            job_id: Some(42),
            interaction_version: Some(3),
            target: DramaWorkerTaskTarget {
                episode_id: Some(7),
                scene_id: Some(9),
                storyboard_id: None,
                character_id: None,
                prop_id: None,
            },
            payload: json!({
                "provider_profile_ref": "jimeng-default",
                "style": "neon_romance"
            }),
            created_at: Utc::now(),
        };

        let serialized =
            serde_json::to_string(&envelope).expect("serialize worker task envelope");
        let deserialized: DramaWorkerTaskEnvelope =
            serde_json::from_str(&serialized).expect("deserialize worker task envelope");

        assert_eq!(deserialized.task_id, "task_001");
        assert_eq!(deserialized.project_id, "proj_001");
        assert_eq!(
            deserialized.task_type,
            DramaWorkerTaskType::GenerateSceneImages
        );
        assert_eq!(deserialized.target.episode_id, Some(7));
        assert_eq!(deserialized.target.scene_id, Some(9));
        assert_eq!(
            deserialized.payload.get("provider_profile_ref"),
            Some(&json!("jimeng-default"))
        );
    }

    #[test]
    fn callback_event_builds_idempotency_key() {
        let event = DramaCallbackEvent {
            event_id: "evt_123".to_string(),
            project_id: "proj_123".to_string(),
            run_id: "run_456".to_string(),
            sequence: 9,
            stage_code: Some("s03_asset".to_string()),
            event_type: "artifact_uploaded".to_string(),
            occurred_at: Utc::now(),
            payload: json!({"asset_id": "asset_001"}),
        };

        assert_eq!(event.idempotency_key(), "proj_123:run_456:evt_123");
    }

    #[test]
    fn private_character_create_request_requires_name() {
        let err = serde_json::from_value::<DramaPrivateCharacterRequest>(json!({
            "notes": "missing name"
        }))
        .expect_err("create request without name should fail");

        assert!(err.to_string().contains("name"));
    }

    #[test]
    fn private_character_update_request_supports_partial_fields() {
        let update = serde_json::from_value::<DramaPrivateCharacterUpdateRequest>(json!({
            "name": "Ava Prime",
            "notes": "Updated"
        }))
        .expect("deserialize partial update request");

        assert_eq!(update.name.as_deref(), Some("Ava Prime"));
        assert_eq!(update.notes.as_deref(), Some("Updated"));
        assert_eq!(update.voice_id, None);
        assert_eq!(update.reference_image_url, None);
    }

    #[test]
    fn private_scene_asset_create_request_defaults_reference_image_urls() {
        let request = serde_json::from_value::<DramaPrivateSceneAssetRequest>(json!({
            "name": "Rooftop Night",
            "mood": "tense"
        }))
        .expect("deserialize scene asset create request");

        assert_eq!(request.name, "Rooftop Night");
        assert_eq!(request.mood.as_deref(), Some("tense"));
        assert!(request.reference_image_urls.is_empty());
    }

    #[test]
    fn private_scene_asset_update_request_supports_partial_fields() {
        let update = serde_json::from_value::<DramaPrivateSceneAssetUpdateRequest>(json!({
            "name": "Rooftop Dawn",
            "reference_image_urls": ["https://example.com/roof-2.png"]
        }))
        .expect("deserialize scene asset update request");

        assert_eq!(update.name.as_deref(), Some("Rooftop Dawn"));
        assert_eq!(
            update.reference_image_urls,
            Some(vec!["https://example.com/roof-2.png".to_string()])
        );
        assert_eq!(update.mood, None);
        assert_eq!(update.notes, None);
    }

    #[test]
    fn private_style_asset_create_request_defaults_reference_image_urls() {
        let request = serde_json::from_value::<DramaPrivateStyleAssetRequest>(json!({
            "name": "Neon Pulse",
            "visual_style": "cyberpunk_neon"
        }))
        .expect("deserialize style asset create request");

        assert_eq!(request.name, "Neon Pulse");
        assert_eq!(request.visual_style.as_deref(), Some("cyberpunk_neon"));
        assert!(request.reference_image_urls.is_empty());
    }

    #[test]
    fn private_style_asset_update_request_supports_partial_fields() {
        let update = serde_json::from_value::<DramaPrivateStyleAssetUpdateRequest>(json!({
            "lighting_mood": "moody",
            "reference_image_urls": ["https://example.com/style-2.png"]
        }))
        .expect("deserialize style asset update request");

        assert_eq!(update.lighting_mood.as_deref(), Some("moody"));
        assert_eq!(
            update.reference_image_urls,
            Some(vec!["https://example.com/style-2.png".to_string()])
        );
        assert_eq!(update.name, None);
        assert_eq!(update.notes, None);
    }
}
