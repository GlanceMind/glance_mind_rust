use crate::config::database::Database;
use crate::repository::login_log_repository::LoginLogRepository;
use crate::repository::user_repository;
// TokenService removed
use crate::service::user_service::UserService;
use std::sync::Arc;

#[derive(Clone)]
pub struct AuthState {
    pub(crate) user_service: UserService<user_repository::UserRepository>,
    pub(crate) login_log_repo: LoginLogRepository,
}

impl AuthState {
    pub fn new(db: &Arc<Database>) -> Self {
        let user_repo = user_repository::UserRepository::new(db.pool.clone());
        let user_service = UserService::new(db, user_repo.clone());
        let login_log_repo = LoginLogRepository::new(db.pool.clone());

        Self {
            user_service,
            login_log_repo,
        }
    }
}
