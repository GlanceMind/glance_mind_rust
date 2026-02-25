use crate::config::database::DBPool;
use async_trait::async_trait;
use diesel::prelude::*;
use diesel::SelectableHelper;
use glance_mind_db::entity::user::User;
use glance_mind_db::schema::gm_users as users;
// use std::sync::Arc;
use tokio::task;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait UserRepositoryTrait {
    async fn find_by_email(&self, email: String) -> Option<User>;
    async fn find_by_username(&self, username: String) -> Option<User>;
    async fn find_by_identifier(&self, identifier: String) -> Option<User>;
    async fn find(&self, id: i32) -> Result<User, diesel::result::Error>;
    async fn create(
        &self,
        email: Option<String>,
        username: Option<String>,
        password_hash: String,
        invitation_code: Option<String>,
        referred_by: Option<String>,
    ) -> Result<User, diesel::result::Error>;
    async fn update(&self, user: User) -> Result<User, diesel::result::Error>;
}

#[derive(Clone)]
pub struct UserRepository {
    pub(crate) pool: DBPool,
}

impl UserRepository {
    pub fn new(pool: DBPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserRepositoryTrait for UserRepository {
    async fn find_by_email(&self, email_addr: String) -> Option<User> {
        let pool = self.pool.clone();
        task::spawn_blocking(move || {
            let mut conn = pool.get().ok()?;
            users::dsl::gm_users
                .filter(users::dsl::email.eq(email_addr))
                .select(User::as_select())
                .first(&mut conn)
                .optional()
                .unwrap_or(None)
        })
        .await
        .unwrap_or(None)
    }

    async fn find_by_username(&self, username: String) -> Option<User> {
        let pool = self.pool.clone();
        // Case-insensitive username lookup
        let username_lower = username.to_lowercase();
        task::spawn_blocking(move || {
            let mut conn = pool.get().ok()?;
            users::dsl::gm_users
                .filter(diesel::dsl::sql::<diesel::sql_types::Bool>(&format!(
                    "LOWER(username) = '{}'",
                    username_lower
                )))
                .select(User::as_select())
                .first(&mut conn)
                .optional()
                .unwrap_or(None)
        })
        .await
        .unwrap_or(None)
    }

    async fn find_by_identifier(&self, identifier: String) -> Option<User> {
        // Try email first
        if identifier.contains('@') {
            if let Some(user) = self.find_by_email(identifier.clone()).await {
                return Some(user);
            }
        }
        // Try username
        self.find_by_username(identifier).await
    }

    async fn find(&self, user_id: i32) -> Result<User, diesel::result::Error> {
        let pool = self.pool.clone();
        task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|_| diesel::result::Error::RollbackTransaction)?; // Mapping pool error to Rollback for now
            users::dsl::gm_users
                .find(user_id)
                .select(User::as_select())
                .first(&mut conn)
        })
        .await
        .map_err(|_| diesel::result::Error::RollbackTransaction)?
    }

    async fn create(
        &self,
        email_addr: Option<String>,
        username: Option<String>,
        pwd_hash: String,
        invite_code: Option<String>,
        ref_by: Option<String>,
    ) -> Result<User, diesel::result::Error> {
        let pool = self.pool.clone();
        // ID is SERIAL, so we don't set it manually
        let new_user = glance_mind_db::entity::user::NewUser {
            email: email_addr,
            username,
            password_hash: pwd_hash,
            invitation_code: invite_code,
            referred_by: ref_by,
            company_name: None,
            api_key: None,
            status: "PENDING_VERIFICATION".to_string(),
            full_name: "".to_string(),
            role: "user".to_string(),
            is_active: true,
            created_at: chrono::Utc::now(),
            updated_at: None,
            permissions: None,
        };

        task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|_| diesel::result::Error::RollbackTransaction)?;
            diesel::insert_into(users::table)
                .values(&new_user)
                .returning(User::as_returning())
                .get_result(&mut conn)
        })
        .await
        .map_err(|_| diesel::result::Error::RollbackTransaction)?
    }

    async fn update(&self, user: User) -> Result<User, diesel::result::Error> {
        let pool = self.pool.clone();
        task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|_| diesel::result::Error::RollbackTransaction)?;
            diesel::update(users::dsl::gm_users.find(user.id))
                .set(&user)
                .returning(User::as_returning())
                .get_result(&mut conn)
        })
        .await
        .map_err(|_| diesel::result::Error::RollbackTransaction)?
    }
}
