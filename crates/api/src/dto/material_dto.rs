//! Material Management DTOs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

/// Create material request (user upload)
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateMaterialRequest {
    #[validate(length(min = 1, message = "Video URL is required"))]
    pub video_url: String,
    #[validate(length(min = 1, message = "Tag is required"))]
    pub tag: String,
    #[validate(length(min = 1, message = "Title is required"))]
    pub title: String,
    pub description: Option<String>,
}

/// Update material request
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UpdateMaterialRequest {
    pub video_url: Option<String>,
    pub tag: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
}

/// Material list item response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialListItem {
    pub id: i32,
    pub user_id: i32,
    pub video_url: String,
    pub prompt: Option<String>, // AI-generated prompt
    pub thumbnail_url: Option<String>,
    pub tag: Option<String>, // Single tag for categorization
    pub title: Option<String>,
    pub description: Option<String>,
    pub duration: Option<i32>,
    pub file_size: Option<i64>,
    pub created_at: DateTime<Utc>,
}

/// Material detail response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialDetail {
    pub id: i32,
    pub user_id: i32,
    pub video_url: String,
    pub prompt: Option<String>,
    pub thumbnail_url: Option<String>,
    pub tag: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub duration: Option<i32>,
    pub file_size: Option<i64>,
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
}

/// Conversion from entity to DTO
impl From<glance_mind_db::entity::material::UserMaterial> for MaterialListItem {
    fn from(material: glance_mind_db::entity::material::UserMaterial) -> Self {
        Self {
            id: material.id,
            user_id: material.user_id,
            video_url: material.video_url,
            prompt: material.prompt,
            thumbnail_url: material.thumbnail_url,
            tag: material.tag,
            title: material.title,
            description: material.description,
            duration: material.duration,
            file_size: material.file_size,
            created_at: material.created_at,
        }
    }
}

impl From<glance_mind_db::entity::material::UserMaterial> for MaterialDetail {
    fn from(material: glance_mind_db::entity::material::UserMaterial) -> Self {
        Self {
            id: material.id,
            user_id: material.user_id,
            video_url: material.video_url,
            prompt: material.prompt,
            thumbnail_url: material.thumbnail_url,
            tag: material.tag,
            title: material.title,
            description: material.description,
            duration: material.duration,
            file_size: material.file_size,
            is_active: material.is_active,
            created_at: material.created_at,
            updated_at: material.updated_at,
        }
    }
}
