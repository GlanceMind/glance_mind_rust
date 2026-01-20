use crate::schema::gm_social_groups;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Selectable, Identifiable, Serialize, Deserialize, Debug, Clone)]
#[diesel(table_name = gm_social_groups)]
pub struct SocialGroup {
    pub id: i32,
    pub user_id: i32,
    pub platform_id: i32,
    pub group_name: String,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Insertable, AsChangeset)]
#[diesel(table_name = gm_social_groups)]
pub struct NewSocialGroup {
    pub user_id: i32,
    pub platform_id: i32,
    pub group_name: String,
}
