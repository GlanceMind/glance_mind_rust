use chrono::{DateTime, Utc};
use glance_mind_db::entity::user::User;
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
    /// Contact phone — required at registration, must be an 11-digit
    /// mainland-China mobile number (validated in `validate`).
    pub phone: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invitation_code: Option<String>,
}

/// True when `phone` is an 11-digit mainland-China mobile number:
/// exactly 11 ASCII digits, first digit `1`, second digit in `3..=9`.
/// Mirrors the frontend regex `^1[3-9]\d{9}$` without pulling in a regex compile.
pub fn is_valid_cn_mobile(phone: &str) -> bool {
    let bytes = phone.as_bytes();
    bytes.len() == 11
        && bytes[0] == b'1'
        && (b'3'..=b'9').contains(&bytes[1])
        && bytes.iter().all(u8::is_ascii_digit)
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

        // Validate contact phone — required 11-digit mainland-China mobile (^1[3-9]\d{9}$).
        if !is_valid_cn_mobile(&self.phone) {
            return Err(
                "Invalid phone number: must be an 11-digit Chinese mobile number".to_string(),
            );
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
    pub phone: Option<String>,
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
            phone: user.phone,
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
            .field("phone", &self.phone)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_dto(phone: &str) -> UserRegisterDto {
        UserRegisterDto {
            email: "user@example.com".to_string(),
            username: "valid_user".to_string(),
            password: "password123".to_string(),
            phone: phone.to_string(),
            invitation_code: None,
        }
    }

    #[test]
    fn accepts_valid_cn_mobile() {
        assert!(base_dto("13800138000").validate().is_ok());
        assert!(base_dto("19912345678").validate().is_ok());
    }

    #[test]
    fn rejects_phone_with_wrong_length() {
        // 10 digits and 12 digits must both be rejected.
        assert!(base_dto("1380013800").validate().is_err());
        assert!(base_dto("138001380000").validate().is_err());
    }

    #[test]
    fn rejects_phone_with_bad_prefix() {
        // Must start with 1 and have a second digit in 3..=9.
        assert!(base_dto("23800138000").validate().is_err());
        assert!(base_dto("12800138000").validate().is_err());
    }

    #[test]
    fn rejects_non_digit_and_empty_phone() {
        assert!(base_dto("1380013800a").validate().is_err());
        assert!(base_dto("138 0013 800").validate().is_err());
        assert!(base_dto("").validate().is_err());
    }

    #[test]
    fn is_valid_cn_mobile_helper_matches_regex_semantics() {
        assert!(is_valid_cn_mobile("13012345678"));
        assert!(is_valid_cn_mobile("18999999999"));
        assert!(!is_valid_cn_mobile("10012345678")); // second digit 0
        assert!(!is_valid_cn_mobile("1301234567")); // 10 digits
        assert!(!is_valid_cn_mobile("+8613012345678")); // country code
    }
}
