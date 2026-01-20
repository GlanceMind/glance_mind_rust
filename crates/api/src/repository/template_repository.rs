use crate::config::database::DBPool;
use glance_mind_db::entity::template::{CampaignTemplate, NewCampaignTemplate};
use glance_mind_db::schema::gm_campaign_templates as campaign_templates;
use diesel::prelude::*;
use diesel::result::Error as DieselError;

#[derive(Clone)]
pub struct TemplateRepository {
    pool: DBPool,
}

impl TemplateRepository {
    pub fn new(pool: DBPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        campaign_id: i32,
        weight: i32,
        dm_prompt: Option<String>,
        reply_prompt: Option<String>,
        reply_post_prompt: Option<String>,
    ) -> Result<CampaignTemplate, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        let new_template = NewCampaignTemplate {
            campaign_id,
            weight,
            dm_prompt,
            reply_prompt,
            reply_post_prompt,
            created_at: chrono::Utc::now(),
            updated_at: None,
        };

        diesel::insert_into(campaign_templates::table)
            .values(&new_template)
            .returning(CampaignTemplate::as_returning())
            .get_result(&mut conn)
    }

    pub async fn find_all_by_campaign(
        &self,
        campaign_id: i32,
        page: i64,
        page_size: i64,
    ) -> Result<(Vec<CampaignTemplate>, i64), DieselError> {
        let mut conn = self.pool.get().expect("Connection error");

        let total = campaign_templates::table
            .filter(campaign_templates::campaign_id.eq(campaign_id))
            .count()
            .get_result(&mut conn)?;

        let items = campaign_templates::table
            .filter(campaign_templates::campaign_id.eq(campaign_id))
            .select(CampaignTemplate::as_select())
            .order(campaign_templates::created_at.desc())
            .limit(page_size)
            .offset((page - 1) * page_size)
            .load(&mut conn)?;

        Ok((items, total))
    }

    pub async fn find_all_by_user(
        &self,
        user_id: i32,
        page: i64,
        page_size: i64,
    ) -> Result<(Vec<CampaignTemplate>, i64), DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        use glance_mind_db::schema::gm_campaigns as campaigns;

        let total = campaign_templates::table
            .inner_join(campaigns::table)
            .filter(campaigns::user_id.eq(user_id))
            .count()
            .get_result(&mut conn)?;

        let items = campaign_templates::table
            .inner_join(campaigns::table)
            .filter(campaigns::user_id.eq(user_id))
            .select(CampaignTemplate::as_select())
            .order(campaign_templates::created_at.desc())
            .limit(page_size)
            .offset((page - 1) * page_size)
            .load(&mut conn)?;

        Ok((items, total))
    }

    pub async fn find_by_id(&self, id: i32) -> Result<CampaignTemplate, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        campaign_templates::table
            .find(id)
            .select(CampaignTemplate::as_select())
            .first(&mut conn)
    }

    pub async fn update(
        &self,
        id: i32,
        weight: Option<i32>,
        dm_prompt: Option<String>,
        reply_prompt: Option<String>,
        reply_post_prompt: Option<String>,
    ) -> Result<CampaignTemplate, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");

        // Fetch first to update selectively.
        let target = campaign_templates::table
            .find(id)
            .select(CampaignTemplate::as_select());
        let mut template = target.first::<CampaignTemplate>(&mut conn)?;

        if let Some(w) = weight {
            template.weight = w;
        }
        if let Some(dm) = dm_prompt {
            template.dm_prompt = Some(dm);
        }
        if let Some(rp) = reply_prompt {
            template.reply_prompt = Some(rp);
        }
        if let Some(rpp) = reply_post_prompt {
            template.reply_post_prompt = Some(rpp);
        }
        template.updated_at = Some(chrono::Utc::now());

        diesel::update(campaign_templates::table.find(id))
            .set(&template)
            .returning(CampaignTemplate::as_returning())
            .get_result(&mut conn)
    }

    pub async fn delete(&self, id: i32) -> Result<usize, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        diesel::delete(campaign_templates::table.find(id)).execute(&mut conn)
    }
}
