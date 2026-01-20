use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NotificationDto {
    pub id: i32,
    pub notification_type: String,
    pub title: String,
    pub title_zh: Option<String>,
    pub description: Option<String>,
    pub description_zh: Option<String>,
    pub link: Option<String>,
    pub link_text: Option<String>,
    pub link_text_zh: Option<String>,
    pub important: bool,
    pub published_at: Option<DateTime<Utc>>,
    pub is_read: bool,
}

impl NotificationDto {
    pub fn from_entity(
        entity: glance_mind_db::entity::notification::Notification,
        is_read: bool,
    ) -> Self {
        Self {
            id: entity.id,
            notification_type: entity.notification_type,
            title: entity.title,
            title_zh: entity.title_zh,
            description: entity.description,
            description_zh: entity.description_zh,
            link: entity.link,
            link_text: entity.link_text,
            link_text_zh: entity.link_text_zh,
            important: entity.important.unwrap_or(false),
            published_at: entity.published_at,
            is_read,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UnreadCountDto {
    pub count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MarkReadRequest {
    pub notification_ids: Option<Vec<i32>>,
}
