use crate::config::database::DBPool;
use diesel::prelude::*;
use diesel::result::Error as DieselError;
use diesel::sql_types::{Array, BigInt, Integer, Nullable, Text, Timestamptz};
use glance_mind_db::entity::template::{
    AssignedCampaignTemplate, CampaignTemplate, NewCampaignTemplate, NewReusableReplyTemplate,
    ResolvedCampaignTemplate, ReusableReplyTemplate,
};
use glance_mind_db::schema::gm_campaign_templates as campaign_templates;
use glance_mind_db::schema::gm_reply_template_library as template_library;
use std::collections::HashMap;

#[derive(Debug, QueryableByName)]
struct CountRow {
    #[diesel(sql_type = BigInt)]
    total: i64,
}

#[derive(Debug, QueryableByName)]
struct ReusableUsageRow {
    #[diesel(sql_type = Integer)]
    id: i32,
    #[diesel(sql_type = BigInt)]
    usage_count: i64,
}

#[derive(Debug, QueryableByName)]
struct ReusableTemplateRow {
    #[diesel(sql_type = Integer)]
    id: i32,
    #[diesel(sql_type = Integer)]
    user_id: i32,
    #[diesel(sql_type = Text)]
    name: String,
    #[diesel(sql_type = Nullable<Text>)]
    description: Option<String>,
    #[diesel(sql_type = Integer)]
    weight: i32,
    #[diesel(sql_type = Nullable<Text>)]
    dm_prompt: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    reply_prompt: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    reply_post_prompt: Option<String>,
    #[diesel(sql_type = BigInt)]
    usage_count: i64,
    #[diesel(sql_type = Timestamptz)]
    created_at: chrono::DateTime<chrono::Utc>,
    #[diesel(sql_type = Nullable<Timestamptz>)]
    updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, QueryableByName)]
struct DeletedReusableTemplateRow {
    #[diesel(sql_type = Integer)]
    id: i32,
}

impl From<ReusableTemplateRow> for ReusableReplyTemplate {
    fn from(row: ReusableTemplateRow) -> Self {
        Self {
            id: row.id,
            user_id: row.user_id,
            name: row.name,
            description: row.description,
            weight: row.weight,
            dm_prompt: row.dm_prompt,
            reply_prompt: row.reply_prompt,
            reply_post_prompt: row.reply_post_prompt,
            usage_count: row.usage_count as i32,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[derive(Clone)]
pub struct TemplateRepository {
    pool: DBPool,
}

#[derive(Debug)]
pub enum AssignReusableTemplateError {
    CampaignNotFound,
    TemplateNotFound,
    Diesel(DieselError),
    ReplyTemplateIdsFull,
}

impl From<DieselError> for AssignReusableTemplateError {
    fn from(error: DieselError) -> Self {
        Self::Diesel(error)
    }
}

#[derive(Debug)]
pub enum CampaignTemplateWriteError {
    Diesel(DieselError),
    ReplyTemplateIdsFull,
}

impl From<DieselError> for CampaignTemplateWriteError {
    fn from(error: DieselError) -> Self {
        Self::Diesel(error)
    }
}

enum ReplyTemplateAppendResult {
    Appended,
    AlreadyPresent,
}

pub struct CampaignTemplateInsert {
    pub campaign_id: i32,
    pub library_template_id: Option<i32>,
    pub name: Option<String>,
    pub weight: i32,
    pub dm_prompt: Option<String>,
    pub reply_prompt: Option<String>,
    pub reply_post_prompt: Option<String>,
}

pub struct CampaignTemplatePatch {
    pub id: i32,
    pub library_template_id: Option<Option<i32>>,
    pub name: Option<Option<String>>,
    pub weight: Option<i32>,
    pub dm_prompt: Option<Option<String>>,
    pub reply_prompt: Option<Option<String>>,
    pub reply_post_prompt: Option<Option<String>>,
}

pub struct ReusableTemplateInsert {
    pub user_id: i32,
    pub name: String,
    pub description: Option<String>,
    pub weight: i32,
    pub dm_prompt: Option<String>,
    pub reply_prompt: Option<String>,
    pub reply_post_prompt: Option<String>,
}

pub struct ReusableTemplatePatch {
    pub id: i32,
    pub user_id: i32,
    pub name: Option<String>,
    pub description: Option<Option<String>>,
    pub weight: Option<i32>,
    pub dm_prompt: Option<Option<String>>,
    pub reply_prompt: Option<Option<String>>,
    pub reply_post_prompt: Option<Option<String>>,
}

impl TemplateRepository {
    pub fn new(pool: DBPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        input: CampaignTemplateInsert,
    ) -> Result<CampaignTemplate, CampaignTemplateWriteError> {
        let mut conn = self.pool.get().expect("Connection error");
        let new_template = NewCampaignTemplate {
            campaign_id: input.campaign_id,
            library_template_id: input.library_template_id,
            weight: input.weight,
            reply_prompt: input.reply_prompt,
            created_at: chrono::Utc::now(),
            updated_at: None,
            dm_prompt: input.dm_prompt,
            reply_post_prompt: input.reply_post_prompt,
            name: input.name,
        };

        conn.transaction(|conn| {
            let template = diesel::insert_into(campaign_templates::table)
                .values(&new_template)
                .returning(CampaignTemplate::as_returning())
                .get_result::<CampaignTemplate>(conn)?;

            if let Some(library_template_id) = template.library_template_id {
                Self::append_reply_template_id(conn, template.campaign_id, library_template_id)?;
            }

            Ok(template)
        })
    }

    pub async fn find_all_by_campaign(
        &self,
        campaign_id: i32,
        page: i64,
        page_size: i64,
    ) -> Result<(Vec<ResolvedCampaignTemplate>, i64), DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        let offset = (page - 1).max(0) * page_size;

        let total = diesel::sql_query(
            r#"
            SELECT COUNT(*) AS total
            FROM gm_campaign_templates
            WHERE campaign_id = $1
            "#,
        )
        .bind::<Integer, _>(campaign_id)
        .get_result::<CountRow>(&mut conn)?
        .total;

        let items = diesel::sql_query(
            r#"
            SELECT
                id,
                campaign_id,
                library_template_id,
                weight,
                reply_prompt,
                created_at,
                updated_at,
                dm_prompt,
                reply_post_prompt,
                name
            FROM gm_resolved_campaign_templates
            WHERE campaign_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind::<Integer, _>(campaign_id)
        .bind::<BigInt, _>(page_size)
        .bind::<BigInt, _>(offset)
        .load::<ResolvedCampaignTemplate>(&mut conn)?;

        Ok((items, total))
    }

    pub async fn find_all_by_user(
        &self,
        user_id: i32,
        page: i64,
        page_size: i64,
    ) -> Result<(Vec<ResolvedCampaignTemplate>, i64), DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        let offset = (page - 1).max(0) * page_size;

        let total = diesel::sql_query(
            r#"
            SELECT COUNT(*) AS total
            FROM gm_resolved_campaign_templates resolved
            JOIN gm_campaigns campaigns ON campaigns.id = resolved.campaign_id
            WHERE campaigns.user_id = $1
            "#,
        )
        .bind::<Integer, _>(user_id)
        .get_result::<CountRow>(&mut conn)?
        .total;

        let items = diesel::sql_query(
            r#"
            SELECT
                resolved.id,
                resolved.campaign_id,
                resolved.library_template_id,
                resolved.weight,
                resolved.reply_prompt,
                resolved.created_at,
                resolved.updated_at,
                resolved.dm_prompt,
                resolved.reply_post_prompt,
                resolved.name
            FROM gm_resolved_campaign_templates resolved
            JOIN gm_campaigns campaigns ON campaigns.id = resolved.campaign_id
            WHERE campaigns.user_id = $1
            ORDER BY resolved.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind::<Integer, _>(user_id)
        .bind::<BigInt, _>(page_size)
        .bind::<BigInt, _>(offset)
        .load::<ResolvedCampaignTemplate>(&mut conn)?;

        Ok((items, total))
    }

    pub async fn find_resolved_by_id(
        &self,
        id: i32,
    ) -> Result<ResolvedCampaignTemplate, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        diesel::sql_query(
            r#"
            SELECT
                id,
                campaign_id,
                library_template_id,
                weight,
                reply_prompt,
                created_at,
                updated_at,
                dm_prompt,
                reply_post_prompt,
                name
            FROM gm_resolved_campaign_templates
            WHERE id = $1
            "#,
        )
        .bind::<Integer, _>(id)
        .get_result(&mut conn)
    }

    pub async fn find_by_id(&self, id: i32) -> Result<CampaignTemplate, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        campaign_templates::table
            .find(id)
            .select(CampaignTemplate::as_select())
            .first(&mut conn)
    }

    pub async fn find_by_campaign_and_library(
        &self,
        campaign_id: i32,
        library_template_id: i32,
    ) -> Result<CampaignTemplate, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        campaign_templates::table
            .filter(campaign_templates::campaign_id.eq(campaign_id))
            .filter(campaign_templates::library_template_id.eq(Some(library_template_id)))
            .select(CampaignTemplate::as_select())
            .first(&mut conn)
    }

    pub async fn update(
        &self,
        input: CampaignTemplatePatch,
    ) -> Result<CampaignTemplate, CampaignTemplateWriteError> {
        let mut conn = self.pool.get().expect("Connection error");

        conn.transaction(|conn| {
            let target = campaign_templates::table
                .find(input.id)
                .for_update()
                .select(CampaignTemplate::as_select());
            let mut template = target.first::<CampaignTemplate>(conn)?;
            let previous_library_template_id = template.library_template_id;

            if let Some(library_id) = input.library_template_id {
                template.library_template_id = library_id;
            }
            if let Some(n) = input.name {
                template.name = n;
            }
            if let Some(w) = input.weight {
                template.weight = w;
            }
            if let Some(dm) = input.dm_prompt {
                template.dm_prompt = dm;
            }
            if let Some(rp) = input.reply_prompt {
                template.reply_prompt = rp;
            }
            if let Some(rpp) = input.reply_post_prompt {
                template.reply_post_prompt = rpp;
            }
            template.updated_at = Some(chrono::Utc::now());

            let updated = diesel::update(campaign_templates::table.find(input.id))
                .set(&template)
                .returning(CampaignTemplate::as_returning())
                .get_result::<CampaignTemplate>(conn)?;

            if previous_library_template_id != updated.library_template_id {
                if let Some(previous_id) = previous_library_template_id {
                    Self::remove_reply_template_id(conn, updated.campaign_id, previous_id)?;
                }
                if let Some(updated_id) = updated.library_template_id {
                    Self::append_reply_template_id(conn, updated.campaign_id, updated_id)?;
                }
            }

            Ok(updated)
        })
    }

    pub async fn delete(&self, id: i32) -> Result<usize, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        diesel::delete(campaign_templates::table.find(id)).execute(&mut conn)
    }

    pub async fn create_reusable(
        &self,
        input: ReusableTemplateInsert,
    ) -> Result<ReusableReplyTemplate, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        let new_template = NewReusableReplyTemplate {
            user_id: input.user_id,
            name: input.name,
            description: input.description,
            weight: input.weight,
            dm_prompt: input.dm_prompt,
            reply_prompt: input.reply_prompt,
            reply_post_prompt: input.reply_post_prompt,
            usage_count: 0,
            created_at: chrono::Utc::now(),
            updated_at: None,
        };

        diesel::insert_into(template_library::table)
            .values(&new_template)
            .returning(ReusableReplyTemplate::as_returning())
            .get_result(&mut conn)
    }

    pub async fn find_reusable_by_id_and_user(
        &self,
        id: i32,
        user_id: i32,
    ) -> Result<ReusableReplyTemplate, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        diesel::sql_query(
            r#"
            SELECT
                library.id,
                library.user_id,
                library.name,
                library.description,
                library.weight,
                library.dm_prompt,
                library.reply_prompt,
                library.reply_post_prompt,
                COALESCE(bindings.usage_count, 0) AS usage_count,
                library.created_at,
                library.updated_at
            FROM gm_reply_template_library library
            LEFT JOIN (
                WITH bindings AS (
                    SELECT campaign.id AS campaign_id, template_ids.template_id AS library_template_id
                    FROM gm_campaigns campaign
                    CROSS JOIN LATERAL UNNEST(campaign.reply_template_ids) AS template_ids(template_id)
                    WHERE campaign.user_id = $2

                    UNION

                    SELECT campaign_template.campaign_id, campaign_template.library_template_id
                    FROM gm_campaign_templates campaign_template
                    JOIN gm_campaigns campaign ON campaign.id = campaign_template.campaign_id
                    WHERE campaign.user_id = $2
                      AND campaign_template.library_template_id IS NOT NULL
                )
                SELECT library_template_id, COUNT(DISTINCT campaign_id) AS usage_count
                FROM bindings
                GROUP BY library_template_id
            ) bindings ON bindings.library_template_id = library.id
            WHERE library.id = $1 AND library.user_id = $2
            "#,
        )
        .bind::<Integer, _>(id)
        .bind::<Integer, _>(user_id)
        .get_result::<ReusableTemplateRow>(&mut conn)
        .map(Into::into)
    }

    pub async fn find_all_reusable_by_user(
        &self,
        user_id: i32,
        page: i64,
        page_size: i64,
    ) -> Result<(Vec<ReusableReplyTemplate>, i64), DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        let offset = (page - 1).max(0) * page_size;

        let total = template_library::table
            .filter(template_library::user_id.eq(user_id))
            .count()
            .get_result(&mut conn)?;

        let items = diesel::sql_query(
            r#"
            SELECT
                library.id,
                library.user_id,
                library.name,
                library.description,
                library.weight,
                library.dm_prompt,
                library.reply_prompt,
                library.reply_post_prompt,
                COALESCE(bindings.usage_count, 0) AS usage_count,
                library.created_at,
                library.updated_at
            FROM gm_reply_template_library library
            LEFT JOIN (
                WITH bindings AS (
                    SELECT campaign.id AS campaign_id, template_ids.template_id AS library_template_id
                    FROM gm_campaigns campaign
                    CROSS JOIN LATERAL UNNEST(campaign.reply_template_ids) AS template_ids(template_id)
                    WHERE campaign.user_id = $1

                    UNION

                    SELECT campaign_template.campaign_id, campaign_template.library_template_id
                    FROM gm_campaign_templates campaign_template
                    JOIN gm_campaigns campaign ON campaign.id = campaign_template.campaign_id
                    WHERE campaign.user_id = $1
                      AND campaign_template.library_template_id IS NOT NULL
                )
                SELECT library_template_id, COUNT(DISTINCT campaign_id) AS usage_count
                FROM bindings
                GROUP BY library_template_id
            ) bindings ON bindings.library_template_id = library.id
            WHERE library.user_id = $1
            ORDER BY library.updated_at DESC NULLS LAST, library.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind::<Integer, _>(user_id)
        .bind::<BigInt, _>(page_size)
        .bind::<BigInt, _>(offset)
        .load::<ReusableTemplateRow>(&mut conn)?
        .into_iter()
        .map(Into::into)
        .collect();

        Ok((items, total))
    }

    pub async fn update_reusable(
        &self,
        input: ReusableTemplatePatch,
    ) -> Result<ReusableReplyTemplate, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        let target = template_library::table
            .filter(template_library::id.eq(input.id))
            .filter(template_library::user_id.eq(input.user_id))
            .select(ReusableReplyTemplate::as_select());
        let template = target.first::<ReusableReplyTemplate>(&mut conn)?;

        let updated_name = input.name.unwrap_or(template.name);
        let updated_description = input.description.unwrap_or(template.description);
        let updated_weight = input.weight.unwrap_or(template.weight);
        let updated_dm_prompt = input.dm_prompt.unwrap_or(template.dm_prompt);
        let updated_reply_prompt = input.reply_prompt.unwrap_or(template.reply_prompt);
        let updated_reply_post_prompt = input
            .reply_post_prompt
            .unwrap_or(template.reply_post_prompt);
        let updated_at = Some(chrono::Utc::now());

        diesel::update(
            template_library::table
                .filter(template_library::id.eq(input.id))
                .filter(template_library::user_id.eq(input.user_id)),
        )
        .set((
            template_library::name.eq(updated_name),
            template_library::description.eq(updated_description),
            template_library::weight.eq(updated_weight),
            template_library::dm_prompt.eq(updated_dm_prompt),
            template_library::reply_prompt.eq(updated_reply_prompt),
            template_library::reply_post_prompt.eq(updated_reply_post_prompt),
            template_library::updated_at.eq(updated_at),
        ))
        .returning(ReusableReplyTemplate::as_returning())
        .get_result(&mut conn)
    }

    pub async fn delete_reusable(&self, id: i32, user_id: i32) -> Result<usize, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        conn.transaction(|conn| {
            let deleted = diesel::sql_query(
                r#"
                DELETE FROM gm_reply_template_library
                WHERE id = $1
                  AND user_id = $2
                RETURNING id
                "#,
            )
            .bind::<Integer, _>(id)
            .bind::<Integer, _>(user_id)
            .get_result::<DeletedReusableTemplateRow>(conn)
            .optional()?;

            let Some(deleted) = deleted else {
                return Ok(0);
            };
            let _deleted_id = deleted.id;

            diesel::sql_query(
                r#"
                UPDATE gm_campaigns
                SET reply_template_ids = array_remove(reply_template_ids, $1)
                WHERE user_id = $2
                  AND $1 = ANY(reply_template_ids)
                "#,
            )
            .bind::<Integer, _>(id)
            .bind::<Integer, _>(user_id)
            .execute(conn)?;

            Ok(1)
        })
    }

    pub async fn find_reusable_by_ids_and_user(
        &self,
        ids: &[i32],
        user_id: i32,
    ) -> Result<HashMap<i32, ReusableReplyTemplate>, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        if ids.is_empty() {
            return Ok(HashMap::new());
        }

        let rows = template_library::table
            .filter(template_library::user_id.eq(user_id))
            .filter(template_library::id.eq_any(ids))
            .select(ReusableReplyTemplate::as_select())
            .load::<ReusableReplyTemplate>(&mut conn)?;

        let usage_rows = diesel::sql_query(
            r#"
            WITH bindings AS (
                SELECT campaign.id AS campaign_id, template_ids.template_id AS library_template_id
                FROM gm_campaigns campaign
                CROSS JOIN LATERAL UNNEST(campaign.reply_template_ids) AS template_ids(template_id)
                WHERE campaign.user_id = $2
                  AND template_ids.template_id = ANY($1)

                UNION

                SELECT campaign_template.campaign_id, campaign_template.library_template_id
                FROM gm_campaign_templates campaign_template
                JOIN gm_campaigns campaign ON campaign.id = campaign_template.campaign_id
                WHERE campaign.user_id = $2
                  AND campaign_template.library_template_id = ANY($1)
            )
            SELECT library_template_id AS id, COUNT(DISTINCT campaign_id) AS usage_count
            FROM bindings
            GROUP BY library_template_id
            "#,
        )
        .bind::<Array<Integer>, _>(ids)
        .bind::<Integer, _>(user_id)
        .load::<ReusableUsageRow>(&mut conn)?;

        let usage_by_id: HashMap<i32, i32> = usage_rows
            .into_iter()
            .map(|row| (row.id, row.usage_count as i32))
            .collect();

        Ok(rows
            .into_iter()
            .map(|mut row| {
                row.usage_count = usage_by_id.get(&row.id).copied().unwrap_or(0);
                (row.id, row)
            })
            .collect())
    }

    pub async fn assign_reusable_to_campaign(
        &self,
        campaign_id: i32,
        library_template_id: i32,
        weight_override: Option<i32>,
        user_id: i32,
    ) -> Result<CampaignTemplate, AssignReusableTemplateError> {
        let mut conn = self.pool.get().expect("Connection error");
        conn.transaction(|conn| {
            let campaign_locked = diesel::sql_query(
                r#"
                SELECT id
                FROM gm_campaigns
                WHERE id = $1
                  AND user_id = $2
                FOR UPDATE
                "#,
            )
            .bind::<Integer, _>(campaign_id)
            .bind::<Integer, _>(user_id)
            .load::<DeletedReusableTemplateRow>(conn)?;
            if campaign_locked.is_empty() {
                return Err(AssignReusableTemplateError::CampaignNotFound);
            }

            let template_locked = diesel::sql_query(
                r#"
                SELECT id
                FROM gm_reply_template_library
                WHERE id = $1
                  AND user_id = $2
                FOR KEY SHARE
                "#,
            )
            .bind::<Integer, _>(library_template_id)
            .bind::<Integer, _>(user_id)
            .load::<DeletedReusableTemplateRow>(conn)?;
            if template_locked.is_empty() {
                return Err(AssignReusableTemplateError::TemplateNotFound);
            }

            let assigned = diesel::sql_query(
                r#"
                INSERT INTO gm_campaign_templates (
                    campaign_id,
                    library_template_id,
                    name,
                    weight,
                    dm_prompt,
                    reply_prompt,
                    reply_post_prompt,
                    created_at,
                    updated_at
                )
                SELECT
                    campaigns.id,
                    library.id,
                    library.name,
                    COALESCE($3, library.weight),
                    library.dm_prompt,
                    library.reply_prompt,
                    library.reply_post_prompt,
                    NOW(),
                    NULL
                FROM gm_campaigns campaigns
                JOIN gm_reply_template_library library
                  ON library.id = $2
                 AND library.user_id = $4
                WHERE campaigns.id = $1
                  AND campaigns.user_id = $4
                ON CONFLICT (campaign_id, library_template_id) WHERE library_template_id IS NOT NULL
                DO UPDATE SET
                    name = EXCLUDED.name,
                    weight = EXCLUDED.weight,
                    dm_prompt = EXCLUDED.dm_prompt,
                    reply_prompt = EXCLUDED.reply_prompt,
                    reply_post_prompt = EXCLUDED.reply_post_prompt,
                    updated_at = NOW()
                RETURNING *
                "#,
            )
            .bind::<Integer, _>(campaign_id)
            .bind::<Integer, _>(library_template_id)
            .bind::<Nullable<Integer>, _>(weight_override)
            .bind::<Integer, _>(user_id)
            .get_result::<AssignedCampaignTemplate>(conn)?;

            Self::append_reply_template_id(conn, campaign_id, library_template_id).map_err(
                |error| match error {
                    CampaignTemplateWriteError::ReplyTemplateIdsFull => {
                        AssignReusableTemplateError::ReplyTemplateIdsFull
                    }
                    CampaignTemplateWriteError::Diesel(error) => {
                        AssignReusableTemplateError::Diesel(error)
                    }
                },
            )?;

            Ok(CampaignTemplate::from(assigned))
        })
    }

    pub async fn delete_and_sync(&self, id: i32) -> Result<usize, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        conn.transaction(|conn| {
            let template = campaign_templates::table
                .find(id)
                .select(CampaignTemplate::as_select())
                .first::<CampaignTemplate>(conn)?;
            let deleted = diesel::delete(campaign_templates::table.find(id)).execute(conn)?;
            if let Some(library_template_id) = template.library_template_id {
                Self::remove_reply_template_id(conn, template.campaign_id, library_template_id)?;
            }
            Ok(deleted)
        })
    }

    fn append_reply_template_id(
        conn: &mut PgConnection,
        campaign_id: i32,
        library_template_id: i32,
    ) -> Result<ReplyTemplateAppendResult, CampaignTemplateWriteError> {
        let appended = diesel::sql_query(
            r#"
            UPDATE gm_campaigns
            SET reply_template_ids = array_append(reply_template_ids, $1)
            WHERE id = $2
              AND NOT ($1 = ANY(reply_template_ids))
              AND cardinality(reply_template_ids) < 100
            "#,
        )
        .bind::<Integer, _>(library_template_id)
        .bind::<Integer, _>(campaign_id)
        .execute(conn)?;

        if appended > 0 {
            return Ok(ReplyTemplateAppendResult::Appended);
        }

        let already_present_count = diesel::sql_query(
            r#"
            SELECT COUNT(*) AS total
            FROM gm_campaigns
            WHERE id = $1
              AND $2 = ANY(reply_template_ids)
            "#,
        )
        .bind::<Integer, _>(campaign_id)
        .bind::<Integer, _>(library_template_id)
        .get_result::<CountRow>(conn)?
        .total;

        if already_present_count > 0 {
            Ok(ReplyTemplateAppendResult::AlreadyPresent)
        } else {
            Err(CampaignTemplateWriteError::ReplyTemplateIdsFull)
        }
    }

    fn remove_reply_template_id(
        conn: &mut PgConnection,
        campaign_id: i32,
        library_template_id: i32,
    ) -> Result<usize, DieselError> {
        diesel::sql_query(
            r#"
            UPDATE gm_campaigns
            SET reply_template_ids = array_remove(reply_template_ids, $1)
            WHERE id = $2
            "#,
        )
        .bind::<Integer, _>(library_template_id)
        .bind::<Integer, _>(campaign_id)
        .execute(conn)
    }
}
