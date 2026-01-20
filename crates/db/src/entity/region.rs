use crate::schema::gm_regions;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use super::platform::Platform;

#[derive(
    Queryable, Selectable, Identifiable, Serialize, Deserialize, Debug, Clone, Associations,
)]
#[diesel(belongs_to(Platform))]
#[diesel(table_name = gm_regions)]
pub struct Region {
    pub id: i32,
    pub name: String,
    pub code: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub platform_id: i32,
    pub display_name: String,
}

#[derive(Insertable, AsChangeset)]
#[diesel(table_name = gm_regions)]
pub struct NewRegion {
    pub platform_id: i32,
    pub code: String,
    pub display_name: String,
    pub is_active: Option<bool>,
}
