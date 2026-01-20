use super::error_code::ErrorCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};

/// Unified API response structure
///
/// Success response example:
/// ```json
/// {
///   "code": 1000,
///   "msg": "Success",
///   "msg_cn": "Success",
///   "data": { ... }
/// }
/// ```
///
/// Error response example:
/// ```json
/// {
///   "code": 2000,
///   "msg": "Validation error",
///   "msg_cn": "Validation failed",
///   "data": null
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T = ()>
where
    T: Serialize,
{
    /// Business error code
    pub code: i32,
    /// English error message
    pub msg: String,
    /// Chinese error message
    pub msg_cn: String,
    /// Response data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

impl<T> ApiResponse<T>
where
    T: Serialize,
{
    /// Create success response (with data)
    pub fn success(data: T) -> Self {
        Self {
            code: ErrorCode::Success.code(),
            msg: ErrorCode::Success.message().to_string(),
            msg_cn: ErrorCode::Success.message_cn().to_string(),
            data: Some(data),
        }
    }

    /// Create success response (with custom message)
    pub fn success_with_message(
        data: T,
        msg: impl Into<String>,
        msg_cn: impl Into<String>,
    ) -> Self {
        Self {
            code: ErrorCode::Success.code(),
            msg: msg.into(),
            msg_cn: msg_cn.into(),
            data: Some(data),
        }
    }

    /// Create error response
    pub fn error(error_code: ErrorCode) -> Self {
        Self {
            code: error_code.code(),
            msg: error_code.message().to_string(),
            msg_cn: error_code.message_cn().to_string(),
            data: None,
        }
    }

    /// Create error response (with custom message)
    pub fn error_with_message(
        error_code: ErrorCode,
        msg: impl Into<String>,
        msg_cn: impl Into<String>,
    ) -> Self {
        Self {
            code: error_code.code(),
            msg: msg.into(),
            msg_cn: msg_cn.into(),
            data: None,
        }
    }

    /// Create response from Result
    pub fn from_result(result: Result<T, crate::error::api_error::ApiError>) -> Self {
        match result {
            Ok(data) => Self::success(data),
            Err(err) => {
                let error_code = err.to_error_code();
                Self {
                    code: error_code.code(),
                    msg: err.to_string(),
                    msg_cn: err.to_message_cn(),
                    data: None,
                }
            }
        }
    }
}

// Implement special methods for empty data type
impl ApiResponse<()> {
    /// Create success response (no data)
    pub fn ok() -> Self {
        Self {
            code: ErrorCode::Success.code(),
            msg: ErrorCode::Success.message().to_string(),
            msg_cn: ErrorCode::Success.message_cn().to_string(),
            data: None,
        }
    }

    /// Create success response (no data, with custom message)
    pub fn ok_with_message(msg: impl Into<String>, msg_cn: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::Success.code(),
            msg: msg.into(),
            msg_cn: msg_cn.into(),
            data: None,
        }
    }
}

// Implement IntoResponse trait for Axum automatic handling
impl<T> IntoResponse for ApiResponse<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        let error_code = ErrorCode::from(self.code);
        let status = error_code.http_status();

        (status, Json(self)).into_response()
    }
}

// Implement From<i32> for ErrorCode
impl From<i32> for ErrorCode {
    fn from(code: i32) -> Self {
        // Try to match error code, return InternalServerError if no match
        match code {
            1000 => ErrorCode::Success,
            2000 => ErrorCode::ValidationError,
            2001 => ErrorCode::BadRequest,
            2002 => ErrorCode::Unauthorized,
            2003 => ErrorCode::Forbidden,
            2004 => ErrorCode::NotFound,
            2005 => ErrorCode::MethodNotAllowed,
            2006 => ErrorCode::TooManyRequests,
            2007 => ErrorCode::TokenExpired,
            2008 => ErrorCode::TokenInvalid,
            3000 => ErrorCode::InternalServerError,
            3001 => ErrorCode::DatabaseError,
            3002 => ErrorCode::CacheError,
            3003 => ErrorCode::ConfigError,
            4000 => ErrorCode::UserNotFound,
            4001 => ErrorCode::UserAlreadyExists,
            4002 => ErrorCode::InvalidPassword,
            4003 => ErrorCode::EmailAlreadyInUse,
            4004 => ErrorCode::InvalidVerificationCode,
            4005 => ErrorCode::VerificationCodeExpired,
            4100 => ErrorCode::InsufficientBalance,
            4101 => ErrorCode::ChargeFailed,
            4102 => ErrorCode::PricingRuleNotFound,
            4200 => ErrorCode::TemplateNotFound,
            4201 => ErrorCode::CampaignNotFound,
            4202 => ErrorCode::TaskNotFound,
            4203 => ErrorCode::VideoNotFound,
            4300 => ErrorCode::SocialAccountNotFound,
            4301 => ErrorCode::SocialAccountAlreadyBound,
            4400 => ErrorCode::PromoCodeNotFound,
            4401 => ErrorCode::PromoCodeExpired,
            4402 => ErrorCode::PromoCodeAlreadyUsed,
            4403 => ErrorCode::PromoCodeUsageLimitReached,
            5000 => ErrorCode::AiServiceError,
            5001 => ErrorCode::EmailServiceError,
            5002 => ErrorCode::PaymentServiceError,
            5100 => ErrorCode::TikTokApiError,
            5101 => ErrorCode::InstagramApiError,
            5102 => ErrorCode::TwitterApiError,
            5103 => ErrorCode::RedditApiError,
            _ => ErrorCode::InternalServerError,
        }
    }
}

// Implement automatic conversion from Result
impl<T, E> From<Result<T, E>> for ApiResponse<T>
where
    T: Serialize,
    E: Into<crate::error::api_error::ApiError>,
{
    fn from(result: Result<T, E>) -> Self {
        match result {
            Ok(data) => Self::success(data),
            Err(err) => {
                let api_error: crate::error::api_error::ApiError = err.into();
                let error_code = api_error.to_error_code();
                Self {
                    code: error_code.code(),
                    msg: api_error.to_string(),
                    msg_cn: api_error.to_message_cn(),
                    data: None,
                }
            }
        }
    }
}

/// Convenience macro: Create success response
///
/// # Examples
///
/// ```
/// // With data
/// api_ok!(user_data)
///
/// // No data
/// api_ok!()
///
/// // With custom message
/// api_ok!(user_data, "User created", "User created successfully")
/// ```
#[macro_export]
macro_rules! api_ok {
    // No data, no custom message
    () => {
        $crate::response::unified_response::ApiResponse::ok()
    };

    // With data, no custom message
    ($data:expr) => {
        $crate::response::unified_response::ApiResponse::success($data)
    };

    // With data, with custom message
    ($data:expr, $msg:expr, $msg_cn:expr) => {
        $crate::response::unified_response::ApiResponse::success_with_message($data, $msg, $msg_cn)
    };

    // No data, with custom message
    (msg: $msg:expr, $msg_cn:expr) => {
        $crate::response::unified_response::ApiResponse::ok_with_message($msg, $msg_cn)
    };
}

/// Convenience macro: Create error response
///
/// # Examples
///
/// ```
/// // Using error code
/// api_err!(ErrorCode::UserNotFound)
///
/// // Using error code and custom message
/// api_err!(ErrorCode::BadRequest, "Invalid email format", "Invalid email format")
/// ```
#[macro_export]
macro_rules! api_err {
    // Error code only
    ($error_code:expr) => {
        $crate::response::unified_response::ApiResponse::<()>::error($error_code)
    };

    // Error code + custom message
    ($error_code:expr, $msg:expr, $msg_cn:expr) => {
        $crate::response::unified_response::ApiResponse::<()>::error_with_message(
            $error_code,
            $msg,
            $msg_cn,
        )
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_success_response() {
        let response = ApiResponse::success(json!({"user_id": 123}));
        assert_eq!(response.code, 1000);
        assert_eq!(response.msg, "Success");
        assert!(response.data.is_some());
    }

    #[test]
    fn test_error_response() {
        let response = ApiResponse::<()>::error(ErrorCode::UserNotFound);
        assert_eq!(response.code, 4000);
        assert_eq!(response.msg, "User not found");
        assert_eq!(response.msg_cn, "User not found");
        assert!(response.data.is_none());
    }

    #[test]
    fn test_ok_macro() {
        let response = api_ok!();
        assert_eq!(response.code, 1000);
        assert!(response.data.is_none());
    }

    #[test]
    fn test_err_macro() {
        let response = api_err!(ErrorCode::BadRequest);
        assert_eq!(response.code, 2001);
    }
}
