use crate::schema::gm_campaign_templates;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use super::campaign::Campaign;

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
    pub weight: i32,
    pub reply_prompt: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub dm_template: Option<String>,
    pub dm_prompt: Option<String>,
    pub reply_post_prompt: Option<String>,
    pub name: Option<String>,
}

#[derive(Insertable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = gm_campaign_templates)]
pub struct NewCampaignTemplate {
    pub campaign_id: i32,
    pub weight: i32,
    pub reply_prompt: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub dm_template: Option<String>,
    pub dm_prompt: Option<String>,
    pub reply_post_prompt: Option<String>,
    pub name: Option<String>,
}
