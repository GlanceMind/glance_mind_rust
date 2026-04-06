/// API Result wrapper type
///
/// Handler returning `Result<ApiResult<T>, ApiError>` will be automatically wrapped into unified format
///
/// # Example
///
/// ```ignore
/// pub async fn get_user() -> Result<ApiResult<User>, ApiError> {
///     let user = fetch_user().await?;
///     Ok(ApiResult::ok(user))
/// }
/// ```
///
/// Response format:
/// ```json
/// {
///   "code": 1000,
///   "msg": "Success",
///   "msg_cn": "Success",
///   "data": { "id": 1, "name": "John" }
/// }
/// ```
use crate::response::unified_response::ApiResponse;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

/// API Result wrapper
///
/// Used to wrap successful business data, automatically converts to unified API response format
#[derive(Debug)]
pub struct ApiResult<T: Serialize> {
    data: T,
    custom_msg: Option<(String, String)>, // (msg, msg_cn)
}

impl<T: Serialize> ApiResult<T> {
    /// Create success result (using default success message)
    pub fn ok(data: T) -> Self {
        Self {
            data,
            custom_msg: None,
        }
    }

    /// Create success result (with custom message)
    pub fn ok_with_message(data: T, msg: impl Into<String>, msg_cn: impl Into<String>) -> Self {
        Self {
            data,
            custom_msg: Some((msg.into(), msg_cn.into())),
        }
    }

    /// Return success status only (no data)
    pub fn empty() -> ApiResult<()> {
        ApiResult {
            data: (),
            custom_msg: None,
        }
    }

    /// Return success status only (with custom message)
    pub fn empty_with_message(msg: impl Into<String>, msg_cn: impl Into<String>) -> ApiResult<()> {
        ApiResult {
            data: (),
            custom_msg: Some((msg.into(), msg_cn.into())),
        }
    }
}

/// Implement IntoResponse trait for automatic conversion to unified format
impl<T: Serialize> IntoResponse for ApiResult<T> {
    fn into_response(self) -> Response {
        let response = if let Some((msg, msg_cn)) = self.custom_msg {
            ApiResponse::success_with_message(self.data, msg, msg_cn)
        } else {
            ApiResponse::success(self.data)
        };

        response.into_response()
    }
}

/// Convenience macro for simplified ApiResult creation
///
/// # Example
///
/// ```ignore
/// // Return data
/// return Ok(api_result!(user));
///
/// // Return data + custom message
/// return Ok(api_result!(user, "User created", "User created"));
///
/// // Return success status only
/// return Ok(api_result!());
///
/// // Return success status + custom message
/// return Ok(api_result!(msg: "Deleted successfully", "Deleted successfully"));
/// ```
#[macro_export]
macro_rules! api_result {
    // api_result!(data)
    ($data:expr) => {
        $crate::response::api_result::ApiResult::ok($data)
    };

    // api_result!(data, msg, msg_cn)
    ($data:expr, $msg:expr, $msg_cn:expr) => {
        $crate::response::api_result::ApiResult::ok_with_message($data, $msg, $msg_cn)
    };

    // api_result!()
    () => {
        $crate::response::api_result::ApiResult::<()>::empty()
    };

    // api_result!(msg: msg, msg_cn)
    (msg: $msg:expr, $msg_cn:expr) => {
        $crate::response::api_result::ApiResult::<()>::empty_with_message($msg, $msg_cn)
    };
}
