use glance_mind_db::entity::user::User;
use jsonwebtoken::{encode, EncodingKey, Header};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone)]
pub struct DramaFacade {
    client: Client,
    gateway_base: String,
}

impl DramaFacade {
    pub fn new() -> Self {
        let gateway_base = std::env::var("AGENT_HUB_GATEWAY_URL")
            .unwrap_or_else(|_| "http://localhost:8090".to_string());
        let gateway_base = gateway_base.trim_end_matches('/').to_string();
        Self {
            client: Client::new(),
            gateway_base,
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}/api/v1/video{}", self.gateway_base, path)
    }

    pub async fn preflight(&self, body: Value, user: &User) -> Result<(u16, Value), FacadeError> {
        self.post("/preflight", body, user).await
    }

    pub async fn create_project(
        &self,
        body: Value,
        user: &User,
    ) -> Result<(u16, Value), FacadeError> {
        self.post("/projects", body, user).await
    }

    pub async fn get_project(
        &self,
        project_id: &str,
        user: &User,
    ) -> Result<(u16, Value), FacadeError> {
        self.get(&format!("/projects/{}", project_id), user).await
    }

    pub async fn list_projects(
        &self,
        query: &[(&str, &str)],
        user: &User,
    ) -> Result<(u16, Value), FacadeError> {
        let url = if query.is_empty() {
            self.url("/projects")
        } else {
            let qs: Vec<String> = query.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
            format!("{}?{}", self.url("/projects"), qs.join("&"))
        };
        let resp = self
            .client
            .get(&url)
            .header(
                "Authorization",
                format!("Bearer {}", self.gateway_token(user)?),
            )
            .header("X-User-Id", user.id.to_string())
            .send()
            .await
            .map_err(FacadeError::Upstream)?;
        let status = resp.status().as_u16();
        let body = resp.json::<Value>().await.unwrap_or(Value::Null);
        Ok((status, body))
    }

    pub async fn cancel_project(
        &self,
        project_id: &str,
        user: &User,
    ) -> Result<(u16, Value), FacadeError> {
        let url = self.url(&format!("/projects/{}", project_id));
        let resp = self
            .client
            .delete(&url)
            .header(
                "Authorization",
                format!("Bearer {}", self.gateway_token(user)?),
            )
            .header("X-User-Id", user.id.to_string())
            .send()
            .await
            .map_err(FacadeError::Upstream)?;
        let status = resp.status().as_u16();
        let body = resp.json::<Value>().await.unwrap_or(Value::Null);
        Ok((status, body))
    }

    pub async fn clarify(
        &self,
        project_id: &str,
        body: Value,
        user: &User,
    ) -> Result<(u16, Value), FacadeError> {
        self.post(&format!("/projects/{}/clarify", project_id), body, user)
            .await
    }

    pub async fn strategy_select(
        &self,
        project_id: &str,
        body: Value,
        user: &User,
    ) -> Result<(u16, Value), FacadeError> {
        self.post(
            &format!("/projects/{}/stages/strategy/select", project_id),
            body,
            user,
        )
        .await
    }

    pub async fn approve(
        &self,
        project_id: &str,
        body: Value,
        user: &User,
    ) -> Result<(u16, Value), FacadeError> {
        self.post(&format!("/projects/{}/approve", project_id), body, user)
            .await
    }

    pub async fn get_script(
        &self,
        project_id: &str,
        user: &User,
    ) -> Result<(u16, Value), FacadeError> {
        self.get(&format!("/projects/{}/script", project_id), user)
            .await
    }

    pub async fn get_shots(
        &self,
        project_id: &str,
        user: &User,
    ) -> Result<(u16, Value), FacadeError> {
        self.get(&format!("/projects/{}/shots", project_id), user)
            .await
    }

    pub async fn get_render(
        &self,
        project_id: &str,
        user: &User,
    ) -> Result<(u16, Value), FacadeError> {
        self.get(&format!("/projects/{}/render", project_id), user)
            .await
    }

    pub async fn get_artifacts(
        &self,
        project_id: &str,
        user: &User,
    ) -> Result<(u16, Value), FacadeError> {
        self.get(&format!("/projects/{}/artifacts", project_id), user)
            .await
    }

    pub async fn get_cost(
        &self,
        project_id: &str,
        user: &User,
    ) -> Result<(u16, Value), FacadeError> {
        self.get(&format!("/projects/{}/cost", project_id), user)
            .await
    }

    pub async fn get_fallbacks(
        &self,
        project_id: &str,
        user: &User,
    ) -> Result<(u16, Value), FacadeError> {
        self.get(&format!("/projects/{}/fallbacks", project_id), user)
            .await
    }

    pub async fn post_script_feedback(
        &self,
        project_id: &str,
        body: Value,
        user: &User,
    ) -> Result<(u16, Value), FacadeError> {
        self.post(
            &format!("/projects/{}/script/feedback", project_id),
            body,
            user,
        )
        .await
    }

    pub async fn retry(
        &self,
        project_id: &str,
        body: Value,
        user: &User,
    ) -> Result<(u16, Value), FacadeError> {
        self.post(&format!("/projects/{}/retry", project_id), body, user)
            .await
    }

    pub async fn clone_project(
        &self,
        project_id: &str,
        body: Value,
        user: &User,
    ) -> Result<(u16, Value), FacadeError> {
        self.post(&format!("/projects/{}/clone", project_id), body, user)
            .await
    }

    pub async fn scene_rerun(
        &self,
        project_id: &str,
        body: Value,
        user: &User,
    ) -> Result<(u16, Value), FacadeError> {
        let scene_id = body
            .get("scene_id")
            .and_then(|v| v.as_str())
            .ok_or(FacadeError::MissingSceneId)?;
        self.post(
            &format!("/projects/{}/scenes/{}/rerun", project_id, scene_id),
            body,
            user,
        )
        .await
    }

    fn auth_identifier(user: &User) -> Result<String, FacadeError> {
        user.email
            .clone()
            .or_else(|| user.username.clone())
            .ok_or(FacadeError::MissingUserIdentifier)
    }

    fn gateway_token(&self, user: &User) -> Result<String, FacadeError> {
        let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "dev-secret".to_string());
        let claims = GatewayClaims::new(user.id, Self::auth_identifier(user)?);
        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .map_err(FacadeError::TokenEncoding)
    }

    async fn get(&self, path: &str, user: &User) -> Result<(u16, Value), FacadeError> {
        let url = self.url(path);
        let resp = self
            .client
            .get(&url)
            .header(
                "Authorization",
                format!("Bearer {}", self.gateway_token(user)?),
            )
            .header("X-User-Id", user.id.to_string())
            .send()
            .await
            .map_err(FacadeError::Upstream)?;
        let status = resp.status().as_u16();
        let body = resp.json::<Value>().await.unwrap_or(Value::Null);
        Ok((status, body))
    }

    async fn post(
        &self,
        path: &str,
        body: Value,
        user: &User,
    ) -> Result<(u16, Value), FacadeError> {
        let url = self.url(path);
        let resp = self
            .client
            .post(&url)
            .header(
                "Authorization",
                format!("Bearer {}", self.gateway_token(user)?),
            )
            .header("X-User-Id", user.id.to_string())
            .json(&body)
            .send()
            .await
            .map_err(FacadeError::Upstream)?;
        let status = resp.status().as_u16();
        let body = resp.json::<Value>().await.unwrap_or(Value::Null);
        Ok((status, body))
    }
}

impl Default for DramaFacade {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct GatewayClaims {
    sub: i32,
    identifier: String,
    iat: i64,
    exp: usize,
}

impl GatewayClaims {
    fn new(user_id: i32, identifier: String) -> Self {
        let iat = chrono::Utc::now().timestamp();
        let exp = (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize;
        Self {
            sub: user_id,
            identifier,
            iat,
            exp,
        }
    }
}

#[derive(Debug)]
pub enum FacadeError {
    Upstream(reqwest::Error),
    TokenEncoding(jsonwebtoken::errors::Error),
    MissingUserIdentifier,
    MissingSceneId,
}

impl std::fmt::Display for FacadeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Upstream(e) => write!(f, "gateway unreachable: {}", e),
            Self::TokenEncoding(e) => write!(f, "gateway token encoding failed: {}", e),
            Self::MissingUserIdentifier => write!(f, "user has neither email nor username"),
            Self::MissingSceneId => write!(f, "scene rerun requires scene_id"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{decode, DecodingKey, Validation};

    #[test]
    fn gateway_claims_match_gateway_middleware_shape() {
        let facade = DramaFacade::new();
        std::env::set_var("JWT_SECRET", "unit-test-secret");

        let user = User {
            id: 42,
            email: Some("test@example.com".to_string()),
            password_hash: "x".to_string(),
            invitation_code: None,
            referred_by: None,
            company_name: None,
            api_key: None,
            status: "ACTIVE".to_string(),
            full_name: "Test".to_string(),
            role: "user".to_string(),
            is_active: true,
            created_at: chrono::Utc::now(),
            updated_at: None,
            username: Some("tester".to_string()),
            permissions: 0,
        };

        let token = facade.gateway_token(&user).expect("token should encode");
        let decoded = decode::<GatewayClaims>(
            &token,
            &DecodingKey::from_secret(b"unit-test-secret"),
            &Validation::default(),
        )
        .expect("token should decode");

        assert_eq!(decoded.claims.sub, 42);
        assert_eq!(decoded.claims.identifier, "test@example.com");
        assert!(decoded.claims.exp > chrono::Utc::now().timestamp() as usize);
    }
}
