use crate::config::database::Database;
use crate::service::oauth_service::OauthService;
use std::sync::Arc;

#[derive(Clone)]
pub struct OauthState {
    pub oauth_service: OauthService,
    /// OTA bearer token read once at startup; avoids per-request env::var() calls (P1 #1).
    pub ota_auth_token: String,
}

impl OauthState {
    pub fn new(db: &Arc<Database>) -> Self {
        // Read OTA_AUTH_TOKEN at construction time so the handler has no per-request env reads.
        let ota_auth_token = std::env::var("OTA_AUTH_TOKEN").unwrap_or_default();
        Self {
            oauth_service: OauthService::new(db),
            ota_auth_token,
        }
    }
}
