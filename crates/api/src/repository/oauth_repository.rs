use crate::config::database::DBPool;
use chrono::Utc;
use diesel::prelude::*;
use glance_mind_db::entity::oauth::{
    NewOauthAuditLog, NewOauthCode, NewOauthRefreshToken, OauthAuditLog, OauthCode,
    OauthRefreshToken, OtaConfig, OtaConfigUpdate,
};
use glance_mind_db::schema::{oauth_audit_log, oauth_codes, oauth_refresh_tokens, ota_config};
use tokio::task;
use uuid::Uuid;

#[derive(Clone)]
pub struct OauthRepository {
    pool: DBPool,
}

impl OauthRepository {
    pub fn new(pool: DBPool) -> Self {
        Self { pool }
    }

    // -----------------------------------------------------------------------
    // oauth_codes
    // -----------------------------------------------------------------------

    pub async fn create_code(
        &self,
        new_code: NewOauthCode,
    ) -> Result<OauthCode, diesel::result::Error> {
        let pool = self.pool.clone();
        task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|_| diesel::result::Error::RollbackTransaction)?;
            diesel::insert_into(oauth_codes::table)
                .values(&new_code)
                .returning(OauthCode::as_returning())
                .get_result(&mut conn)
        })
        .await
        .map_err(|_| diesel::result::Error::RollbackTransaction)?
    }

    pub async fn find_code(
        &self,
        code: String,
    ) -> Result<Option<OauthCode>, diesel::result::Error> {
        let pool = self.pool.clone();
        task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|_| diesel::result::Error::RollbackTransaction)?;
            oauth_codes::table
                .find(code)
                .select(OauthCode::as_select())
                .first(&mut conn)
                .optional()
        })
        .await
        .map_err(|_| diesel::result::Error::RollbackTransaction)?
    }

    /// Mark a code as redeemed. Returns an error if it was already redeemed (or missing).
    pub async fn mark_code_redeemed(&self, code: String) -> Result<(), diesel::result::Error> {
        let pool = self.pool.clone();
        task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|_| diesel::result::Error::RollbackTransaction)?;
            diesel::update(oauth_codes::table.find(code))
                .set(oauth_codes::redeemed_at.eq(Utc::now()))
                .execute(&mut conn)
                .map(|_| ())
        })
        .await
        .map_err(|_| diesel::result::Error::RollbackTransaction)?
    }

    // -----------------------------------------------------------------------
    // oauth_refresh_tokens
    // -----------------------------------------------------------------------

    pub async fn create_refresh_token(
        &self,
        new_token: NewOauthRefreshToken,
    ) -> Result<OauthRefreshToken, diesel::result::Error> {
        let pool = self.pool.clone();
        task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|_| diesel::result::Error::RollbackTransaction)?;
            diesel::insert_into(oauth_refresh_tokens::table)
                .values(&new_token)
                .returning(OauthRefreshToken::as_returning())
                .get_result(&mut conn)
        })
        .await
        .map_err(|_| diesel::result::Error::RollbackTransaction)?
    }

    pub async fn find_refresh_token_by_hash(
        &self,
        token_hash: String,
    ) -> Result<Option<OauthRefreshToken>, diesel::result::Error> {
        let pool = self.pool.clone();
        task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|_| diesel::result::Error::RollbackTransaction)?;
            oauth_refresh_tokens::table
                .filter(oauth_refresh_tokens::token_hash.eq(token_hash))
                .select(OauthRefreshToken::as_select())
                .first(&mut conn)
                .optional()
        })
        .await
        .map_err(|_| diesel::result::Error::RollbackTransaction)?
    }

    /// Revoke a single refresh token by id, setting replaced_by_id.
    pub async fn revoke_refresh_token(
        &self,
        id: i64,
        replaced_by_id: Option<i64>,
    ) -> Result<(), diesel::result::Error> {
        let pool = self.pool.clone();
        task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|_| diesel::result::Error::RollbackTransaction)?;
            diesel::update(oauth_refresh_tokens::table.find(id))
                .set((
                    oauth_refresh_tokens::revoked_at.eq(Utc::now()),
                    oauth_refresh_tokens::replaced_by_id.eq(replaced_by_id),
                ))
                .execute(&mut conn)
                .map(|_| ())
        })
        .await
        .map_err(|_| diesel::result::Error::RollbackTransaction)?
    }

    /// Revoke ALL tokens in a family (family revocation on reuse detection).
    pub async fn revoke_family(&self, family_id: Uuid) -> Result<usize, diesel::result::Error> {
        let pool = self.pool.clone();
        task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|_| diesel::result::Error::RollbackTransaction)?;
            diesel::update(
                oauth_refresh_tokens::table
                    .filter(oauth_refresh_tokens::family_id.eq(family_id))
                    .filter(oauth_refresh_tokens::revoked_at.is_null()),
            )
            .set(oauth_refresh_tokens::revoked_at.eq(Utc::now()))
            .execute(&mut conn)
        })
        .await
        .map_err(|_| diesel::result::Error::RollbackTransaction)?
    }

    // -----------------------------------------------------------------------
    // oauth_audit_log
    // -----------------------------------------------------------------------

    pub async fn write_audit(
        &self,
        entry: NewOauthAuditLog,
    ) -> Result<OauthAuditLog, diesel::result::Error> {
        let pool = self.pool.clone();
        task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|_| diesel::result::Error::RollbackTransaction)?;
            diesel::insert_into(oauth_audit_log::table)
                .values(&entry)
                .returning(OauthAuditLog::as_returning())
                .get_result(&mut conn)
        })
        .await
        .map_err(|_| diesel::result::Error::RollbackTransaction)?
    }

    // -----------------------------------------------------------------------
    // ota_config
    // -----------------------------------------------------------------------

    pub async fn get_ota_config(
        &self,
        key: String,
    ) -> Result<Option<OtaConfig>, diesel::result::Error> {
        let pool = self.pool.clone();
        task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|_| diesel::result::Error::RollbackTransaction)?;
            ota_config::table
                .find(key)
                .select(OtaConfig::as_select())
                .first(&mut conn)
                .optional()
        })
        .await
        .map_err(|_| diesel::result::Error::RollbackTransaction)?
    }

    pub async fn upsert_ota_config(
        &self,
        key: String,
        value: String,
        updated_by: String,
    ) -> Result<OtaConfig, diesel::result::Error> {
        let pool = self.pool.clone();
        task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|_| diesel::result::Error::RollbackTransaction)?;
            let update = OtaConfigUpdate {
                value,
                updated_at: Utc::now(),
                updated_by,
            };
            diesel::update(ota_config::table.find(key))
                .set(&update)
                .returning(OtaConfig::as_returning())
                .get_result(&mut conn)
        })
        .await
        .map_err(|_| diesel::result::Error::RollbackTransaction)?
    }
}
