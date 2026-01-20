use crate::schema::gm_email_verifications;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Selectable, Identifiable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = gm_email_verifications)]
pub struct EmailVerification {
    pub id: i32,
    pub email: String,
    pub code: String,
    pub expires_at: DateTime<Utc>,
    pub verified: bool,
    pub created_at: DateTime<Utc>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

#[derive(Insertable, Serialize, Deserialize)]
#[diesel(table_name = gm_email_verifications)]
pub struct NewEmailVerification {
    pub email: String,
    pub code: String,
    pub expires_at: DateTime<Utc>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}
