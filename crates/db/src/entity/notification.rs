use crate::schema::{gm_notifications, gm_user_notification_reads};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Selectable, Identifiable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = gm_notifications)]
pub struct Notification {
    pub id: i32,
    pub notification_type: String,
    pub title: String,
    pub title_zh: Option<String>,
    pub description: Option<String>,
    pub description_zh: Option<String>,
    pub link: Option<String>,
    pub link_text: Option<String>,
    pub link_text_zh: Option<String>,
    pub important: Option<bool>,
    pub published_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Insertable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = gm_notifications)]
pub struct NewNotification {
    pub notification_type: String,
    pub title: String,
    pub title_zh: Option<String>,
    pub description: Option<String>,
    pub description_zh: Option<String>,
    pub link: Option<String>,
    pub link_text: Option<String>,
    pub link_text_zh: Option<String>,
    pub important: Option<bool>,
    pub published_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Queryable, Selectable, Identifiable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = gm_user_notification_reads)]
pub struct UserNotificationRead {
    pub id: i32,
    pub user_id: i32,
    pub notification_id: i32,
    pub read_at: Option<DateTime<Utc>>,
}

#[derive(Insertable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = gm_user_notification_reads)]
pub struct NewUserNotificationRead {
    pub user_id: i32,
    pub notification_id: i32,
}
