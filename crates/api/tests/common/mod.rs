/// 集成测试公共辅助模块
///
/// 提供统一响应格式的测试辅助函数
use serde_json::Value;

/// Unified response format
#[allow(dead_code)]
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ApiResponse<T> {
    pub code: i32,
    pub msg: String,
    pub msg_cn: String,
    pub data: Option<T>,
}

/// 从 JSON 值中提取 data 字段
///
/// 新的响应格式：
/// ```json
/// {
///   "code": 1000,
///   "msg": "Success",
///   "msg_cn": "操作成功",
///   "data": { ... }
/// }
/// ```
pub fn extract_data(json: &Value) -> Option<&Value> {
    json.get("data")
}

/// 检查响应是否成功
pub fn is_success(json: &Value) -> bool {
    json.get("code")
        .and_then(|v| v.as_i64())
        .map(|code| code == 1000)
        .unwrap_or(false)
}

/// 获取错误消息
pub fn get_error_message(json: &Value) -> String {
    // 优先使用中文消息
    json.get("msg_cn")
        .and_then(|v| v.as_str())
        .or_else(|| json.get("msg").and_then(|v| v.as_str()))
        .unwrap_or("Unknown error")
        .to_string()
}

/// 获取错误码
pub fn get_error_code(json: &Value) -> i32 {
    json.get("code").and_then(|v| v.as_i64()).unwrap_or(3000) as i32
}

/// 断言响应成功
pub fn assert_success(json: &Value) {
    let code = get_error_code(json);
    if code != 1000 {
        let msg = get_error_message(json);
        panic!(
            "Expected success response (code 1000), got code {} with message: {}",
            code, msg
        );
    }
}

/// Assert response is an error and return error message
#[allow(dead_code)]
pub fn assert_error(json: &Value, expected_code: Option<i32>) -> String {
    let code = get_error_code(json);
    if code == 1000 {
        panic!("Expected error response, got success (code 1000)");
    }

    if let Some(expected) = expected_code {
        assert_eq!(
            code, expected,
            "Expected error code {}, got {}",
            expected, code
        );
    }

    get_error_message(json)
}

/// Extract data field from response and assert success
#[allow(dead_code)]
pub fn extract_success_data(json: &Value) -> &Value {
    assert_success(json);
    extract_data(json).expect("Response should contain data field")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_is_success() {
        let json = json!({
            "code": 1000,
            "msg": "Success",
            "msg_cn": "操作成功",
            "data": {"id": 1}
        });
        assert!(is_success(&json));
    }

    #[test]
    fn test_is_not_success() {
        let json = json!({
            "code": 4000,
            "msg": "User not found",
            "msg_cn": "用户不存在"
        });
        assert!(!is_success(&json));
    }

    #[test]
    fn test_extract_data() {
        let json = json!({
            "code": 1000,
            "msg": "Success",
            "msg_cn": "操作成功",
            "data": {"id": 1, "name": "John"}
        });
        let data = extract_data(&json).unwrap();
        assert_eq!(data["id"], 1);
        assert_eq!(data["name"], "John");
    }

    #[test]
    fn test_get_error_message() {
        let json = json!({
            "code": 4000,
            "msg": "User not found",
            "msg_cn": "用户不存在"
        });
        assert_eq!(get_error_message(&json), "用户不存在");
    }
}
