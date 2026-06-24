use crate::schema::gm_users;
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
#[diesel(table_name = gm_users)]
pub struct User {
    pub id: i32,
    pub email: Option<String>,
    pub password_hash: String,
    pub invitation_code: Option<String>,
    pub referred_by: Option<String>,
    pub company_name: Option<String>,
    pub api_key: Option<String>,
    pub status: String,
    pub full_name: String,
    pub role: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub username: Option<String>,
    pub permissions: i64,
    pub phone: Option<String>,
}

#[derive(Insertable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = gm_users)]
pub struct NewUser {
    pub email: Option<String>,
    pub password_hash: String,
    pub invitation_code: Option<String>,
    pub referred_by: Option<String>,
    pub company_name: Option<String>,
    pub api_key: Option<String>,
    pub status: String,
    pub full_name: String,
    pub role: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub username: Option<String>,
    pub permissions: Option<i64>,
    pub phone: Option<String>,
}
