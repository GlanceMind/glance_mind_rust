use crate::schema::gm_social_accounts;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Selectable, Identifiable, Serialize, Deserialize, Debug, Clone)]
#[diesel(table_name = gm_social_accounts)]
pub struct SocialAccount {
    pub id: i32,
    pub user_id: i32,
    pub platform_id: i32,
    pub username: String,
    pub cookie: String,
    pub proxy_url: Option<String>,
    pub status: String,
    pub health_score: Option<i32>,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
    pub group_id: Option<i32>,
    pub daily_max_replies: i32,
    pub device_id: Option<String>,
    pub profile_name: Option<String>,
    pub fb_pages_id: Option<String>,
}

#[derive(AsChangeset, Deserialize, Serialize, Debug, Clone)]
#[diesel(table_name = gm_social_accounts)]
pub struct UpdateSocialAccount {
    pub platform_id: Option<i32>,
    pub group_id: Option<Option<i32>>,
    pub username: Option<String>,
    pub cookie: Option<String>,
    pub proxy_url: Option<Option<String>>,
    pub status: Option<String>,
    pub health_score: Option<i32>,
    pub updated_at: Option<NaiveDateTime>,
    pub daily_max_replies: Option<i32>,
    pub device_id: Option<Option<String>>,
    pub profile_name: Option<Option<String>>,
    pub fb_pages_id: Option<Option<String>>,
}

#[derive(Debug, Insertable, AsChangeset)]
#[diesel(table_name = gm_social_accounts)]
pub struct NewSocialAccount {
    pub user_id: i32,
    pub platform_id: i32,
    pub group_id: Option<i32>,
    pub username: String,
    pub cookie: String,
    pub proxy_url: Option<String>,
    pub status: String,
    pub health_score: Option<i32>,
    pub daily_max_replies: i32,
    pub device_id: Option<String>,
    pub profile_name: Option<String>,
    pub fb_pages_id: Option<String>,
}
