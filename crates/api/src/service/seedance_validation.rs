//! Seedance 2.0 video configuration validation
//! Validates ai_input.seedance_config against proto SeedanceVideoConfig spec
//!
//! ## Ordering Contract (素材顺序契约)
//!
//! The `image_urls` array order is the **single source of truth** for prompt references:
//! - `image_urls[0]` = "图片1" / "@Image1" in prompt
//! - `image_urls[1]` = "图片2" / "@Image2" in prompt
//! - Same pattern for `video_urls` ("视频N") and `audio_urls` ("音频N")
//! - Each media type is numbered independently (images, videos, audios each start from 1)
//!
//! The `image_roles` map keys ("1"-"9") are 1-based indices into `image_urls`.
//! When the Scheduler builds the Seedance API `content[]` array, it MUST:
//! 1. Iterate `image_urls` by array index (0, 1, 2...)
//! 2. Look up role via `image_roles[str(index+1)]` for each image
//! 3. Append to `content[]` in array order — this preserves the "图片N" mapping
//!
//! For `start_end_frame` mode, position binding is enforced:
//! - `image_roles["1"]` MUST be "first_frame" (first image in content[] = first frame)
//! - `image_roles["2"]` MUST be "last_frame" (second image = last frame)
//!
//! Frontend MUST ensure upload result URLs are added to `image_urls`
//! in the same visual order the user arranged them in the UI.
//!
//! ## Typed access
//!
//! Phase 2.5 migration: parses `ai_input.seedance_config` into typed
//! `SeedanceVideoConfig` (from glance_mind_protocol) once at the top of
//! `validate_seedance_config`; subsequent field access is all compile-checked.
//! Replaces 15+ `config["mode"].as_str()` / `config["image_urls"].as_array()`
//! dynamic accesses. Same error messages preserved for test compatibility.

use crate::error::api_error::ApiError;
use crate::error::business_error::BusinessError;
use glance_mind_protocol::glance_mind::SeedanceVideoConfig;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

const VALID_MODES: &[&str] = &[
    "text2video",
    "image2video",
    "start_end_frame",
    "multimodal",
    "edit",
    "extend",
];
const VALID_QUALITIES: &[&str] = &["480p", "720p"];
const VALID_ASPECT_RATIOS: &[&str] = &["16:9", "9:16", "1:1", "4:3", "3:4", "21:9", "adaptive"];
const VALID_IMAGE_ROLES: &[&str] = &["first_frame", "last_frame", "reference_image"];
const VALID_VIDEO_ROLES: &[&str] = &["reference_video"];
const VALID_AUDIO_ROLES: &[&str] = &["reference_audio"];

const MAX_IMAGES: usize = 9;
const MAX_VIDEOS: usize = 3;
const MAX_AUDIOS: usize = 3;
const MAX_TOTAL_MEDIA: usize = 12;
const MIN_DURATION: i32 = 4;
const MAX_DURATION: i32 = 15;

fn invalid_input(msg: impl Into<String>) -> ApiError {
    ApiError::BusinessError(BusinessError::InvalidInput(msg.into()))
}

pub fn validate_seedance_config(ai_input: &JsonValue) -> Result<SeedanceValidated, ApiError> {
    let config_json = ai_input
        .get("seedance_config")
        .filter(|v| !v.is_null())
        .ok_or_else(|| invalid_input("seedance_config is required in ai_input"))?;

    // Typed parse — lenient (#[serde(default)] via protocol build.rs)
    // tolerates missing fields on the wire; required-field validation
    // (mode / quality / aspect_ratio non-empty) is done below.
    let config: SeedanceVideoConfig = serde_json::from_value(config_json.clone())
        .map_err(|e| invalid_input(format!("seedance_config is malformed: {}", e)))?;

    if config.mode.is_empty() {
        return Err(invalid_input("seedance_config.mode is required"));
    }
    if !VALID_MODES.contains(&config.mode.as_str()) {
        return Err(invalid_input(format!(
            "Invalid seedance_config.mode: '{}'. Supported: {}",
            config.mode,
            VALID_MODES.join(", ")
        )));
    }

    if !(MIN_DURATION..=MAX_DURATION).contains(&config.duration) {
        return Err(invalid_input(format!(
            "seedance_config.duration must be {}-{}, got {}",
            MIN_DURATION, MAX_DURATION, config.duration
        )));
    }

    if config.quality.is_empty() {
        return Err(invalid_input("seedance_config.quality is required"));
    }
    if !VALID_QUALITIES.contains(&config.quality.as_str()) {
        return Err(invalid_input(format!(
            "Invalid seedance_config.quality: '{}'. Supported: {}",
            config.quality,
            VALID_QUALITIES.join(", ")
        )));
    }

    if config.aspect_ratio.is_empty() {
        return Err(invalid_input("seedance_config.aspect_ratio is required"));
    }
    if !VALID_ASPECT_RATIOS.contains(&config.aspect_ratio.as_str()) {
        return Err(invalid_input(format!(
            "Invalid seedance_config.aspect_ratio: '{}'. Supported: {}",
            config.aspect_ratio,
            VALID_ASPECT_RATIOS.join(", ")
        )));
    }

    let image_count = config.image_urls.len();
    let video_count = config.video_urls.len();
    let audio_count = config.audio_urls.len();
    let total_media = image_count + video_count + audio_count;

    if image_count > MAX_IMAGES {
        return Err(invalid_input(format!(
            "seedance_config.image_urls max {}, got {}",
            MAX_IMAGES, image_count
        )));
    }
    if video_count > MAX_VIDEOS {
        return Err(invalid_input(format!(
            "seedance_config.video_urls max {}, got {}",
            MAX_VIDEOS, video_count
        )));
    }
    if audio_count > MAX_AUDIOS {
        return Err(invalid_input(format!(
            "seedance_config.audio_urls max {}, got {}",
            MAX_AUDIOS, audio_count
        )));
    }
    if total_media > MAX_TOTAL_MEDIA {
        return Err(invalid_input(format!(
            "Total media count must be ≤ {}, got {}",
            MAX_TOTAL_MEDIA, total_media
        )));
    }

    match config.mode.as_str() {
        "text2video" => {}
        "image2video" if image_count < 1 => {
            return Err(invalid_input(
                "image2video mode requires at least 1 image in image_urls",
            ));
        }
        "start_end_frame" => {
            if image_count != 2 {
                return Err(invalid_input(
                    "start_end_frame mode requires exactly 2 images in image_urls",
                ));
            }
            validate_start_end_frame_roles(&config.image_roles)?;
        }
        "multimodal" if total_media < 1 => {
            return Err(invalid_input(
                "multimodal mode requires at least 1 media item",
            ));
        }
        "edit" if video_count < 1 => {
            return Err(invalid_input(
                "edit mode requires at least 1 video in video_urls",
            ));
        }
        "extend" if !(1..=3).contains(&video_count) => {
            return Err(invalid_input(
                "extend mode requires 1-3 videos in video_urls",
            ));
        }
        _ => {}
    }

    validate_role_map(
        &config.image_roles,
        image_count,
        VALID_IMAGE_ROLES,
        "image_roles",
    )?;
    validate_role_map(
        &config.video_roles,
        video_count,
        VALID_VIDEO_ROLES,
        "video_roles",
    )?;
    validate_role_map(
        &config.audio_roles,
        audio_count,
        VALID_AUDIO_ROLES,
        "audio_roles",
    )?;

    Ok(SeedanceValidated {
        mode: config.mode.clone(),
        duration: config.duration as i64,
        quality: config.quality.clone(),
        generate_audio: config.generate_audio,
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct SeedanceValidated {
    pub mode: String,
    pub duration: i64,
    pub quality: String,
    pub generate_audio: bool,
}

fn validate_start_end_frame_roles(roles: &HashMap<String, String>) -> Result<(), ApiError> {
    if roles.is_empty() {
        return Err(invalid_input(
            "start_end_frame requires image_roles with positional role binding",
        ));
    }

    let role_1 = roles.get("1").map(String::as_str);
    if role_1 != Some("first_frame") {
        return Err(invalid_input(
            "start_end_frame: image_roles[\"1\"] must be 'first_frame' \
             (first image in image_urls = first frame in video). \
             Ensure the start frame image is at image_urls[0].",
        ));
    }

    let role_2 = roles.get("2").map(String::as_str);
    if role_2 != Some("last_frame") {
        return Err(invalid_input(
            "start_end_frame: image_roles[\"2\"] must be 'last_frame' \
             (second image in image_urls = last frame in video). \
             Ensure the end frame image is at image_urls[1].",
        ));
    }

    Ok(())
}

fn validate_role_map(
    roles: &HashMap<String, String>,
    media_count: usize,
    valid_roles: &[&str],
    field_name: &str,
) -> Result<(), ApiError> {
    if roles.is_empty() {
        return Ok(());
    }

    for (key, role) in roles {
        let idx: usize = key.parse().map_err(|_| {
            invalid_input(format!(
                "{}.key '{}' must be a numeric string",
                field_name, key
            ))
        })?;
        if idx < 1 || idx > media_count {
            return Err(invalid_input(format!(
                "{}.key '{}' out of range (1-{})",
                field_name, key, media_count
            )));
        }
        if !valid_roles.contains(&role.as_str()) {
            return Err(invalid_input(format!(
                "Invalid {}.{} role: '{}'. Supported: {}",
                field_name,
                key,
                role,
                valid_roles.join(", ")
            )));
        }
    }
    Ok(())
}

pub fn is_seedance_plan(ai_input: &Option<JsonValue>) -> bool {
    ai_input
        .as_ref()
        .and_then(|v| v.get("seedance_config"))
        .is_some_and(|v| !v.is_null())
}

pub fn has_content_prompt(ai_input: &Option<JsonValue>) -> bool {
    ai_input
        .as_ref()
        .and_then(|v| v["content_prompt"].as_str())
        .is_some_and(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn base_seedance_config(mode: &str) -> JsonValue {
        json!({
            "seedance_config": {
                "mode": mode,
                "duration": 5,
                "quality": "720p",
                "aspect_ratio": "16:9",
                "generate_audio": false,
                "image_urls": [],
                "video_urls": [],
                "audio_urls": [],
            },
            "content_prompt": "hello",
        })
    }

    #[test]
    fn missing_seedance_config_rejected() {
        let input = json!({"content_prompt": "x"});
        let err = validate_seedance_config(&input).unwrap_err();
        assert!(err.to_string().contains("seedance_config is required"));
    }

    #[test]
    fn malformed_seedance_config_rejected() {
        let input = json!({"seedance_config": "not an object"});
        let err = validate_seedance_config(&input).unwrap_err();
        assert!(err.to_string().contains("malformed"));
    }

    #[test]
    fn text2video_happy_path() {
        let input = base_seedance_config("text2video");
        let ok = validate_seedance_config(&input).unwrap();
        assert_eq!(ok.mode, "text2video");
        assert_eq!(ok.duration, 5);
        assert_eq!(ok.quality, "720p");
    }

    #[test]
    fn invalid_mode_rejected() {
        let input = base_seedance_config("invalidmode");
        let err = validate_seedance_config(&input).unwrap_err();
        assert!(err.to_string().contains("Invalid seedance_config.mode"));
    }

    #[test]
    fn duration_out_of_range_rejected() {
        let mut input = base_seedance_config("text2video");
        input["seedance_config"]["duration"] = json!(20);
        let err = validate_seedance_config(&input).unwrap_err();
        assert!(err.to_string().contains("duration must be"));
    }

    #[test]
    fn invalid_quality_rejected() {
        let mut input = base_seedance_config("text2video");
        input["seedance_config"]["quality"] = json!("4k");
        let err = validate_seedance_config(&input).unwrap_err();
        assert!(err.to_string().contains("Invalid seedance_config.quality"));
    }

    #[test]
    fn invalid_aspect_ratio_rejected() {
        let mut input = base_seedance_config("text2video");
        input["seedance_config"]["aspect_ratio"] = json!("5:1");
        let err = validate_seedance_config(&input).unwrap_err();
        assert!(err
            .to_string()
            .contains("Invalid seedance_config.aspect_ratio"));
    }

    #[test]
    fn image2video_requires_image() {
        let input = base_seedance_config("image2video");
        let err = validate_seedance_config(&input).unwrap_err();
        assert!(err
            .to_string()
            .contains("image2video mode requires at least 1 image"));
    }

    #[test]
    fn image2video_with_image_ok() {
        let mut input = base_seedance_config("image2video");
        input["seedance_config"]["image_urls"] = json!(["https://x/1.jpg"]);
        assert!(validate_seedance_config(&input).is_ok());
    }

    #[test]
    fn start_end_frame_requires_exactly_2_images() {
        let mut input = base_seedance_config("start_end_frame");
        input["seedance_config"]["image_urls"] = json!(["https://x/1.jpg"]);
        let err = validate_seedance_config(&input).unwrap_err();
        assert!(err.to_string().contains("exactly 2 images"));
    }

    #[test]
    fn start_end_frame_requires_role_binding() {
        let mut input = base_seedance_config("start_end_frame");
        input["seedance_config"]["image_urls"] = json!(["https://x/1.jpg", "https://x/2.jpg"]);
        let err = validate_seedance_config(&input).unwrap_err();
        assert!(err.to_string().contains("image_roles"));
    }

    #[test]
    fn start_end_frame_happy_path() {
        let mut input = base_seedance_config("start_end_frame");
        input["seedance_config"]["image_urls"] = json!(["https://x/1.jpg", "https://x/2.jpg"]);
        input["seedance_config"]["image_roles"] = json!({
            "1": "first_frame",
            "2": "last_frame",
        });
        assert!(validate_seedance_config(&input).is_ok());
    }

    #[test]
    fn extend_mode_video_count_range() {
        // 0 videos -> fail
        let input = base_seedance_config("extend");
        let err = validate_seedance_config(&input).unwrap_err();
        assert!(err.to_string().contains("extend mode requires 1-3 videos"));
        // 2 videos -> ok
        let mut input2 = base_seedance_config("extend");
        input2["seedance_config"]["video_urls"] = json!(["https://x/a.mp4", "https://x/b.mp4"]);
        assert!(validate_seedance_config(&input2).is_ok());
    }

    #[test]
    fn image_count_over_max_rejected() {
        let mut input = base_seedance_config("multimodal");
        input["seedance_config"]["image_urls"] = json!((0..10)
            .map(|i| format!("https://x/{}.jpg", i))
            .collect::<Vec<_>>());
        let err = validate_seedance_config(&input).unwrap_err();
        assert!(err.to_string().contains("image_urls max 9"));
    }

    #[test]
    fn role_map_invalid_key_rejected() {
        let mut input = base_seedance_config("multimodal");
        input["seedance_config"]["image_urls"] = json!(["https://x/1.jpg"]);
        input["seedance_config"]["image_roles"] = json!({"notanumber": "first_frame"});
        let err = validate_seedance_config(&input).unwrap_err();
        assert!(err.to_string().contains("must be a numeric string"));
    }

    #[test]
    fn role_map_index_out_of_range() {
        let mut input = base_seedance_config("multimodal");
        input["seedance_config"]["image_urls"] = json!(["https://x/1.jpg"]);
        input["seedance_config"]["image_roles"] = json!({"5": "first_frame"});
        let err = validate_seedance_config(&input).unwrap_err();
        assert!(err.to_string().contains("out of range"));
    }

    #[test]
    fn role_map_invalid_role_rejected() {
        let mut input = base_seedance_config("multimodal");
        input["seedance_config"]["image_urls"] = json!(["https://x/1.jpg"]);
        input["seedance_config"]["image_roles"] = json!({"1": "bogus_role"});
        let err = validate_seedance_config(&input).unwrap_err();
        assert!(err.to_string().contains("Invalid image_roles"));
    }

    #[test]
    fn is_seedance_plan_detection() {
        let v1 = Some(json!({"seedance_config": {"mode": "text2video"}}));
        assert!(is_seedance_plan(&v1));

        let v2 = Some(json!({"seedance_config": null}));
        assert!(!is_seedance_plan(&v2));

        let v3 = Some(json!({}));
        assert!(!is_seedance_plan(&v3));

        let v4 = None;
        assert!(!is_seedance_plan(&v4));
    }

    #[test]
    fn has_content_prompt_detection() {
        assert!(has_content_prompt(&Some(json!({"content_prompt": "x"}))));
        assert!(!has_content_prompt(&Some(json!({"content_prompt": ""}))));
        assert!(!has_content_prompt(&Some(json!({}))));
        assert!(!has_content_prompt(&None));
    }
}
