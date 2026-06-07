//! OpenMontage Cross-Tier Contract Test
//!
//! M0b-T6 (rust part): Validates that request fields accepted by the frontend
//! are TYPED in CreateJobDto and EMITTED into the worker request_json envelope.
//!
//! Tests the following 6 fields:
//! - source_script
//! - voice_selection
//! - production_mode
//! - audience
//! - objective
//! - brand_json
//!
//! Canonical values (shared with engine + front parts):
//! - source_script: "SCENE 1: ..."
//! - voice_selection: "en-US-Wavenet-D"
//! - production_mode: "synthetic_terminal"
//! - audience: "developers"
//! - objective: "awareness"
//! - brand_json: {"primary_color":"#0af","logo_alt":"ACME"}

use glance_mind_api::dto::openmontage_dto::{CreateJobDto, ServerContext};
use serde_json::json;

/// CANONICAL VALUES (shared with engine + frontend parts)
const CANONICAL_SOURCE_SCRIPT: &str = "SCENE 1: ...";
const CANONICAL_VOICE_SELECTION: &str = "en-US-Wavenet-D";
const CANONICAL_PRODUCTION_MODE: &str = "synthetic_terminal";
const CANONICAL_AUDIENCE: &str = "developers";
const CANONICAL_OBJECTIVE: &str = "awareness";

fn canonical_brand_json() -> serde_json::Value {
    json!({
        "primary_color": "#0af",
        "logo_alt": "ACME"
    })
}

#[test]
fn test_cross_tier_fields_emitted_into_request_json() {
    // Build a CreateJobDto with ALL 6 cross-tier fields set to canonical values
    let dto = CreateJobDto {
        title: "Test Job".to_string(),
        prompt: "Create a video about Rust".to_string(),
        target_platform: "youtube".to_string(),
        pipeline: Some("screen-demo".to_string()),
        source_script: Some(CANONICAL_SOURCE_SCRIPT.to_string()),
        voice_selection: Some(CANONICAL_VOICE_SELECTION.to_string()),
        production_mode: Some(CANONICAL_PRODUCTION_MODE.to_string()),
        audience: Some(CANONICAL_AUDIENCE.to_string()),
        objective: Some(CANONICAL_OBJECTIVE.to_string()),
        brand_json: Some(canonical_brand_json()),
        ..Default::default()
    };

    let server_ctx = ServerContext {
        job_id: "test-job-123".to_string(),
        user_id: 999,
        tenant_id: "test-tenant".to_string(),
        callback_secret_ref: Some("test-callback-secret".to_string()),
    };

    // Call to_protocol_request — this builds the worker request_json
    let request_json = dto.to_protocol_request(server_ctx);

    // ASSERT: Each of the 6 fields is present in request_json with the exact canonical value
    // (proving cross-tier emit — none is dropped)

    assert_eq!(
        request_json["source_script"],
        json!(CANONICAL_SOURCE_SCRIPT),
        "source_script must be emitted into request_json"
    );

    assert_eq!(
        request_json["voice_selection"],
        json!(CANONICAL_VOICE_SELECTION),
        "voice_selection must be emitted into request_json"
    );

    assert_eq!(
        request_json["production_mode"],
        json!(CANONICAL_PRODUCTION_MODE),
        "production_mode must be emitted into request_json"
    );

    assert_eq!(
        request_json["audience"],
        json!(CANONICAL_AUDIENCE),
        "audience must be emitted into request_json"
    );

    assert_eq!(
        request_json["objective"],
        json!(CANONICAL_OBJECTIVE),
        "objective must be emitted into request_json"
    );

    assert_eq!(
        request_json["brand_json"],
        canonical_brand_json(),
        "brand_json must be emitted into request_json"
    );
}

#[test]
fn test_cross_tier_fields_round_trip_serde() {
    // Build a JSON body with all 6 fields
    let json_body = json!({
        "title": "Test Job",
        "prompt": "Create a video",
        "target_platform": "youtube",
        "source_script": CANONICAL_SOURCE_SCRIPT,
        "voice_selection": CANONICAL_VOICE_SELECTION,
        "production_mode": CANONICAL_PRODUCTION_MODE,
        "audience": CANONICAL_AUDIENCE,
        "objective": CANONICAL_OBJECTIVE,
        "brand_json": canonical_brand_json(),
    });

    // Deserialize into CreateJobDto
    let dto: CreateJobDto = serde_json::from_value(json_body)
        .expect("Should deserialize JSON with all 6 cross-tier fields");

    // Assert all fields parsed correctly
    assert_eq!(
        dto.source_script.as_deref(),
        Some(CANONICAL_SOURCE_SCRIPT),
        "source_script should parse from JSON"
    );
    assert_eq!(
        dto.voice_selection.as_deref(),
        Some(CANONICAL_VOICE_SELECTION),
        "voice_selection should parse from JSON"
    );
    assert_eq!(
        dto.production_mode.as_deref(),
        Some(CANONICAL_PRODUCTION_MODE),
        "production_mode should parse from JSON"
    );
    assert_eq!(
        dto.audience.as_deref(),
        Some(CANONICAL_AUDIENCE),
        "audience should parse from JSON"
    );
    assert_eq!(
        dto.objective.as_deref(),
        Some(CANONICAL_OBJECTIVE),
        "objective should parse from JSON"
    );
    assert_eq!(
        dto.brand_json,
        Some(canonical_brand_json()),
        "brand_json should parse from JSON"
    );
}

#[test]
fn test_secret_in_brand_json_rejected() {
    // Build a CreateJobDto with a secret nested in brand_json
    let dto = CreateJobDto {
        title: "Test Job".to_string(),
        prompt: "Create a video".to_string(),
        target_platform: "youtube".to_string(),
        brand_json: Some(json!({
            "primary_color": "#0af",
            "api_key": "ghp_FAKE_SECRET_TOKEN_12345"
        })),
        ..Default::default()
    };

    // validate_no_secret_material should reject the secret in brand_json
    let result = dto.validate_no_secret_material();

    assert!(
        result.is_err(),
        "Secret nested in brand_json should be rejected"
    );

    let err_msg = result.unwrap_err();
    // The scan should detect EITHER "api_key" (the JSON key) OR "ghp_" (the token value)
    assert!(
        err_msg.contains("api_key") || err_msg.contains("ghp_"),
        "Error should mention a secret token (api_key or ghp_): {}",
        err_msg
    );
}
