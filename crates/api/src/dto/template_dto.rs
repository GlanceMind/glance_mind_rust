use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use validator::Validate;

fn deserialize_nullable_patch<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Deserialize::deserialize(deserializer).map(Some)
}

#[derive(Debug, Serialize, Deserialize, Clone, Validate)]
pub struct TemplateCreateDto {
    pub campaign_id: i32,
    pub library_template_id: Option<i32>,
    pub name: Option<String>,
    pub weight: i32,
    pub dm_prompt: Option<String>,
    pub reply_prompt: Option<String>,
    pub reply_post_prompt: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct TemplateUpdateDto {
    #[serde(default, deserialize_with = "deserialize_nullable_patch")]
    pub name: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_nullable_patch")]
    pub library_template_id: Option<Option<i32>>,
    pub weight: Option<i32>,
    #[serde(default, deserialize_with = "deserialize_nullable_patch")]
    pub dm_prompt: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_nullable_patch")]
    pub reply_prompt: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_nullable_patch")]
    pub reply_post_prompt: Option<Option<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TemplateReadDto {
    pub id: i32,
    pub campaign_id: i32,
    pub library_template_id: Option<i32>,
    pub name: Option<String>,
    pub weight: i32,
    pub dm_prompt: Option<String>,
    pub reply_prompt: Option<String>,
    pub reply_post_prompt: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reusable_template: Option<ReusableTemplateReadDto>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Validate)]
pub struct ReusableTemplateCreateDto {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    pub description: Option<String>,
    pub weight: Option<i32>,
    pub dm_prompt: Option<String>,
    pub reply_prompt: Option<String>,
    pub reply_post_prompt: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Validate)]
pub struct ReusableTemplateUpdateDto {
    #[validate(length(min = 1, max = 255))]
    pub name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_patch")]
    pub description: Option<Option<String>>,
    pub weight: Option<i32>,
    #[serde(default, deserialize_with = "deserialize_nullable_patch")]
    pub dm_prompt: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_nullable_patch")]
    pub reply_prompt: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_nullable_patch")]
    pub reply_post_prompt: Option<Option<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReusableTemplateReadDto {
    pub id: i32,
    pub user_id: i32,
    pub name: String,
    pub description: Option<String>,
    pub weight: i32,
    pub dm_prompt: Option<String>,
    pub reply_prompt: Option<String>,
    pub reply_post_prompt: Option<String>,
    pub usage_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Validate)]
pub struct AssignReusableTemplateDto {
    pub campaign_id: i32,
    pub weight: Option<i32>,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct TemplateBatchCreateDto {
    pub campaign_id: i32,
    pub templates: Vec<TemplateCreateItem>,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TemplateCreateItem {
    pub weight: i32,
    pub dm_prompt: Option<String>,
    pub reply_prompt: Option<String>,
    pub reply_post_prompt: Option<String>,
}

impl From<glance_mind_db::entity::template::CampaignTemplate> for TemplateReadDto {
    fn from(entity: glance_mind_db::entity::template::CampaignTemplate) -> Self {
        Self {
            id: entity.id,
            campaign_id: entity.campaign_id,
            library_template_id: entity.library_template_id,
            name: entity.name,
            weight: entity.weight,
            dm_prompt: entity.dm_prompt,
            reply_prompt: entity.reply_prompt,
            reply_post_prompt: entity.reply_post_prompt,
            created_at: entity.created_at,
            updated_at: entity.updated_at,
            reusable_template: None,
        }
    }
}

impl From<glance_mind_db::entity::template::ResolvedCampaignTemplate> for TemplateReadDto {
    fn from(entity: glance_mind_db::entity::template::ResolvedCampaignTemplate) -> Self {
        Self {
            id: entity.id,
            campaign_id: entity.campaign_id,
            library_template_id: entity.library_template_id,
            name: entity.name,
            weight: entity.weight,
            dm_prompt: entity.dm_prompt,
            reply_prompt: entity.reply_prompt,
            reply_post_prompt: entity.reply_post_prompt,
            created_at: entity.created_at,
            updated_at: entity.updated_at,
            reusable_template: None,
        }
    }
}

impl From<glance_mind_db::entity::template::ReusableReplyTemplate> for ReusableTemplateReadDto {
    fn from(entity: glance_mind_db::entity::template::ReusableReplyTemplate) -> Self {
        Self {
            id: entity.id,
            user_id: entity.user_id,
            name: entity.name,
            description: entity.description,
            weight: entity.weight,
            dm_prompt: entity.dm_prompt,
            reply_prompt: entity.reply_prompt,
            reply_post_prompt: entity.reply_post_prompt,
            usage_count: entity.usage_count,
            created_at: entity.created_at,
            updated_at: entity.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_create_accepts_reusable_library_reference() {
        let payload = r#"{
            "campaign_id": 42,
            "library_template_id": 7,
            "weight": 80
        }"#;

        let dto: TemplateCreateDto = serde_json::from_str(payload).expect("valid template payload");

        assert_eq!(dto.campaign_id, 42);
        assert_eq!(dto.library_template_id, Some(7));
        assert_eq!(dto.weight, 80);
    }

    #[test]
    fn reusable_template_create_requires_a_library_name() {
        let payload = r#"{
            "name": "Friendly launch responder",
            "weight": 55,
            "dm_prompt": "DM with context",
            "reply_prompt": "Reply with context",
            "reply_post_prompt": "Post a short hook"
        }"#;

        let dto: ReusableTemplateCreateDto =
            serde_json::from_str(payload).expect("valid reusable template payload");

        assert_eq!(dto.name, "Friendly launch responder");
        assert_eq!(dto.weight, Some(55));
        assert_eq!(dto.dm_prompt.as_deref(), Some("DM with context"));
        assert_eq!(dto.reply_prompt.as_deref(), Some("Reply with context"));
        assert_eq!(dto.reply_post_prompt.as_deref(), Some("Post a short hook"));
    }

    #[test]
    fn template_update_distinguishes_omitted_null_and_string_prompts() {
        let omitted: TemplateUpdateDto =
            serde_json::from_str(r#"{"weight": 12}"#).expect("valid omitted prompt update");
        let cleared: TemplateUpdateDto = serde_json::from_str(
            r#"{
                "name": null,
                "library_template_id": null,
                "dm_prompt": null,
                "reply_post_prompt": null
            }"#,
        )
        .expect("valid null prompt update");
        let changed: TemplateUpdateDto = serde_json::from_str(
            r#"{
                "name": "fresh name",
                "library_template_id": 44,
                "dm_prompt": "fresh dm",
                "reply_post_prompt": "fresh post"
            }"#,
        )
        .expect("valid string prompt update");

        assert_eq!(omitted.name, None);
        assert_eq!(omitted.library_template_id, None);
        assert_eq!(omitted.dm_prompt, None);
        assert_eq!(omitted.reply_post_prompt, None);
        assert_eq!(cleared.name, Some(None));
        assert_eq!(cleared.library_template_id, Some(None));
        assert_eq!(cleared.dm_prompt, Some(None));
        assert_eq!(cleared.reply_post_prompt, Some(None));
        assert_eq!(changed.name, Some(Some("fresh name".to_string())));
        assert_eq!(changed.library_template_id, Some(Some(44)));
        assert_eq!(changed.dm_prompt, Some(Some("fresh dm".to_string())));
        assert_eq!(
            changed.reply_post_prompt,
            Some(Some("fresh post".to_string()))
        );
    }

    #[test]
    fn reusable_template_update_distinguishes_omitted_null_and_string_prompts() {
        let omitted: ReusableTemplateUpdateDto =
            serde_json::from_str(r#"{"weight": 12}"#).expect("valid omitted prompt update");
        let cleared: ReusableTemplateUpdateDto = serde_json::from_str(
            r#"{
                "description": null,
                "reply_prompt": null,
                "dm_prompt": null
            }"#,
        )
        .expect("valid null prompt update");
        let changed: ReusableTemplateUpdateDto = serde_json::from_str(
            r#"{
                "description": "fresh description",
                "reply_prompt": "fresh reply",
                "dm_prompt": "fresh dm"
            }"#,
        )
        .expect("valid string prompt update");

        assert_eq!(omitted.description, None);
        assert_eq!(omitted.reply_prompt, None);
        assert_eq!(omitted.dm_prompt, None);
        assert_eq!(cleared.description, Some(None));
        assert_eq!(cleared.reply_prompt, Some(None));
        assert_eq!(cleared.dm_prompt, Some(None));
        assert_eq!(
            changed.description,
            Some(Some("fresh description".to_string()))
        );
        assert_eq!(changed.reply_prompt, Some(Some("fresh reply".to_string())));
        assert_eq!(changed.dm_prompt, Some(Some("fresh dm".to_string())));
    }
}
