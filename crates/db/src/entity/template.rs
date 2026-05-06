use crate::schema::{gm_campaign_templates, gm_reply_template_library};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::sql_types::{Integer, Nullable, Text, Timestamptz};
use serde::{Deserialize, Serialize};

use super::campaign::Campaign;
use super::user::User;

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
    Associations,
)]
#[diesel(belongs_to(Campaign))]
#[diesel(table_name = gm_campaign_templates)]
pub struct CampaignTemplate {
    pub id: i32,
    pub campaign_id: i32,
    pub library_template_id: Option<i32>,
    pub weight: i32,
    pub reply_prompt: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub dm_prompt: Option<String>,
    pub reply_post_prompt: Option<String>,
    pub name: Option<String>,
}

#[derive(Insertable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = gm_campaign_templates)]
pub struct NewCampaignTemplate {
    pub campaign_id: i32,
    pub library_template_id: Option<i32>,
    pub weight: i32,
    pub reply_prompt: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub dm_prompt: Option<String>,
    pub reply_post_prompt: Option<String>,
    pub name: Option<String>,
}

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
    Associations,
)]
#[diesel(belongs_to(User))]
#[diesel(table_name = gm_reply_template_library)]
pub struct ReusableReplyTemplate {
    pub id: i32,
    pub user_id: i32,
    pub name: String,
    pub description: Option<String>,
    pub weight: i32,
    pub dm_prompt: Option<String>,
    pub reply_prompt: Option<String>,
    pub reply_post_prompt: Option<String>,
    pub usage_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Insertable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = gm_reply_template_library)]
pub struct NewReusableReplyTemplate {
    pub user_id: i32,
    pub name: String,
    pub description: Option<String>,
    pub weight: i32,
    pub dm_prompt: Option<String>,
    pub reply_prompt: Option<String>,
    pub reply_post_prompt: Option<String>,
    pub usage_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(QueryableByName, Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedCampaignTemplate {
    #[diesel(sql_type = Integer)]
    pub id: i32,
    #[diesel(sql_type = Integer)]
    pub campaign_id: i32,
    #[diesel(sql_type = Nullable<Integer>)]
    pub library_template_id: Option<i32>,
    #[diesel(sql_type = Integer)]
    pub weight: i32,
    #[diesel(sql_type = Nullable<Text>)]
    pub reply_prompt: Option<String>,
    #[diesel(sql_type = Timestamptz)]
    pub created_at: DateTime<Utc>,
    #[diesel(sql_type = Nullable<Timestamptz>)]
    pub updated_at: Option<DateTime<Utc>>,
    #[diesel(sql_type = Nullable<Text>)]
    pub dm_prompt: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub reply_post_prompt: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub name: Option<String>,
}

#[derive(QueryableByName, Debug, Clone, Serialize, Deserialize)]
pub struct AssignedCampaignTemplate {
    #[diesel(sql_type = Integer)]
    pub id: i32,
    #[diesel(sql_type = Integer)]
    pub campaign_id: i32,
    #[diesel(sql_type = Nullable<Integer>)]
    pub library_template_id: Option<i32>,
    #[diesel(sql_type = Integer)]
    pub weight: i32,
    #[diesel(sql_type = Nullable<Text>)]
    pub reply_prompt: Option<String>,
    #[diesel(sql_type = Timestamptz)]
    pub created_at: DateTime<Utc>,
    #[diesel(sql_type = Nullable<Timestamptz>)]
    pub updated_at: Option<DateTime<Utc>>,
    #[diesel(sql_type = Nullable<Text>)]
    pub dm_prompt: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub reply_post_prompt: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub name: Option<String>,
}

impl From<AssignedCampaignTemplate> for CampaignTemplate {
    fn from(row: AssignedCampaignTemplate) -> Self {
        Self {
            id: row.id,
            campaign_id: row.campaign_id,
            library_template_id: row.library_template_id,
            weight: row.weight,
            reply_prompt: row.reply_prompt,
            created_at: row.created_at,
            updated_at: row.updated_at,
            dm_prompt: row.dm_prompt,
            reply_post_prompt: row.reply_post_prompt,
            name: row.name,
        }
    }
}
