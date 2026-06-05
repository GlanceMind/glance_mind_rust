//! OpenMontage snapshot transform tests (preflight/pipelines schema realignment)
//!
//! Reproduces the production 500 on `GET /openmontage/preflight` and
//! `GET /openmontage/pipelines`: the worker publishes one JSON shape to redis,
//! the rust API used to `serde_json::from_str::<DTO>` it into a DIFFERENT
//! (drama-style) DTO, so parsing failed -> HTTP 500.
//!
//! These tests feed the EXACT worker-published JSON (confirmed against
//! `lib/protocol_export.py::build_preflight_snapshot` /
//! `tools/tool_registry.py::provider_menu_summary`) through the pure transform
//! functions and assert the OUTPUT matches the FRONTEND contract
//! (`packages/shared/src/api/openmontageTypes.ts`):
//!
//!   PreflightDto { composition_runtimes: {ffmpeg,remotion,hyperframes},
//!                  available_pipelines: string[], tool_availability: Record<string,bool>,
//!                  setup_offers: [{provider, instructions}] }
//!   PipelineInfoDto { id, name, description, best_for, stability, required_tools }
//!   PipelinesDto { pipelines: PipelineInfoDto[] }
//!
//! The transform MUST be tolerant: missing/wrong-typed worker fields degrade to
//! sensible defaults so these endpoints never 500 again.

use glance_mind_api::service::openmontage_client::{transform_pipelines, transform_preflight};

/// The exact `openmontage:preflight` JSON the worker publishes (abbreviated to
/// the fields the transform reads, with the real array/object shapes).
fn worker_preflight_json() -> serde_json::Value {
    serde_json::json!({
        "composition_runtimes": [
            {"name": "ffmpeg", "available": true},
            {"name": "remotion", "available": true},
            {"name": "hyperframes", "available": false}
        ],
        "capabilities": [
            {
                "capability": "video_generation",
                "configured": 10,
                "total": 16,
                "available_providers": ["fal", "heygen"],
                "unavailable_providers": ["openai"]
            }
        ],
        "setup_offers": [
            {
                "capability": "music_generation",
                "tool": "suno_music",
                "provider": "suno",
                "install_instructions": "Add SUNO_API_KEY to .env"
            }
        ],
        "runtime_warnings": ["hyperframes: npm package `hyperframes` not resolvable"],
        "tools": [
            {"name": "ffmpeg_compose", "status": "available", "stability": "production"},
            {"name": "suno_music", "status": "unavailable", "stability": "experimental"}
        ],
        "pipelines": [
            {"name": "avatar-spokesperson", "version": "2.0", "stability": "production"},
            {"name": "animated-explainer", "version": "1.0", "stability": "production"}
        ],
        "captured_at": "2026-06-05T00:00:00+00:00",
        "provider_menu_summary_json": "{}"
    })
}

/// The exact `openmontage:pipelines` JSON the worker publishes: a BARE ARRAY of
/// pipeline manifests (NOT an object).
fn worker_pipelines_json() -> serde_json::Value {
    serde_json::json!([
        {
            "name": "avatar-spokesperson",
            "version": "2.0",
            "description": "A talking-head spokesperson video.",
            "category": "custom",
            "stability": "production",
            "default_checkpoint_policy": "guided",
            "compatible_playbooks": ["news-anchor"]
        },
        {
            "name": "animated-explainer",
            "version": "1.0",
            "description": "Topic to fully generated explainer.",
            "category": "animation",
            "stability": "production",
            "default_checkpoint_policy": "auto",
            "compatible_playbooks": []
        }
    ])
}

#[test]
fn transform_preflight_maps_runtime_array_to_object_booleans() {
    let dto = transform_preflight(&worker_preflight_json());

    // composition_runtimes: ARRAY [{name,available}] -> OBJECT {ffmpeg,remotion,hyperframes}
    assert!(
        dto.composition_runtimes.ffmpeg,
        "ffmpeg should be true (worker available=true)"
    );
    assert!(
        dto.composition_runtimes.remotion,
        "remotion should be true (worker available=true)"
    );
    assert!(
        !dto.composition_runtimes.hyperframes,
        "hyperframes should be false (worker available=false)"
    );

    // available_pipelines: names lifted from the worker preflight `pipelines` field
    assert_eq!(
        dto.available_pipelines,
        vec![
            "avatar-spokesperson".to_string(),
            "animated-explainer".to_string()
        ]
    );

    // setup_offers: {provider, instructions} mapped from worker {provider, install_instructions}
    assert_eq!(dto.setup_offers.len(), 1);
    assert_eq!(dto.setup_offers[0].provider, "suno");
    assert_eq!(dto.setup_offers[0].instructions, "Add SUNO_API_KEY to .env");

    // tool_availability: best-effort name -> (status=="available")
    assert_eq!(
        dto.tool_availability.get("ffmpeg_compose"),
        Some(&true),
        "available tool maps to true"
    );
    assert_eq!(
        dto.tool_availability.get("suno_music"),
        Some(&false),
        "unavailable tool maps to false"
    );
}

#[test]
fn transform_pipelines_wraps_bare_array_with_id_equals_name() {
    let dto = transform_pipelines(&worker_pipelines_json());

    assert_eq!(dto.pipelines.len(), 2);

    let first = &dto.pipelines[0];
    // id == name (frontend keys the dropdown by id, labels by name)
    assert_eq!(first.id, "avatar-spokesperson");
    assert_eq!(first.name, "avatar-spokesperson");
    assert_eq!(first.description, "A talking-head spokesperson video.");
    assert_eq!(first.best_for, "custom"); // from worker `category`
    assert_eq!(first.stability, "production");
    // pipeline-level required_tools not present in worker manifest -> empty
    assert_eq!(first.required_tools, Vec::<String>::new());

    let second = &dto.pipelines[1];
    assert_eq!(second.id, "animated-explainer");
    assert_eq!(second.name, "animated-explainer");
}

#[test]
fn transform_preflight_tolerates_empty_object() {
    // A malformed / empty worker snapshot must NOT panic and must NOT error:
    // it degrades to all-false runtimes + empty collections (so the endpoint
    // returns 200 instead of 500).
    let dto = transform_preflight(&serde_json::json!({}));
    assert!(!dto.composition_runtimes.ffmpeg);
    assert!(!dto.composition_runtimes.remotion);
    assert!(!dto.composition_runtimes.hyperframes);
    assert!(dto.available_pipelines.is_empty());
    assert!(dto.tool_availability.is_empty());
    assert!(dto.setup_offers.is_empty());
}

#[test]
fn transform_pipelines_tolerates_non_array() {
    // If the worker drifts and publishes a non-array (e.g. an object), the
    // transform degrades to an empty list rather than panicking.
    let dto = transform_pipelines(&serde_json::json!({"unexpected": "object"}));
    assert!(dto.pipelines.is_empty());
}

#[test]
fn preflight_dto_serializes_to_frontend_shape() {
    // The serialized JSON the API hands the desktop must carry the frontend keys.
    let dto = transform_preflight(&worker_preflight_json());
    let v = serde_json::to_value(&dto).unwrap();
    assert!(v.get("composition_runtimes").is_some());
    assert_eq!(v["composition_runtimes"]["ffmpeg"], true);
    assert_eq!(v["composition_runtimes"]["hyperframes"], false);
    assert!(v.get("available_pipelines").is_some());
    assert!(v.get("tool_availability").is_some());
    assert!(v.get("setup_offers").is_some());
}
