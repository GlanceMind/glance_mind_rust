use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::schema::gm_login_logs;

/// Login log entity
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = gm_login_logs)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct LoginLog {
    pub id: i32,
    pub user_id: i32,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub login_status: String,
    pub failure_reason: Option<String>,
    pub login_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// New login log record
#[derive(Debug, Clone, Insertable, Serialize, Deserialize)]
#[diesel(table_name = gm_login_logs)]
pub struct NewLoginLog {
    pub user_id: i32,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub login_status: String,
    pub failure_reason: Option<String>,
}

impl NewLoginLog {
    /// Create successful login log
    pub fn success(user_id: i32, ip_address: Option<String>, user_agent: Option<String>) -> Self {
        Self {
            user_id,
            ip_address,
            user_agent,
            login_status: "SUCCESS".to_string(),
            failure_reason: None,
        }
    }

    /// Create failed login log
    pub fn failed(
        user_id: i32,
        ip_address: Option<String>,
        user_agent: Option<String>,
        reason: String,
    ) -> Self {
        Self {
            user_id,
            ip_address,
            user_agent,
            login_status: "FAILED".to_string(),
            failure_reason: Some(reason),
        }
    }
}

/// Login log DTO (for API response)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct LoginLogDto {
    pub id: i32,
    pub user_id: i32,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub login_status: String,
    pub failure_reason: Option<String>,
    pub login_at: DateTime<Utc>,
}

impl From<LoginLog> for LoginLogDto {
    fn from(log: LoginLog) -> Self {
        Self {
            id: log.id,
            user_id: log.user_id,
            ip_address: log.ip_address,
            user_agent: log.user_agent,
            login_status: log.login_status,
            failure_reason: log.failure_reason,
            login_at: log.login_at,
        }
    }
}
