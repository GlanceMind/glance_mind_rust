use crate::schema::gm_material_folders;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

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
#[diesel(table_name = gm_material_folders)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct MaterialFolder {
    pub id: i32,
    pub user_id: i32,
    pub parent_id: Option<i32>,
    pub name: String,
    pub depth: i16,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Insertable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = gm_material_folders)]
pub struct NewMaterialFolder {
    pub user_id: i32,
    pub parent_id: Option<i32>,
    pub name: String,
    pub depth: i16,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
}
