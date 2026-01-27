use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

// Social Group DTOs
#[derive(Debug, Serialize, Deserialize)]
pub struct SocialGroupDto {
    pub id: i32,
    pub user_id: i32,
    pub platform_id: i32,
    pub group_name: String,
    pub accounts: Option<Vec<SocialAccountDto>>,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSocialGroupDto {
    pub platform_id: i32,
    pub group_name: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSocialGroupDto {
    pub group_name: String,
}

// Social Account DTOs
#[derive(Debug, Deserialize)]
pub struct CreateSocialAccountDto {
    pub username: String,
    #[serde(default)]
    pub cookie: Option<String>,
    pub proxy_url: Option<String>,
    pub daily_max_replies: Option<i32>, // Optional, defaults to 50
    #[serde(default)]
    pub device_id: Option<String>,
    #[serde(default)]
    pub profile_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSocialAccountDto {
    pub username: Option<String>,
    pub cookie: Option<String>,
    pub proxy_url: Option<String>,
    pub status: Option<String>,
    pub group_id: Option<i32>,
    pub daily_max_replies: Option<i32>,
    pub device_id: Option<String>,
    pub profile_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SocialAccountDto {
    pub id: i32,
    pub platform_id: i32,
    pub group_id: Option<i32>,
    pub username: String,
    pub cookie: Option<String>,
    pub proxy_url: Option<String>,
    pub status: String,
    pub health_score: Option<i32>,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
    pub daily_max_replies: i32,
    pub device_id: Option<String>,
    pub profile_name: Option<String>,
}

// Account Statistics DTO
#[derive(Debug, Serialize, Deserialize)]
pub struct AccountStatisticsDto {
    pub total: i64,
    pub active: i64,
    pub risk_control: i64,
    pub unavailable: i64,
}
