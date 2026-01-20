use crate::config::database::DBPool;
use async_trait::async_trait;
use diesel::prelude::*;
use diesel::SelectableHelper;
use glance_mind_db::entity::login_log::{LoginLog, NewLoginLog};
use glance_mind_db::schema::gm_login_logs;
use tokio::task;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait LoginLogRepositoryTrait {
    /// Create login log
    async fn create(&self, new_log: NewLoginLog) -> Result<LoginLog, diesel::result::Error>;

    /// Get user's login history
    async fn find_by_user_id(
        &self,
        user_id: i32,
        limit: i64,
    ) -> Result<Vec<LoginLog>, diesel::result::Error>;

    /// Get recent login records for specified IP
    async fn find_by_ip(
        &self,
        ip: String,
        limit: i64,
    ) -> Result<Vec<LoginLog>, diesel::result::Error>;
}

#[derive(Clone)]
pub struct LoginLogRepository {
    pool: DBPool,
}

impl LoginLogRepository {
    pub fn new(pool: DBPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl LoginLogRepositoryTrait for LoginLogRepository {
    async fn create(&self, new_log: NewLoginLog) -> Result<LoginLog, diesel::result::Error> {
        let pool = self.pool.clone();
        task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|_| diesel::result::Error::RollbackTransaction)?;

            diesel::insert_into(gm_login_logs::table)
                .values(&new_log)
                .returning(LoginLog::as_returning())
                .get_result(&mut conn)
        })
        .await
        .map_err(|_| diesel::result::Error::RollbackTransaction)?
    }

    async fn find_by_user_id(
        &self,
        user_id: i32,
        limit: i64,
    ) -> Result<Vec<LoginLog>, diesel::result::Error> {
        let pool = self.pool.clone();
        task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|_| diesel::result::Error::RollbackTransaction)?;

            gm_login_logs::table
                .filter(gm_login_logs::user_id.eq(user_id))
                .order(gm_login_logs::login_at.desc())
                .limit(limit)
                .select(LoginLog::as_select())
                .load(&mut conn)
        })
        .await
        .map_err(|_| diesel::result::Error::RollbackTransaction)?
    }

    async fn find_by_ip(
        &self,
        ip: String,
        limit: i64,
    ) -> Result<Vec<LoginLog>, diesel::result::Error> {
        let pool = self.pool.clone();
        task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|_| diesel::result::Error::RollbackTransaction)?;

            gm_login_logs::table
                .filter(gm_login_logs::ip_address.eq(ip))
                .order(gm_login_logs::login_at.desc())
                .limit(limit)
                .select(LoginLog::as_select())
                .load(&mut conn)
        })
        .await
        .map_err(|_| diesel::result::Error::RollbackTransaction)?
    }
}
