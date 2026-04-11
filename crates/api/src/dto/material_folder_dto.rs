use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateFolderRequest {
    #[validate(length(min = 1, max = 255, message = "Folder name is required (1-255 chars)"))]
    pub name: String,
    pub parent_id: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UpdateFolderRequest {
    #[validate(length(min = 1, max = 255, message = "Folder name must be 1-255 chars"))]
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FolderListQuery {
    pub parent_id: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderDetail {
    pub id: i32,
    pub user_id: i32,
    pub parent_id: Option<i32>,
    pub name: String,
    pub depth: i16,
    pub sort_order: i32,
    pub material_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderTreeNode {
    pub id: i32,
    pub name: String,
    pub parent_id: Option<i32>,
    pub depth: i16,
    pub material_count: i64,
    pub children: Vec<FolderTreeNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderTreeResponse {
    pub folders: Vec<FolderTreeNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderListResponse {
    pub folders: Vec<FolderDetail>,
}

impl From<glance_mind_db::entity::material_folder::MaterialFolder> for FolderDetail {
    fn from(f: glance_mind_db::entity::material_folder::MaterialFolder) -> Self {
        Self {
            id: f.id,
            user_id: f.user_id,
            parent_id: f.parent_id,
            name: f.name,
            depth: f.depth,
            sort_order: f.sort_order,
            material_count: 0,
            created_at: f.created_at,
            updated_at: f.updated_at,
        }
    }
}
