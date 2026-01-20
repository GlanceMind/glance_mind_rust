use serde::{Deserialize, Serialize};
use validator::Validate;

/// Google OAuth2 login request
#[derive(Debug, Deserialize, Validate)]
pub struct GoogleAuthDto {
    /// Google ID token from frontend
    #[validate(length(min = 1, message = "ID token is required"))]
    pub id_token: String,
}

/// Decoded Google ID token payload
#[derive(Debug, Deserialize, Serialize)]
pub struct GoogleTokenPayload {
    /// Issuer (should be accounts.google.com or https://accounts.google.com)
    pub iss: String,

    /// Subject (Google user ID)
    pub sub: String,

    /// Audience (should be your Google Client ID)
    pub aud: String,

    /// Expiration time
    pub exp: i64,

    /// Issued at time
    pub iat: i64,

    /// Email address
    pub email: String,

    /// Email verified
    pub email_verified: bool,

    /// User's full name
    #[serde(default)]
    pub name: Option<String>,

    /// User's profile picture URL
    #[serde(default)]
    pub picture: Option<String>,

    /// User's given name
    #[serde(default)]
    pub given_name: Option<String>,

    /// User's family name
    #[serde(default)]
    pub family_name: Option<String>,

    /// User's locale
    #[serde(default)]
    pub locale: Option<String>,
}
