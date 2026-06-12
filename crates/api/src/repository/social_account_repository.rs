use crate::config::database::DBPool;
use diesel::prelude::*;
use diesel::result::Error as DieselError;
use diesel::SelectableHelper;
use glance_mind_db::entity::social_account::{
    NewSocialAccount, SocialAccount, UpdateSocialAccount,
};
use glance_mind_db::schema::gm_social_accounts as social_accounts;

#[derive(Clone)]
pub struct SocialAccountRepository {
    pool: DBPool,
}

impl SocialAccountRepository {
    pub fn new(pool: DBPool) -> Self {
        Self { pool }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn find_by_user(
        &self,
        user_id: i32,
        page: i64,
        page_size: i64,
        group_id: Option<i32>,
        username: Option<String>,
        platform_id: Option<i32>,
        status: Option<String>,
        device_id: Option<String>,
    ) -> Result<(Vec<SocialAccount>, i64), DieselError> {
        let mut conn = self.pool.get().expect("Connection error");

        // Build base query with dynamic filters using into_boxed()
        let mut count_query = social_accounts::table
            .filter(social_accounts::user_id.eq(user_id))
            .filter(social_accounts::status.ne("DELETED"))
            .into_boxed();

        let mut items_query = social_accounts::table
            .filter(social_accounts::user_id.eq(user_id))
            .filter(social_accounts::status.ne("DELETED"))
            .into_boxed();

        // Apply optional filters
        if let Some(gid) = group_id {
            count_query = count_query.filter(social_accounts::group_id.eq(gid));
            items_query = items_query.filter(social_accounts::group_id.eq(gid));
        }

        if let Some(ref name) = username {
            let pattern = format!("%{}%", name);
            count_query = count_query.filter(social_accounts::username.ilike(pattern.clone()));
            items_query = items_query.filter(social_accounts::username.ilike(pattern));
        }

        if let Some(pid) = platform_id {
            count_query = count_query.filter(social_accounts::platform_id.eq(pid));
            items_query = items_query.filter(social_accounts::platform_id.eq(pid));
        }

        if let Some(ref s) = status {
            count_query = count_query.filter(social_accounts::status.eq(s.clone()));
            items_query = items_query.filter(social_accounts::status.eq(s.clone()));
        }

        if let Some(ref did) = device_id {
            let pattern = format!("%{}%", did);
            count_query = count_query.filter(social_accounts::device_id.ilike(pattern.clone()));
            items_query = items_query.filter(social_accounts::device_id.ilike(pattern));
        }

        let total: i64 = count_query.count().get_result(&mut conn)?;

        let items = items_query
            .order(social_accounts::created_at.desc())
            .limit(page_size)
            .offset((page - 1) * page_size)
            .select(SocialAccount::as_select())
            .load(&mut conn)?;

        Ok((items, total))
    }

    pub async fn find_by_group(
        &self,
        group_id: i32,
        page: i64,
        page_size: i64,
    ) -> Result<(Vec<SocialAccount>, i64), DieselError> {
        let mut conn = self.pool.get().expect("Connection error");

        let total = social_accounts::table
            .filter(social_accounts::group_id.eq(group_id))
            .count()
            .get_result(&mut conn)?;

        let items = social_accounts::table
            .filter(social_accounts::group_id.eq(group_id))
            .limit(page_size)
            .offset((page - 1) * page_size)
            .offset((page - 1) * page_size)
            .select(SocialAccount::as_select())
            .load(&mut conn)?;

        Ok((items, total))
    }

    pub async fn find_by_id(&self, account_id: i32) -> Result<SocialAccount, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        social_accounts::table
            .find(account_id)
            .select(SocialAccount::as_select())
            .first(&mut conn)
    }

    pub async fn create(
        &self,
        new_account: NewSocialAccount,
    ) -> Result<SocialAccount, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        diesel::insert_into(social_accounts::table)
            .values(&new_account)
            .returning(SocialAccount::as_returning())
            .get_result(&mut conn)
    }

    pub async fn update(
        &self,
        account_id: i32,
        update_data: UpdateSocialAccount,
    ) -> Result<SocialAccount, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        diesel::update(social_accounts::table.find(account_id))
            .set(&update_data)
            .returning(SocialAccount::as_returning())
            .get_result(&mut conn)
    }

    pub async fn delete(&self, account_id: i32) -> Result<usize, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        diesel::delete(social_accounts::table.find(account_id)).execute(&mut conn)
    }

    /// Clear group_id (set to NULL) for an account
    pub async fn clear_group(&self, account_id: i32) -> Result<usize, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        diesel::update(social_accounts::table.find(account_id))
            .set(social_accounts::group_id.eq(None::<i32>))
            .execute(&mut conn)
    }

    pub async fn count_by_group(&self, group_id: i32) -> Result<i64, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        social_accounts::table
            .filter(social_accounts::group_id.eq(group_id))
            .count()
            .get_result(&mut conn)
    }

    pub async fn get_statistics(
        &self,
        user_id: i32,
        group_id: Option<i32>,
    ) -> Result<crate::dto::social_account_dto::AccountStatisticsDto, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");

        let base_filter = social_accounts::table
            .filter(social_accounts::user_id.eq(user_id))
            .filter(social_accounts::status.ne("DELETED"));

        let total: i64 = if let Some(gid) = group_id {
            base_filter
                .filter(social_accounts::group_id.eq(gid))
                .count()
                .get_result(&mut conn)?
        } else {
            base_filter.count().get_result(&mut conn)?
        };

        let active: i64 = if let Some(gid) = group_id {
            base_filter
                .filter(social_accounts::group_id.eq(gid))
                .filter(social_accounts::status.eq("ACTIVE"))
                .count()
                .get_result(&mut conn)?
        } else {
            base_filter
                .filter(social_accounts::status.eq("ACTIVE"))
                .count()
                .get_result(&mut conn)?
        };

        let risk_control: i64 = if let Some(gid) = group_id {
            base_filter
                .filter(social_accounts::group_id.eq(gid))
                .filter(social_accounts::status.eq("RISK_CONTROL"))
                .count()
                .get_result(&mut conn)?
        } else {
            base_filter
                .filter(social_accounts::status.eq("RISK_CONTROL"))
                .count()
                .get_result(&mut conn)?
        };

        let unavailable: i64 = if let Some(gid) = group_id {
            base_filter
                .filter(social_accounts::group_id.eq(gid))
                .filter(social_accounts::status.eq("UNAVAILABLE"))
                .count()
                .get_result(&mut conn)?
        } else {
            base_filter
                .filter(social_accounts::status.eq("UNAVAILABLE"))
                .count()
                .get_result(&mut conn)?
        };

        Ok(crate::dto::social_account_dto::AccountStatisticsDto {
            total,
            active,
            risk_control,
            unavailable,
        })
    }

    pub async fn get_platform_name(&self, platform_id: i32) -> Result<String, DieselError> {
        use glance_mind_db::schema::gm_platforms;

        let mut conn = self.pool.get().expect("Connection error");
        gm_platforms::table
            .find(platform_id)
            .select(gm_platforms::name)
            .first(&mut conn)
    }

    /// Batch create multiple accounts
    pub async fn batch_create(
        &self,
        new_accounts: Vec<NewSocialAccount>,
    ) -> Result<Vec<SocialAccount>, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        diesel::insert_into(social_accounts::table)
            .values(&new_accounts)
            .returning(SocialAccount::as_returning())
            .get_results(&mut conn)
    }

    /// Return the subset of `profile_names` that already exist as non-DELETED
    /// accounts for this (user, platform). Used to dedupe before batch insert
    /// since there is no DB-level UNIQUE constraint on (user_id, platform_id,
    /// profile_name).
    pub async fn find_existing_profile_names(
        &self,
        user_id: i32,
        platform_id: i32,
        profile_names: &[String],
    ) -> Result<Vec<String>, DieselError> {
        if profile_names.is_empty() {
            return Ok(Vec::new());
        }
        let mut conn = self.pool.get().expect("Connection error");
        let rows: Vec<Option<String>> = social_accounts::table
            .filter(social_accounts::user_id.eq(user_id))
            .filter(social_accounts::platform_id.eq(platform_id))
            .filter(social_accounts::status.ne("DELETED"))
            .filter(social_accounts::profile_name.eq_any(profile_names))
            .select(social_accounts::profile_name)
            .load(&mut conn)?;
        Ok(rows.into_iter().flatten().collect())
    }
}
