use crate::dto::video_dto::{VideoModelCapabilitiesDto, VideoModelOptionDto};
use glance_mind_db::entity::ai_model::AiModel;

pub fn build_video_model_capabilities(model: &AiModel) -> VideoModelCapabilitiesDto {
    let key = model.model_key.as_str();
    let provider = model.provider.as_str();

    let (default_orientation, orientation_options) = if key == "sora-2-pro" {
        (
            "portrait".to_string(),
            vec![orientation_option("portrait", "portrait (9:16)")],
        )
    } else {
        (
            "landscape".to_string(),
            vec![
                orientation_option("landscape", "landscape (16:9)"),
                orientation_option("portrait", "portrait (9:16)"),
            ],
        )
    };

    let (default_seconds, duration_options) = if is_jimeng_model(key, provider) {
        (
            "5".to_string(),
            vec![
                duration_option("5", "5 sec", Some("Short clip")),
                duration_option("10", "10 sec", Some("Standard clip")),
            ],
        )
    } else {
        (
            "4".to_string(),
            vec![
                duration_option("4", "4 sec", Some("Short clip")),
                duration_option("8", "8 sec", Some("Standard clip")),
            ],
        )
    };

    let (image_input_mode, requires_image, min_images, max_images, supports_keyframe_prompts) =
        match key {
            "vidu-t2v" => ("none".to_string(), false, None, 0, false),
            "vidu-i2v" => ("single".to_string(), true, None, 1, false),
            "vidu-startend" => ("start_end".to_string(), true, None, 2, false),
            "vidu-fast" => ("start_end".to_string(), false, None, 2, false),
            "vidu-ref2v" => ("reference_gallery".to_string(), true, None, 3, false),
            "vidu-multiframe" => ("multi_frame".to_string(), true, Some(3), 10, true),
            "vidu-template" => ("template".to_string(), true, None, 1, false),
            "vidu-general-film" | "vidu-ad-film" => {
                ("material_gallery".to_string(), true, None, 3, false)
            }
            _ if key.contains("-fl") => ("start_end".to_string(), false, None, 2, false),
            _ if is_jimeng_model(key, provider) && !key.contains("pro") => {
                ("start_end".to_string(), false, None, 2, false)
            }
            _ => ("single".to_string(), false, None, 1, false),
        };

    VideoModelCapabilitiesDto {
        default_orientation,
        orientation_options,
        default_seconds,
        duration_options,
        image_input_mode,
        requires_image,
        min_images,
        max_images,
        supports_keyframe_prompts,
    }
}

pub fn preferred_video_model(models: &[AiModel]) -> Option<&AiModel> {
    models
        .iter()
        .find(|model| matches!(model.model_key.as_str(), "sora-2" | "sora2"))
        .or_else(|| {
            models
                .iter()
                .find(|model| model.model_key.starts_with("sora"))
        })
        .or_else(|| models.first())
}

fn orientation_option(value: &str, label: &str) -> VideoModelOptionDto {
    VideoModelOptionDto {
        value: value.to_string(),
        label: label.to_string(),
        description: None,
    }
}

fn duration_option(value: &str, label: &str, description: Option<&str>) -> VideoModelOptionDto {
    VideoModelOptionDto {
        value: value.to_string(),
        label: label.to_string(),
        description: description.map(str::to_string),
    }
}

fn is_jimeng_model(key: &str, provider: &str) -> bool {
    key.starts_with("jimeng-") || provider.eq_ignore_ascii_case("jimeng")
}

#[cfg(test)]
mod tests {
    use super::{build_video_model_capabilities, preferred_video_model};
    use glance_mind_db::entity::ai_model::AiModel;

    fn test_model(id: i32, name: &str, provider: &str, model_key: &str) -> AiModel {
        AiModel {
            id,
            name: name.to_string(),
            provider: provider.to_string(),
            model_key: model_key.to_string(),
            cost_multiplier: 1.into(),
            is_active: true,
            created_at: chrono::DateTime::from_timestamp(0, 0).unwrap().naive_utc(),
            updated_at: None,
            model_type: "video".to_string(),
        }
    }

    #[test]
    fn prefers_sora_two_when_available() {
        let models = vec![
            test_model(1, "Vidu", "vidu", "vidu-t2v"),
            test_model(7, "Sora 2", "openai", "sora-2"),
            test_model(9, "Jimeng", "jimeng", "jimeng-v2"),
        ];

        let selected = preferred_video_model(&models).expect("preferred model");
        assert_eq!(selected.id, 7);
    }

    #[test]
    fn sora_two_capabilities_match_chat_questionnaire_defaults() {
        let capabilities =
            build_video_model_capabilities(&test_model(7, "Sora 2", "openai", "sora-2"));

        assert_eq!(capabilities.default_orientation, "landscape");
        assert_eq!(capabilities.default_seconds, "4");
        assert_eq!(
            capabilities
                .orientation_options
                .iter()
                .map(|option| option.value.as_str())
                .collect::<Vec<_>>(),
            vec!["landscape", "portrait"]
        );
        assert_eq!(
            capabilities
                .duration_options
                .iter()
                .map(|option| option.value.as_str())
                .collect::<Vec<_>>(),
            vec!["4", "8"]
        );
        assert_eq!(capabilities.image_input_mode, "single");
        assert!(!capabilities.requires_image);
    }

    #[test]
    fn jimeng_capabilities_use_five_and_ten_second_options() {
        let capabilities =
            build_video_model_capabilities(&test_model(9, "Jimeng v2", "jimeng", "jimeng-v2"));

        assert_eq!(capabilities.default_seconds, "5");
        assert_eq!(
            capabilities
                .duration_options
                .iter()
                .map(|option| option.value.as_str())
                .collect::<Vec<_>>(),
            vec!["5", "10"]
        );
        assert_eq!(capabilities.image_input_mode, "start_end");
        assert_eq!(capabilities.max_images, 2);
    }
}
