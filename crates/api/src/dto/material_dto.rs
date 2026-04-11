//! Material Management DTOs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

/// Create material request (user upload)
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateMaterialRequest {
    pub video_url: Option<String>,
    pub file_url: Option<String>,
    pub tag: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub folder_id: Option<i32>,
    pub media_type: Option<String>,
    pub mime_type: Option<String>,
}

/// Update material request
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UpdateMaterialRequest {
    pub video_url: Option<String>,
    pub tag: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub folder_id: Option<Option<i32>>,
}

/// Material list item response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialListItem {
    pub id: i32,
    pub user_id: i32,
    pub video_url: Option<String>,
    pub file_url: Option<String>,
    pub media_type: String,
    pub mime_type: Option<String>,
    pub prompt: Option<String>,
    pub thumbnail_url: Option<String>,
    pub tag: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub duration: Option<i32>,
    pub file_size: Option<i64>,
    pub folder_id: Option<i32>,
    pub created_at: DateTime<Utc>,
}

/// Material detail response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialDetail {
    pub id: i32,
    pub user_id: i32,
    pub video_url: Option<String>,
    pub file_url: Option<String>,
    pub media_type: String,
    pub mime_type: Option<String>,
    pub prompt: Option<String>,
    pub thumbnail_url: Option<String>,
    pub tag: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub duration: Option<i32>,
    pub file_size: Option<i64>,
    pub folder_id: Option<i32>,
    pub is_active: Option<bool>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

/// Material list response with pagination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialListResponse {
    pub list: Vec<MaterialListItem>,
    pub total: i64,
    pub page: i32,
    pub page_size: i32,
}

/// Material tag response (collected from video_cases)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialTag {
    pub name: String, // category_name_cn or category_name_en
    pub name_cn: Option<String>,
    pub name_en: Option<String>,
    pub usage_count: i64, // Count of materials using this tag
}

/// Material tags list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialTagsResponse {
    pub tags: Vec<MaterialTag>,
}

/// Query parameters for material list
#[derive(Debug, Deserialize)]
pub struct MaterialListQuery {
    pub page: Option<i32>,
    pub page_size: Option<i32>,
    pub tag: Option<String>,
    pub search: Option<String>,
    pub folder_id: Option<String>,
    pub media_type: Option<String>,
}

/// Conversion from entity to DTO
impl From<glance_mind_db::entity::material::UserMaterial> for MaterialListItem {
    fn from(m: glance_mind_db::entity::material::UserMaterial) -> Self {
        Self {
            id: m.id,
            user_id: m.user_id,
            video_url: m.video_url,
            file_url: m.file_url,
            media_type: m.media_type,
            mime_type: m.mime_type,
            prompt: m.prompt,
            thumbnail_url: m.thumbnail_url,
            tag: m.tag,
            title: m.title,
            description: m.description,
            duration: m.duration,
            file_size: m.file_size,
            folder_id: m.folder_id,
            created_at: m.created_at,
        }
    }
}

impl From<glance_mind_db::entity::material::UserMaterial> for MaterialDetail {
    fn from(m: glance_mind_db::entity::material::UserMaterial) -> Self {
        Self {
            id: m.id,
            user_id: m.user_id,
            video_url: m.video_url,
            file_url: m.file_url,
            media_type: m.media_type,
            mime_type: m.mime_type,
            prompt: m.prompt,
            thumbnail_url: m.thumbnail_url,
            tag: m.tag,
            title: m.title,
            description: m.description,
            duration: m.duration,
            file_size: m.file_size,
            folder_id: m.folder_id,
            is_active: m.is_active,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}
