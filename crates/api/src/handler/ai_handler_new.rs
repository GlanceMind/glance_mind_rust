/// New AI Handler example
/// Demonstrates how to use unified ApiResponse format
use crate::dto::ai_dto::{AiGenerateRequest, AiGenerateResponse};
use crate::error::{api_error::ApiError, business_error::BusinessError};
use crate::response::error_code::ErrorCode;
use crate::response::unified_response::ApiResponse;
use crate::service::ai_service::AiService;
use crate::{api_err, api_ok};
use axum::{response::IntoResponse, Json};
use validator::Validate;

/// Method 1: Use Result + ? operator (recommended)
///
/// Advantage: Clean code, automatic error propagation
pub async fn generate_content_v1(
    Json(payload): Json<AiGenerateRequest>,
) -> Result<impl IntoResponse, ApiError> {
    // Validate request parameters
    payload
        .validate()
        .map_err(|e| ApiError::ValidationError(e.to_string()))?;

    // Call service
    let response = AiService::generate(payload)
        .await
        .map_err(|e| ApiError::AiServiceError(e))?;

    // Return success response
    Ok(ApiResponse::success(response))
}

/// Method 2: Use convenience macro
///
/// Advantage: More concise syntax
pub async fn generate_content_v2(
    Json(payload): Json<AiGenerateRequest>,
) -> Result<impl IntoResponse, ApiError> {
    // Validate
    payload
        .validate()
        .map_err(|e| ApiError::ValidationError(e.to_string()))?;

    // Call service
    let response = AiService::generate(payload)
        .await
        .map_err(|e| ApiError::AiServiceError(e))?;

    // Use macro to return success response
    Ok(api_ok!(response))
}

/// Method 3: Return ApiResponse directly (without Result)
///
/// Advantage: Full control over response, suitable for complex logic
pub async fn generate_content_v3(Json(payload): Json<AiGenerateRequest>) -> impl IntoResponse {
    // Validate request parameters
    if let Err(e) = payload.validate() {
        return api_err!(
            ErrorCode::ValidationError,
            format!("Validation error: {}", e),
            format!("Validation failed: {}", e)
        );
    }

    // Call service
    match AiService::generate(payload).await {
        Ok(response) => api_ok!(response),
        Err(e) => api_err!(
            ErrorCode::AiServiceError,
            format!("AI service error: {}", e),
            format!("AI service error: {}", e)
        ),
    }
}

/// Method 4: Use custom message
///
/// Advantage: Can customize success message
pub async fn generate_content_v4(
    Json(payload): Json<AiGenerateRequest>,
) -> Result<impl IntoResponse, ApiError> {
    payload
        .validate()
        .map_err(|e| ApiError::ValidationError(e.to_string()))?;

    let response = AiService::generate(payload)
        .await
        .map_err(|e| ApiError::AiServiceError(e))?;

    // Success response with custom message
    Ok(ApiResponse::success_with_message(
        response,
        "Content generated successfully",
        "Content generated successfully",
    ))
}

/// Method 5: No data response (e.g., delete operation)
pub async fn delete_something(// ... parameters
) -> Result<impl IntoResponse, ApiError> {
    // ... delete logic

    // Return success response without data
    Ok(api_ok!())
}

/// Method 6: No data response (with custom message)
pub async fn delete_something_with_message(// ... parameters
) -> Result<impl IntoResponse, ApiError> {
    // ... delete logic

    // Return success response without data but with custom message
    Ok(api_ok!(msg: "Resource deleted successfully", "Resource deleted successfully"))
}

// ============ Usage Examples ============

/// Example: List query
pub async fn list_items() -> Result<impl IntoResponse, ApiError> {
    let items = vec!["item1", "item2", "item3"];
    Ok(api_ok!(items))
}

/// Example: Detail query
pub async fn get_item_detail(id: i32) -> Result<impl IntoResponse, ApiError> {
    // Simulate query
    if id == 0 {
        return Err(ApiError::BusinessError(BusinessError::ItemNotFound));
    }

    let item = serde_json::json!({
        "id": id,
        "name": "Sample Item"
    });

    Ok(api_ok!(item))
}

/// Example: Create resource
pub async fn create_item(
    Json(payload): Json<serde_json::Value>,
) -> Result<impl IntoResponse, ApiError> {
    // Create logic...
    let created_item = serde_json::json!({
        "id": 123,
        "name": payload.get("name")
    });

    Ok(api_ok!(
        created_item,
        "Item created successfully",
        "Item created successfully"
    ))
}

/// Example: Update resource
pub async fn update_item(
    id: i32,
    Json(payload): Json<serde_json::Value>,
) -> Result<impl IntoResponse, ApiError> {
    if id == 0 {
        return Err(ApiError::BusinessError(BusinessError::ItemNotFound));
    }

    // Update logic...
    Ok(api_ok!(msg: "Item updated successfully", "Item updated successfully"))
}

/// Example: Error handling
pub async fn example_error_handling() -> Result<impl IntoResponse, ApiError> {
    // Different types of errors
    let condition = 1;

    match condition {
        1 => Err(ApiError::UserNotFound),
        2 => Err(ApiError::InsufficientBalance),
        3 => Err(ApiError::ValidationError("Invalid input".to_string())),
        _ => Ok(api_ok!()),
    }
}
