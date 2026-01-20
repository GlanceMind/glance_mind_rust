use crate::response::error_code::ErrorCode;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum InfrastructureError {
    #[error("Database connection failed")]
    DatabaseConnectionFailed,

    #[error("Database operation failed: {0}")]
    DatabaseOperationFailed(String),

    #[error("External API request failed: {0}")]
    ExternalApiRequestFailed(String),

    #[error("External API response parsing failed: {0}")]
    ExternalApiResponseParsingFailed(String),

    #[error("Email service error: {0}")]
    EmailServiceError(String),

    #[error("Turnstile verification failed: {0}")]
    TurnstileVerificationFailed(String),
}

impl InfrastructureError {
    pub fn to_error_code(&self) -> ErrorCode {
        match self {
            InfrastructureError::DatabaseConnectionFailed => ErrorCode::DatabaseError,
            InfrastructureError::DatabaseOperationFailed(_) => ErrorCode::DatabaseError,
            InfrastructureError::ExternalApiRequestFailed(_) => ErrorCode::InternalServerError,
            InfrastructureError::ExternalApiResponseParsingFailed(_) => {
                ErrorCode::InternalServerError
            }
            InfrastructureError::EmailServiceError(_) => ErrorCode::EmailServiceError,
            InfrastructureError::TurnstileVerificationFailed(_) => ErrorCode::BadRequest,
        }
    }

    pub fn to_message_cn(&self) -> String {
        match self {
            InfrastructureError::DatabaseConnectionFailed => "数据库连接失败".to_string(),
            InfrastructureError::DatabaseOperationFailed(msg) => {
                format!("数据库操作失败: {}", msg)
            }
            InfrastructureError::ExternalApiRequestFailed(msg) => {
                format!("外部API请求失败: {}", msg)
            }
            InfrastructureError::ExternalApiResponseParsingFailed(msg) => {
                format!("外部API响应解析失败: {}", msg)
            }
            InfrastructureError::EmailServiceError(msg) => format!("邮件服务错误: {}", msg),
            InfrastructureError::TurnstileVerificationFailed(msg) => {
                format!("Turnstile验证失败: {}", msg)
            }
        }
    }
}
