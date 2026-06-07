//! TDD test for OpenMontage input_mode enum + per-pipeline validation (M0-T3)
//!
//! This test MUST fail during RED phase (InputMode and validate_input_mode_for_pipeline don't exist yet).
//! Once implemented, all assertions must pass.

use glance_mind_api::dto::openmontage_dto::{CreateJobDto, InputMode};

/// Manual test table — independent of production to catch drift.
/// Pipeline -> (allowed_modes, required_asset_roles)
fn get_pipeline_contract_table(
) -> std::collections::HashMap<&'static str, (Vec<&'static str>, Vec<&'static str>)> {
    let mut table = std::collections::HashMap::new();

    table.insert("animated-explainer", (vec!["text_to_video"], vec![]));
    table.insert("animation", (vec!["text_to_video"], vec![]));
    table.insert(
        "avatar-spokesperson",
        (vec!["source_script", "text_to_video"], vec!["avatar"]),
    );
    table.insert(
        "cinematic",
        (
            vec!["source_clip", "reference_driven", "text_to_video"],
            vec![],
        ),
    );
    table.insert(
        "screen-demo",
        (vec!["text_to_video", "source_clip"], vec![]),
    );
    table.insert("hybrid", (vec!["source_clip"], vec!["source_video"]));

    table
}

#[test]
fn test_hybrid_rejects_text_to_video() {
    let result = glance_mind_api::dto::openmontage_dto::validate_input_mode_for_pipeline(
        "hybrid",
        InputMode::TextToVideo,
        &[],
    );
    assert!(result.is_err(), "hybrid should reject text_to_video");
}

#[test]
fn test_hybrid_accepts_source_clip_with_required_role() {
    let result = glance_mind_api::dto::openmontage_dto::validate_input_mode_for_pipeline(
        "hybrid",
        InputMode::SourceClip,
        &["source_video".to_string()],
    );
    assert!(
        result.is_ok(),
        "hybrid should accept source_clip when source_video role present"
    );
}

#[test]
fn test_hybrid_rejects_source_clip_without_required_role() {
    let result = glance_mind_api::dto::openmontage_dto::validate_input_mode_for_pipeline(
        "hybrid",
        InputMode::SourceClip,
        &[],
    );
    assert!(
        result.is_err(),
        "hybrid should reject source_clip when source_video role missing"
    );
}

#[test]
fn test_unknown_input_mode_deserialization_fails() {
    let json = r#"{"input_mode": "not_a_mode"}"#;

    #[derive(serde::Deserialize)]
    struct TestDto {
        input_mode: InputMode,
    }

    let result: Result<TestDto, _> = serde_json::from_str(json);
    assert!(
        result.is_err(),
        "Deserializing unknown input_mode should fail"
    );
}

#[test]
fn test_text_pipeline_accepts_text_to_video() {
    let result = glance_mind_api::dto::openmontage_dto::validate_input_mode_for_pipeline(
        "animated-explainer",
        InputMode::TextToVideo,
        &[],
    );
    assert!(
        result.is_ok(),
        "animated-explainer should accept text_to_video with no required roles"
    );
}

// Property test: for all (pipeline, mode) pairs, validation succeeds iff mode is allowed AND required roles present
#[cfg(test)]
mod proptest_suite {
    use super::*;
    use proptest::prelude::*;

    fn all_pipelines() -> Vec<&'static str> {
        vec![
            "animated-explainer",
            "animation",
            "avatar-spokesperson",
            "cinematic",
            "screen-demo",
            "hybrid",
        ]
    }

    fn all_modes() -> Vec<InputMode> {
        vec![
            InputMode::TextToVideo,
            InputMode::SourceScript,
            InputMode::ImageToVideo,
            InputMode::FirstLastFrame,
            InputMode::ReferenceDriven,
            InputMode::SourceClip,
        ]
    }

    proptest! {
        #[test]
        fn property_validation_matches_table(pipeline_idx in 0usize..6, mode_idx in 0usize..6) {
            let pipelines = all_pipelines();
            let modes = all_modes();

            let pipeline = pipelines[pipeline_idx];
            let mode = modes[mode_idx].clone();

            let table = get_pipeline_contract_table();
            let (allowed_modes, required_roles) = table.get(pipeline).unwrap();

            // Convert mode to snake_case string for table lookup
            let mode_str = match mode {
                InputMode::TextToVideo => "text_to_video",
                InputMode::SourceScript => "source_script",
                InputMode::ImageToVideo => "image_to_video",
                InputMode::FirstLastFrame => "first_last_frame",
                InputMode::ReferenceDriven => "reference_driven",
                InputMode::SourceClip => "source_clip",
            };

            let is_mode_allowed = allowed_modes.contains(&mode_str);

            if is_mode_allowed && required_roles.is_empty() {
                // Should succeed with empty asset_roles
                let result = glance_mind_api::dto::openmontage_dto::validate_input_mode_for_pipeline(
                    pipeline,
                    mode.clone(),
                    &[],
                );
                prop_assert!(result.is_ok(), "Expected Ok for {}/{:?} with no required roles", pipeline, mode);
            } else if is_mode_allowed && !required_roles.is_empty() {
                // Should fail without required roles
                let result_without = glance_mind_api::dto::openmontage_dto::validate_input_mode_for_pipeline(
                    pipeline,
                    mode.clone(),
                    &[],
                );
                prop_assert!(result_without.is_err(), "Expected Err for {}/{:?} without required roles", pipeline, mode);

                // Should succeed with required roles
                let asset_roles: Vec<String> = required_roles.iter().map(|s| s.to_string()).collect();
                let result_with = glance_mind_api::dto::openmontage_dto::validate_input_mode_for_pipeline(
                    pipeline,
                    mode.clone(),
                    &asset_roles,
                );
                prop_assert!(result_with.is_ok(), "Expected Ok for {}/{:?} with required roles", pipeline, mode);
            } else {
                // Mode not allowed — should fail regardless of asset_roles
                let result = glance_mind_api::dto::openmontage_dto::validate_input_mode_for_pipeline(
                    pipeline,
                    mode.clone(),
                    &[],
                );
                prop_assert!(result.is_err(), "Expected Err for {}/{:?} (mode not allowed)", pipeline, mode);
            }
        }
    }
}
