use crate::schema::gm_platforms;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Selectable, Identifiable, Serialize, Deserialize, Debug, Clone)]
#[diesel(table_name = gm_platforms)]
pub struct Platform {
    pub id: i32,
    pub name: String,
    pub base_url: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub display_name: String,
    pub page_size: i32,
}

#[derive(Insertable, AsChangeset)]
#[diesel(table_name = gm_platforms)]
pub struct NewPlatform {
    pub name: String,
    pub display_name: String,
    pub is_active: Option<bool>,
}
