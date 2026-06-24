use crate::config::database::Database;
use crate::config::parameter;
use crate::dto::user_dto::{
    TokenClaimsDto, TokenReadDto, UserAuthResponseDto, UserReadDto, UserRegisterDto,
};
use crate::error::db_error::DbError;
use crate::error::token_error::TokenError;
use crate::error::user_error::UserError;
use crate::error::{api_error::ApiError, business_error::BusinessError};
use crate::repository::user_repository::UserRepositoryTrait;
use crate::repository::wallet_repository::WalletRepository;
use bigdecimal::BigDecimal;
use diesel::result::Error as DieselError;
use glance_mind_db::entity::user::User;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, TokenData, Validation};
use rand::{distr::Alphanumeric, Rng};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct UserService<R: UserRepositoryTrait> {
    pub user_repo: R,
    pub wallet_repo: WalletRepository,
    pub secret: String,
}

impl<R: UserRepositoryTrait> UserService<R> {
    pub fn new(db_conn: &Arc<Database>, user_repo: R) -> Self {
        Self {
            user_repo,
            wallet_repo: WalletRepository::new(db_conn.pool.clone()),
            secret: parameter::get("JWT_SECRET"),
        }
    }

    pub async fn create_user(
        &self,
        payload: UserRegisterDto,
        db: &Arc<Database>,
    ) -> Result<UserAuthResponseDto, ApiError> {
        // Validate the payload
        payload.validate().map_err(ApiError::BadRequest)?;

        // Check if email is verified
        use diesel::prelude::*;
        use glance_mind_db::entity::email_verification::EmailVerification;
        use glance_mind_db::schema::gm_email_verifications;

        let mut conn = db
            .pool
            .get()
            .map_err(|_| ApiError::InternalServerError("Database connection failed".to_string()))?;

        let verified = gm_email_verifications::table
            .filter(gm_email_verifications::email.eq(&payload.email))
            .filter(gm_email_verifications::verified.eq(true))
            .filter(gm_email_verifications::expires_at.gt(chrono::Utc::now()))
            .first::<EmailVerification>(&mut conn)
            .optional()
            .map_err(|e| ApiError::InternalServerError(format!("Database error: {}", e)))?;

        if verified.is_none() {
            return Err(ApiError::BadRequest(
                "Email not verified. Please verify your email first.".to_string(),
            ));
        }

        // Check if email already exists
        if self
            .user_repo
            .find_by_email(payload.email.clone())
            .await
            .is_some()
        {
            return Err(ApiError::from(UserError::UserAlreadyExists));
        }

        // Check if username already exists
        if self
            .user_repo
            .find_by_username(payload.username.clone())
            .await
            .is_some()
        {
            return Err(ApiError::BusinessError(
                BusinessError::UsernameAlreadyExists,
            ));
        }

        let has_referral = payload.invitation_code.is_some();
        let user = self.add_user(payload).await?;

        // Give welcome bonus if user was invited
        if has_referral {
            if let Err(e) = self.give_welcome_bonus(user.id).await {
                tracing::warn!("Failed to give welcome bonus to user {}: {:?}", user.id, e);
                // Don't fail registration if bonus fails
            }
        }

        let token = self.generate_token(user.clone())?;

        Ok(UserAuthResponseDto {
            user: UserReadDto::from(user),
            token,
        })
    }

    async fn add_user(&self, payload: UserRegisterDto) -> Result<User, ApiError> {
        let password_hash = bcrypt::hash(&payload.password, 4)
            .map_err(|_| DbError::SomethingWentWrong("Failed to hash password".to_string()))?;

        // Generate unique invite code for new user
        let invite_code = Uuid::new_v4().to_string();

        self.user_repo
            .create(
                Some(payload.email),
                Some(payload.username),
                password_hash,
                Some(invite_code),
                payload.invitation_code,
                Some(payload.phone),
            )
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))
    }

    /// Give 300 points welcome bonus to invited users
    async fn give_welcome_bonus(&self, user_id: i32) -> Result<(), ApiError> {
        let bonus_amount = BigDecimal::from(300);

        self.wallet_repo
            .add_balance_with_transaction(
                user_id,
                bonus_amount,
                "WELCOME_BONUS".to_string(),
                "Welcome bonus for new invited user".to_string(),
            )
            .await
            .map_err(|e| {
                tracing::error!("Failed to add welcome bonus: {:?}", e);
                ApiError::InternalServerError("Failed to add welcome bonus".to_string())
            })?;

        tracing::info!("Welcome bonus granted to user {}", user_id);
        Ok(())
    }

    pub fn verify_password(&self, user: &User, password: &str) -> bool {
        bcrypt::verify(password, &user.password_hash).unwrap_or(false)
    }

    pub fn generate_token(&self, user: User) -> Result<TokenReadDto, TokenError> {
        let iat = chrono::Utc::now().timestamp();
        let exp = chrono::Utc::now()
            .checked_add_signed(chrono::Duration::weeks(1)) // Updated to 1 week by user request
            .unwrap()
            .timestamp();

        // Use email if available, otherwise username
        let identifier = user
            .email
            .clone()
            .or_else(|| user.username.clone())
            .ok_or_else(|| {
                TokenError::TokenCreationError("User has neither email nor username".to_string())
            })?;

        let claims = TokenClaimsDto {
            sub: user.id,
            identifier,
            iat,
            exp,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_ref()),
        )
        .map_err(|e| TokenError::TokenCreationError(e.to_string()))?;

        Ok(TokenReadDto { token, iat, exp })
    }

    pub fn retrieve_token_claims(
        &self,
        token: &str,
    ) -> jsonwebtoken::errors::Result<TokenData<TokenClaimsDto>> {
        let result = decode::<TokenClaimsDto>(
            token,
            &DecodingKey::from_secret(self.secret.as_ref()),
            &Validation::default(),
        );

        result
    }

    pub async fn update_profile(
        &self,
        user_id: i32,
        payload: crate::dto::user_dto::UserUpdateProfileDto,
    ) -> Result<UserReadDto, ApiError> {
        let mut user = self.user_repo.find(user_id).await.map_err(|e| match e {
            // Diesel doesn't have explicit RowNotFound in top-level, it's usually Error::NotFound
            DieselError::NotFound => ApiError::from(UserError::UserNotFound),
            _ => ApiError::from(DbError::SomethingWentWrong(e.to_string())),
        })?;

        if let Some(company) = payload.company_name {
            user.company_name = Some(company);
        }
        if let Some(key) = payload.api_key {
            user.api_key = Some(key);
        }

        let updated_user = self
            .user_repo
            .update(user)
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;
        Ok(UserReadDto::from(updated_user))
    }

    pub async fn change_password(
        &self,
        user_id: i32,
        payload: crate::dto::user_dto::UserUpdatePasswordDto,
    ) -> Result<(), ApiError> {
        let mut user = self.user_repo.find(user_id).await.map_err(|e| match e {
            DieselError::NotFound => ApiError::from(UserError::UserNotFound),
            _ => ApiError::from(DbError::SomethingWentWrong(e.to_string())),
        })?;

        if !self.verify_password(&user, &payload.old_password) {
            return Err(ApiError::from(UserError::InvalidPassword));
        }

        let new_hash = bcrypt::hash(&payload.new_password, 4)
            .map_err(|_e| DieselError::RollbackTransaction)
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;
        user.password_hash = new_hash;

        self.user_repo
            .update(user)
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;
        Ok(())
    }
    pub async fn regenerate_api_key(&self, user_id: i32) -> Result<String, ApiError> {
        let mut user = self.user_repo.find(user_id).await.map_err(|e| match e {
            DieselError::NotFound => ApiError::from(UserError::UserNotFound),
            _ => ApiError::from(DbError::SomethingWentWrong(e.to_string())),
        })?;

        let new_key: String = rand::rng()
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();
        let api_key = format!("sk_{}", new_key);

        user.api_key = Some(api_key.clone());

        self.user_repo
            .update(user)
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        Ok(api_key)
    }
}
