use crate::schema::gm_user_materials;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

/// User material entity - represents user-uploaded or favorited materials
#[derive(
    Queryable,
    Selectable,
    Insertable,
    AsChangeset,
    Identifiable,
    Debug,
    Clone,
    Serialize,
    Deserialize,
)]
#[diesel(table_name = gm_user_materials)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UserMaterial {
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

/// New user material for insertion
#[derive(Insertable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = gm_user_materials)]
pub struct NewUserMaterial {
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
