use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Clone, Validate)]
pub struct TemplateCreateDto {
    pub campaign_id: i32,
    pub name: Option<String>,
    pub weight: i32,
    pub dm_prompt: Option<String>,
    pub reply_prompt: Option<String>,
    pub reply_post_prompt: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct TemplateUpdateDto {
    pub name: Option<String>,
    pub weight: Option<i32>,
    pub dm_prompt: Option<String>,
    pub reply_prompt: Option<String>,
    pub reply_post_prompt: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TemplateReadDto {
    pub id: i32,
    pub campaign_id: i32,
    pub name: Option<String>,
    pub weight: i32,
    pub dm_prompt: Option<String>,
    pub reply_prompt: Option<String>,
    pub reply_post_prompt: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
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
            name: entity.name,
            weight: entity.weight,
            dm_prompt: entity.dm_prompt,
            reply_prompt: entity.reply_prompt,
            reply_post_prompt: entity.reply_post_prompt,
            created_at: entity.created_at,
            updated_at: entity.updated_at,
        }
    }
}
