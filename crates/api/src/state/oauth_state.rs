use crate::config::database::Database;
use crate::service::oauth_service::OauthService;
use std::sync::Arc;

#[derive(Clone)]
pub struct OauthState {
    pub oauth_service: OauthService,
}

impl OauthState {
    pub fn new(db: &Arc<Database>) -> Self {
        Self {
            oauth_service: OauthService::new(db),
        }
    }
}
