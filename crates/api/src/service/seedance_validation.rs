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

use crate::error::api_error::ApiError;
use crate::error::business_error::BusinessError;
use serde_json::Value as JsonValue;

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
const MIN_DURATION: i64 = 4;
const MAX_DURATION: i64 = 15;

fn invalid_input(msg: impl Into<String>) -> ApiError {
    ApiError::BusinessError(BusinessError::InvalidInput(msg.into()))
}

pub fn validate_seedance_config(ai_input: &JsonValue) -> Result<SeedanceValidated, ApiError> {
    let config = ai_input
        .get("seedance_config")
        .filter(|v| !v.is_null())
        .ok_or_else(|| invalid_input("seedance_config is required in ai_input"))?;

    let mode = config["mode"]
        .as_str()
        .ok_or_else(|| invalid_input("seedance_config.mode is required"))?;
    if !VALID_MODES.contains(&mode) {
        return Err(invalid_input(format!(
            "Invalid seedance_config.mode: '{}'. Supported: {}",
            mode,
            VALID_MODES.join(", ")
        )));
    }

    let duration = config["duration"]
        .as_i64()
        .ok_or_else(|| invalid_input("seedance_config.duration is required (integer 4-15)"))?;
    if !(MIN_DURATION..=MAX_DURATION).contains(&duration) {
        return Err(invalid_input(format!(
            "seedance_config.duration must be {}-{}, got {}",
            MIN_DURATION, MAX_DURATION, duration
        )));
    }

    let quality = config["quality"]
        .as_str()
        .ok_or_else(|| invalid_input("seedance_config.quality is required"))?;
    if !VALID_QUALITIES.contains(&quality) {
        return Err(invalid_input(format!(
            "Invalid seedance_config.quality: '{}'. Supported: {}",
            quality,
            VALID_QUALITIES.join(", ")
        )));
    }

    let aspect_ratio = config["aspect_ratio"]
        .as_str()
        .ok_or_else(|| invalid_input("seedance_config.aspect_ratio is required"))?;
    if !VALID_ASPECT_RATIOS.contains(&aspect_ratio) {
        return Err(invalid_input(format!(
            "Invalid seedance_config.aspect_ratio: '{}'. Supported: {}",
            aspect_ratio,
            VALID_ASPECT_RATIOS.join(", ")
        )));
    }

    let generate_audio = config["generate_audio"].as_bool().unwrap_or(false);

    let image_count = config["image_urls"].as_array().map_or(0, |a| a.len());
    let video_count = config["video_urls"].as_array().map_or(0, |a| a.len());
    let audio_count = config["audio_urls"].as_array().map_or(0, |a| a.len());
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

    match mode {
        "text2video" => {}
        "image2video" => {
            if image_count < 1 {
                return Err(invalid_input(
                    "image2video mode requires at least 1 image in image_urls",
                ));
            }
        }
        "start_end_frame" => {
            if image_count != 2 {
                return Err(invalid_input(
                    "start_end_frame mode requires exactly 2 images in image_urls",
                ));
            }
            validate_start_end_frame_roles(config)?;
        }
        "multimodal" => {
            if total_media < 1 {
                return Err(invalid_input(
                    "multimodal mode requires at least 1 media item",
                ));
            }
        }
        "edit" => {
            if video_count < 1 {
                return Err(invalid_input(
                    "edit mode requires at least 1 video in video_urls",
                ));
            }
        }
        "extend" => {
            if !(1..=3).contains(&video_count) {
                return Err(invalid_input(
                    "extend mode requires 1-3 videos in video_urls",
                ));
            }
        }
        _ => {}
    }

    validate_role_map(
        &config["image_roles"],
        image_count,
        VALID_IMAGE_ROLES,
        "image_roles",
    )?;
    validate_role_map(
        &config["video_roles"],
        video_count,
        VALID_VIDEO_ROLES,
        "video_roles",
    )?;
    validate_role_map(
        &config["audio_roles"],
        audio_count,
        VALID_AUDIO_ROLES,
        "audio_roles",
    )?;

    Ok(SeedanceValidated {
        mode: mode.to_string(),
        duration,
        quality: quality.to_string(),
        generate_audio,
    })
}

pub struct SeedanceValidated {
    pub mode: String,
    pub duration: i64,
    pub quality: String,
    pub generate_audio: bool,
}

fn validate_start_end_frame_roles(config: &JsonValue) -> Result<(), ApiError> {
    let roles = match config["image_roles"].as_object() {
        Some(m) => m,
        None => {
            return Err(invalid_input(
                "start_end_frame requires image_roles with positional role binding",
            ));
        }
    };

    let role_1 = roles.get("1").and_then(|v| v.as_str());
    if role_1 != Some("first_frame") {
        return Err(invalid_input(
            "start_end_frame: image_roles[\"1\"] must be 'first_frame' \
             (first image in image_urls = first frame in video). \
             Ensure the start frame image is at image_urls[0].",
        ));
    }

    let role_2 = roles.get("2").and_then(|v| v.as_str());
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
    roles_value: &JsonValue,
    media_count: usize,
    valid_roles: &[&str],
    field_name: &str,
) -> Result<(), ApiError> {
    let roles = match roles_value.as_object() {
        Some(m) if !m.is_empty() => m,
        _ => return Ok(()),
    };

    for (key, value) in roles {
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
        let role = value
            .as_str()
            .ok_or_else(|| invalid_input(format!("{}.{} must be a string", field_name, key)))?;
        if !valid_roles.contains(&role) {
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
