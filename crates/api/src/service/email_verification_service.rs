use crate::config::database::Database;
use crate::dto::email_verification_dto::{
    ResetPasswordRequest, ResetPasswordResponse, SendPasswordResetCodeRequest,
    SendVerificationCodeRequest, SendVerificationCodeResponse, VerifyCodeRequest,
    VerifyCodeResponse,
};
use crate::error::{
    api_error::ApiError, business_error::BusinessError, infrastructure_error::InfrastructureError,
};
use chrono::{Duration, Utc};
use diesel::prelude::*;
use glance_mind_db::entity::email_verification::{EmailVerification, NewEmailVerification};
use glance_mind_db::schema::{gm_email_verifications, gm_users};
use rand::Rng;
use resend_rs::types::{CreateEmailBaseOptions, EmailTemplate};
use resend_rs::Resend;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct EmailVerificationService {
    db: Arc<Database>,
    resend_api_key: String,
    turnstile_secret: String,
}

#[derive(Debug, Deserialize)]
struct TurnstileVerifyResponse {
    success: bool,
    #[serde(rename = "error-codes")]
    error_codes: Option<Vec<String>>,
}

impl EmailVerificationService {
    pub fn new(db: &Arc<Database>) -> Self {
        let resend_api_key = std::env::var("RESEND_API_KEY").unwrap();
        let turnstile_secret = std::env::var("TURNSTILE_SECRET_KEY").unwrap();

        Self {
            db: db.clone(),
            resend_api_key,
            turnstile_secret,
        }
    }

    /// Verify Turnstile token
    async fn verify_turnstile(&self, token: &str) -> Result<bool, ApiError> {
        if self.turnstile_secret.trim().is_empty() {
            tracing::info!("Skipping Turnstile verification because TURNSTILE_SECRET_KEY is empty");
            return Ok(true);
        }

        if token.trim().is_empty() {
            tracing::warn!("Turnstile token missing while verification is enabled");
            return Ok(false);
        }

        let client = reqwest::Client::new();

        let response = client
            .post("https://challenges.cloudflare.com/turnstile/v0/siteverify")
            .json(&serde_json::json!({
                "secret": self.turnstile_secret,
                "response": token,
            }))
            .send()
            .await
            .map_err(|e| {
                ApiError::InfrastructureError(InfrastructureError::TurnstileVerificationFailed(
                    e.to_string(),
                ))
            })?;

        let result: TurnstileVerifyResponse = response.json().await.map_err(|e| {
            ApiError::InfrastructureError(InfrastructureError::TurnstileVerificationFailed(
                e.to_string(),
            ))
        })?;

        if !result.success {
            tracing::warn!("Turnstile verification failed: {:?}", result.error_codes);
        }

        Ok(result.success)
    }

    /// Generate 6-digit verification code
    fn generate_code() -> String {
        let mut rng = rand::rng();
        format!("{:06}", rng.random_range(0..1000000))
    }

    /// Send verification code email (using Resend template)
    async fn send_email(&self, email: &str, code: &str) -> Result<(), ApiError> {
        if self.resend_api_key.trim().is_empty() {
            tracing::info!(
                "Skipping verification email delivery because RESEND_API_KEY is empty: email={}, code={}",
                email,
                code
            );
            return Ok(());
        }

        let resend = Resend::new(&self.resend_api_key);

        // Send email using Resend template
        let from = "GlanceMind <noreply@glancemind.org>";
        let to = vec![email];
        let subject = "Verify Your GlanceMind Account"; // Template email also needs subject

        // Prepare template variables
        let mut variables = HashMap::new();
        variables.insert("code".to_string(), serde_json::json!(code));

        // Create template object
        let template = EmailTemplate::new("account-verification-code").with_variables(variables);

        // Send email with template
        let email_options = CreateEmailBaseOptions::new(from, to, subject).with_template(template);

        resend.emails.send(email_options).await.map_err(|e| {
            ApiError::InfrastructureError(InfrastructureError::EmailServiceError(e.to_string()))
        })?;

        Ok(())
    }

    /// Send verification code
    pub async fn send_verification_code(
        &self,
        request: SendVerificationCodeRequest,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Result<SendVerificationCodeResponse, ApiError> {
        // 1. Verify Turnstile
        if !self.verify_turnstile(&request.turnstile_token).await? {
            return Ok(SendVerificationCodeResponse {
                success: false,
                message: "Turnstile verification failed".to_string(),
                expires_in_minutes: None,
            });
        }

        // 2. Check if code was sent recently (prevent abuse)
        let mut conn = self.db.pool.get().map_err(|_| {
            ApiError::InfrastructureError(InfrastructureError::DatabaseConnectionFailed)
        })?;

        let recent_verification = gm_email_verifications::table
            .filter(gm_email_verifications::email.eq(&request.email))
            .filter(gm_email_verifications::created_at.gt(Utc::now() - Duration::minutes(1)))
            .first::<EmailVerification>(&mut conn)
            .optional()
            .map_err(|e| {
                ApiError::InfrastructureError(InfrastructureError::DatabaseOperationFailed(
                    e.to_string(),
                ))
            })?;

        if recent_verification.is_some() {
            return Ok(SendVerificationCodeResponse {
                success: false,
                message: "Please wait 1 minute before requesting a new code".to_string(),
                expires_in_minutes: None,
            });
        }

        // 3. Generate verification code
        let code = Self::generate_code();
        let expires_at = Utc::now() + Duration::minutes(10);

        // 4. Save to database
        let new_verification = NewEmailVerification {
            email: request.email.clone(),
            code: code.clone(),
            expires_at,
            ip_address,
            user_agent,
        };

        diesel::insert_into(gm_email_verifications::table)
            .values(&new_verification)
            .execute(&mut conn)
            .map_err(|e| {
                ApiError::InfrastructureError(InfrastructureError::DatabaseOperationFailed(
                    e.to_string(),
                ))
            })?;

        // 5. Send email
        self.send_email(&request.email, &code).await?;

        Ok(SendVerificationCodeResponse {
            success: true,
            message: "Verification code sent successfully".to_string(),
            expires_in_minutes: Some(10),
        })
    }

    /// Verify verification code
    pub async fn verify_code(
        &self,
        request: VerifyCodeRequest,
    ) -> Result<VerifyCodeResponse, ApiError> {
        let mut conn = self.db.pool.get().map_err(|_| {
            ApiError::InfrastructureError(InfrastructureError::DatabaseConnectionFailed)
        })?;

        // Find the most recent unverified code
        let verification = gm_email_verifications::table
            .filter(gm_email_verifications::email.eq(&request.email))
            .filter(gm_email_verifications::code.eq(&request.code))
            .filter(gm_email_verifications::verified.eq(false))
            .order(gm_email_verifications::created_at.desc())
            .first::<EmailVerification>(&mut conn)
            .optional()
            .map_err(|e| {
                ApiError::InfrastructureError(InfrastructureError::DatabaseOperationFailed(
                    e.to_string(),
                ))
            })?;

        let verification = match verification {
            Some(v) => v,
            None => {
                return Ok(VerifyCodeResponse {
                    success: false,
                    message: "Invalid verification code".to_string(),
                    email: None,
                });
            }
        };

        // Check if expired
        if verification.expires_at < Utc::now() {
            return Ok(VerifyCodeResponse {
                success: false,
                message: "Verification code has expired".to_string(),
                email: None,
            });
        }

        // Mark as verified
        diesel::update(gm_email_verifications::table)
            .filter(gm_email_verifications::id.eq(verification.id))
            .set(gm_email_verifications::verified.eq(true))
            .execute(&mut conn)
            .map_err(|e| {
                ApiError::InfrastructureError(InfrastructureError::DatabaseOperationFailed(
                    e.to_string(),
                ))
            })?;

        Ok(VerifyCodeResponse {
            success: true,
            message: "Email verified successfully".to_string(),
            email: Some(verification.email),
        })
    }

    /// Send password reset email (reuse registration verification code template)
    async fn send_password_reset_email(&self, email: &str, code: &str) -> Result<(), ApiError> {
        if self.resend_api_key.trim().is_empty() {
            tracing::info!(
                "Skipping password reset email delivery because RESEND_API_KEY is empty: email={}, code={}",
                email,
                code
            );
            return Ok(());
        }

        let resend = Resend::new(&self.resend_api_key);

        let from = "GlanceMind <noreply@glancemind.org>";
        let to = vec![email];
        let subject = "Reset Your GlanceMind Password";

        // Prepare template variables
        let mut variables = HashMap::new();
        variables.insert("code".to_string(), serde_json::json!(code));

        // Reuse registration verification code template
        let template = EmailTemplate::new("account-verification-code").with_variables(variables);
        let email_options = CreateEmailBaseOptions::new(from, to, subject).with_template(template);

        resend.emails.send(email_options).await.map_err(|e| {
            ApiError::InfrastructureError(InfrastructureError::EmailServiceError(e.to_string()))
        })?;

        Ok(())
    }

    /// Send password reset verification code
    pub async fn send_password_reset_code(
        &self,
        request: SendPasswordResetCodeRequest,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Result<SendVerificationCodeResponse, ApiError> {
        // 1. Verify Turnstile
        if !self.verify_turnstile(&request.turnstile_token).await? {
            return Ok(SendVerificationCodeResponse {
                success: false,
                message: "Turnstile verification failed".to_string(),
                expires_in_minutes: None,
            });
        }

        let mut conn = self.db.pool.get().map_err(|_| {
            ApiError::InfrastructureError(InfrastructureError::DatabaseConnectionFailed)
        })?;

        // 2. Check if email exists (must be a registered user)
        let user_count: i64 = gm_users::table
            .filter(gm_users::email.eq(&request.email))
            .count()
            .get_result(&mut conn)
            .map_err(|e| {
                ApiError::InfrastructureError(InfrastructureError::DatabaseOperationFailed(
                    e.to_string(),
                ))
            })?;

        if user_count == 0 {
            // For security, don't reveal if email exists, return same success message
            return Ok(SendVerificationCodeResponse {
                success: true,
                message: "If an account with that email exists, a reset code has been sent"
                    .to_string(),
                expires_in_minutes: Some(10),
            });
        }

        // 3. Check if code was sent recently (prevent abuse)
        let recent_verification = gm_email_verifications::table
            .filter(gm_email_verifications::email.eq(&request.email))
            .filter(gm_email_verifications::created_at.gt(Utc::now() - Duration::minutes(1)))
            .first::<EmailVerification>(&mut conn)
            .optional()
            .map_err(|e| {
                ApiError::InfrastructureError(InfrastructureError::DatabaseOperationFailed(
                    e.to_string(),
                ))
            })?;

        if recent_verification.is_some() {
            return Ok(SendVerificationCodeResponse {
                success: false,
                message: "Please wait 1 minute before requesting a new code".to_string(),
                expires_in_minutes: None,
            });
        }

        // 4. Generate verification code
        let code = Self::generate_code();
        let expires_at = Utc::now() + Duration::minutes(10);

        // 5. Save to database
        let new_verification = NewEmailVerification {
            email: request.email.clone(),
            code: code.clone(),
            expires_at,
            ip_address,
            user_agent,
        };

        diesel::insert_into(gm_email_verifications::table)
            .values(&new_verification)
            .execute(&mut conn)
            .map_err(|e| {
                ApiError::InfrastructureError(InfrastructureError::DatabaseOperationFailed(
                    e.to_string(),
                ))
            })?;

        // 6. Send email
        self.send_password_reset_email(&request.email, &code)
            .await?;

        Ok(SendVerificationCodeResponse {
            success: true,
            message: "Password reset code sent successfully".to_string(),
            expires_in_minutes: Some(10),
        })
    }

    /// Reset password
    pub async fn reset_password(
        &self,
        request: ResetPasswordRequest,
    ) -> Result<ResetPasswordResponse, ApiError> {
        let mut conn = self.db.pool.get().map_err(|_| {
            ApiError::InfrastructureError(InfrastructureError::DatabaseConnectionFailed)
        })?;

        // 1. Verify verification code
        let verification = gm_email_verifications::table
            .filter(gm_email_verifications::email.eq(&request.email))
            .filter(gm_email_verifications::code.eq(&request.code))
            .filter(gm_email_verifications::verified.eq(false))
            .order(gm_email_verifications::created_at.desc())
            .first::<EmailVerification>(&mut conn)
            .optional()
            .map_err(|e| {
                ApiError::InfrastructureError(InfrastructureError::DatabaseOperationFailed(
                    e.to_string(),
                ))
            })?;

        let verification = match verification {
            Some(v) => v,
            None => {
                return Ok(ResetPasswordResponse {
                    success: false,
                    message: "Invalid verification code".to_string(),
                });
            }
        };

        // 2. Check if verification code expired
        if verification.expires_at < Utc::now() {
            return Ok(ResetPasswordResponse {
                success: false,
                message: "Verification code has expired".to_string(),
            });
        }

        // 3. Update password (directly by email)
        let new_password_hash = bcrypt::hash(&request.new_password, 4)
            .map_err(|_e| ApiError::BusinessError(BusinessError::PasswordHashFailed))?;

        let updated_rows = diesel::update(gm_users::table)
            .filter(gm_users::email.eq(&request.email))
            .set(gm_users::password_hash.eq(new_password_hash))
            .execute(&mut conn)
            .map_err(|e| {
                ApiError::InfrastructureError(InfrastructureError::DatabaseOperationFailed(
                    e.to_string(),
                ))
            })?;

        if updated_rows == 0 {
            return Ok(ResetPasswordResponse {
                success: false,
                message: "User not found".to_string(),
            });
        }

        // 5. Mark verification code as used
        diesel::update(gm_email_verifications::table)
            .filter(gm_email_verifications::id.eq(verification.id))
            .set(gm_email_verifications::verified.eq(true))
            .execute(&mut conn)
            .map_err(|e| {
                ApiError::InfrastructureError(InfrastructureError::DatabaseOperationFailed(
                    e.to_string(),
                ))
            })?;

        Ok(ResetPasswordResponse {
            success: true,
            message: "Password reset successfully".to_string(),
        })
    }
}
