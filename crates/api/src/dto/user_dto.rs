use glance_mind_db::entity::user::User;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Clone, Serialize, Deserialize, Validate)]
pub struct UserLoginDto {
    // Can be either email or username
    #[validate(length(min = 3, max = 255))]
    pub identifier: String,
    #[validate(length(
        min = 3,
        max = 20,
        message = "Password must be between 3 and 20 characters"
    ))]
    pub password: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct UserRegisterDto {
    pub email: String,
    pub username: String,
    pub password: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invitation_code: Option<String>,
}

impl UserRegisterDto {
    pub fn validate(&self) -> Result<(), String> {
        // Validate email format
        if !self.email.contains('@') || self.email.len() < 5 {
            return Err("Invalid email format".to_string());
        }

        // Validate username format
        if self.username.len() < 3 || self.username.len() > 20 {
            return Err("Username must be between 3 and 20 characters".to_string());
        }
        // Only allow alphanumeric and underscore
        if !self
            .username
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_')
        {
            return Err("Username can only contain letters, numbers, and underscores".to_string());
        }

        // Validate password
        if self.password.len() < 6 || self.password.len() > 50 {
            return Err("Password must be between 6 and 50 characters".to_string());
        }

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Validate)]
pub struct UserUpdateProfileDto {
    pub company_name: Option<String>,
    // Add other profile fields if needed (e.g. nickname, avatar url)
    pub api_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Validate)]
pub struct UserUpdatePasswordDto {
    #[validate(length(min = 6, max = 50))]
    pub old_password: String,
    #[validate(length(min = 6, max = 50))]
    pub new_password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponseDto {
    pub token: String,
    pub user: UserReadDto,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChangePasswordDto {
    pub old_password: String,
    pub new_password: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserReadDto {
    pub id: i32,
    pub email: Option<String>,
    pub username: Option<String>,
    pub invitation_code: Option<String>,
    pub referred_by: Option<String>,
    pub company_name: Option<String>,
    pub api_key: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TokenReadDto {
    pub token: String,
    pub iat: i64,
    pub exp: i64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct UserAuthResponseDto {
    pub user: UserReadDto,
    pub token: TokenReadDto,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TokenClaimsDto {
    pub sub: i32,
    pub identifier: String, // Can be email or username
    pub iat: i64,
    pub exp: i64,
}

impl From<User> for UserReadDto {
    fn from(user: User) -> Self {
        UserReadDto {
            id: user.id,
            email: user.email,
            username: user.username,
            invitation_code: user.invitation_code,
            referred_by: user.referred_by,
            company_name: user.company_name,
            api_key: user.api_key,
            status: user.status,
            created_at: user.created_at,
            updated_at: user.updated_at,
        }
    }
}

impl std::fmt::Debug for UserLoginDto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("User")
            .field("identifier", &self.identifier)
            .finish()
    }
}

impl std::fmt::Debug for UserRegisterDto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("User")
            .field("email", &self.email)
            .field("username", &self.username)
            .finish()
    }
}
