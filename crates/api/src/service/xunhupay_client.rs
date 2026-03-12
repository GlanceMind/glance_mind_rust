use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;
use tracing::warn;

const DEFAULT_GATEWAY: &str = "https://api.xunhupay.com";
const API_VERSION: &str = "1.1";

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct XunhuPayClient {
    app_id: String,
    app_secret: String,
    gateway: String,
    http: reqwest::Client,
}

impl XunhuPayClient {
    pub fn new(app_id: String, app_secret: String, gateway: Option<String>) -> Self {
        let gateway = normalize_gateway(gateway.as_deref().unwrap_or(DEFAULT_GATEWAY));
        Self {
            app_id: app_id.trim().to_string(),
            app_secret: app_secret.trim().to_string(),
            gateway,
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .expect("reqwest client"),
        }
    }

    // ---- Pay (下单) -------------------------------------------------------

    pub async fn pay(&self, req: PayRequest) -> Result<PayResponse, String> {
        let mut params = BTreeMap::new();
        params.insert("version".to_string(), API_VERSION.to_string());
        params.insert("appid".to_string(), self.app_id.clone());
        params.insert("trade_order_id".to_string(), req.trade_order_id);
        params.insert("total_fee".to_string(), req.total_fee);
        params.insert("title".to_string(), req.title);
        params.insert("notify_url".to_string(), req.notify_url);
        params.insert(
            "time".to_string(),
            chrono::Utc::now().timestamp().to_string(),
        );
        params.insert("nonce_str".to_string(), nonce());

        if let Some(v) = req.return_url {
            params.insert("return_url".to_string(), v);
        }
        if let Some(v) = req.callback_url {
            params.insert("callback_url".to_string(), v);
        }
        if let Some(v) = req.attach {
            params.insert("attach".to_string(), v);
        }
        if let Some(v) = req.wap_url {
            params.insert("wap_url".to_string(), v);
        }
        if let Some(v) = req.wap_name {
            params.insert("wap_name".to_string(), v);
        }
        if let Some(v) = req.r#type {
            params.insert("type".to_string(), v);
        }

        let hash = generate_hash(&params, &self.app_secret);
        params.insert("hash".to_string(), hash);

        let url = self.endpoint_url("do.html");
        let resp = self
            .http
            .post(&url)
            .form(&params)
            .send()
            .await
            .map_err(|e| format!("HTTP error: {e}"))?;

        let body = resp.text().await.map_err(|e| format!("Read body: {e}"))?;
        let raw_json: JsonValue = serde_json::from_str(&body).map_err(|e| {
            warn!("XunhuPay pay response parse failed: {}; body={}", e, &body[..body.len().min(500)]);
            format!("Parse pay response: unexpected response format")
        })?;
        verify_response_hash_from_value(&raw_json, &self.app_secret)
            .map_err(|e| format!("Invalid pay response signature: {e}"))?;
        serde_json::from_value::<PayResponse>(raw_json)
            .map_err(|e| format!("Parse pay response payload: {e}"))
    }

    // ---- Query (查询) -----------------------------------------------------

    pub async fn query_order(&self, order_id: OrderId) -> Result<QueryResponse, String> {
        let mut params = BTreeMap::new();
        params.insert("appid".to_string(), self.app_id.clone());
        let order_desc = match order_id {
            OrderId::TradeOrderId(v) => {
                params.insert("out_trade_order".to_string(), v.clone());
                v
            }
            OrderId::OpenOrderId(v) => {
                params.insert("open_order_id".to_string(), v.clone());
                v
            }
        };
        params.insert(
            "time".to_string(),
            chrono::Utc::now().timestamp().to_string(),
        );
        params.insert("nonce_str".to_string(), nonce());

        let hash = generate_hash(&params, &self.app_secret);
        params.insert("hash".to_string(), hash);

        let url = self.endpoint_url("query.html");
        tracing::info!("XunhuPay query: POST {} order={}", url, order_desc);

        let resp = self
            .http
            .post(&url)
            .form(&params)
            .send()
            .await
            .map_err(|e| format!("HTTP error: {e}"))?;

        let body = resp.text().await.map_err(|e| format!("Read body: {e}"))?;
        tracing::debug!("XunhuPay query response: {}", &body[..body.len().min(500)]);

        serde_json::from_str::<QueryResponse>(&body)
            .map_err(|e| format!("Parse query response: {e} | body={}", &body[..body.len().min(300)]))
    }

    // ---- Refund (退款) -----------------------------------------------------

    pub async fn refund_order(&self, req: RefundRequest) -> Result<RefundResponse, String> {
        let mut params = BTreeMap::new();
        params.insert("appid".to_string(), self.app_id.clone());
        match req.order_id {
            OrderId::TradeOrderId(v) => {
                params.insert("trade_order_id".to_string(), v);
            }
            OrderId::OpenOrderId(v) => {
                params.insert("open_order_id".to_string(), v);
            }
        }
        if let Some(v) = req.reason {
            params.insert("reason".to_string(), v);
        }
        params.insert(
            "time".to_string(),
            chrono::Utc::now().timestamp().to_string(),
        );
        params.insert("nonce_str".to_string(), nonce());

        let hash = generate_hash(&params, &self.app_secret);
        params.insert("hash".to_string(), hash);

        let url = self.endpoint_url("refund.html");
        let resp = self
            .http
            .post(&url)
            .form(&params)
            .send()
            .await
            .map_err(|e| format!("HTTP error: {e}"))?;

        let body = resp.text().await.map_err(|e| format!("Read body: {e}"))?;
        let raw_json: JsonValue = serde_json::from_str(&body).map_err(|e| {
            warn!("XunhuPay refund response parse failed: {}; body={}", e, &body[..body.len().min(500)]);
            format!("Parse refund response: unexpected response format")
        })?;
        verify_response_hash_from_value(&raw_json, &self.app_secret)
            .map_err(|e| format!("Invalid refund response signature: {e}"))?;
        serde_json::from_value::<RefundResponse>(raw_json)
            .map_err(|e| format!("Parse refund response payload: {e}"))
    }

    // ---- Notification verification ----------------------------------------

    /// Verify the `hash` in an async callback notification from XunhuPay.
    /// Supports unknown extension fields (official requirement).
    pub fn verify_notification(&self, notification: &PayNotification) -> bool {
        let mut params = BTreeMap::new();
        params.insert(
            "trade_order_id".to_string(),
            notification.trade_order_id.clone(),
        );
        params.insert("total_fee".to_string(), notification.total_fee.clone());
        params.insert(
            "transaction_id".to_string(),
            notification.transaction_id.clone(),
        );
        params.insert(
            "open_order_id".to_string(),
            notification.open_order_id.clone(),
        );
        params.insert("order_title".to_string(), notification.order_title.clone());
        params.insert("status".to_string(), notification.status.clone());
        params.insert("appid".to_string(), notification.appid.clone());
        params.insert("time".to_string(), notification.time.clone());
        params.insert("nonce_str".to_string(), notification.nonce_str.clone());

        if let Some(ref v) = notification.plugins {
            if !v.is_empty() {
                params.insert("plugins".to_string(), v.clone());
            }
        }
        if let Some(ref v) = notification.attach {
            if !v.is_empty() {
                params.insert("attach".to_string(), v.clone());
            }
        }

        // Include any extra fields from `extra` so unknown extensions are covered
        for (k, v) in &notification.extra {
            if k != "hash" && !v.is_empty() {
                params.entry(k.clone()).or_insert_with(|| v.clone());
            }
        }

        let expected = generate_hash(&params, &self.app_secret);
        expected == notification.hash
    }

    fn endpoint_url(&self, endpoint: &str) -> String {
        format!("{}/payment/{}", self.gateway, endpoint)
    }

    pub fn app_id(&self) -> &str {
        &self.app_id
    }
}

// ---------------------------------------------------------------------------
// Hash / signing helpers (public for unit-testing)
// ---------------------------------------------------------------------------

/// Official XunhuPay signing algorithm:
/// 1. Collect all non-empty params except `hash`, sort by ASCII key order.
/// 2. Join as `key1=value1&key2=value2&...` (no trailing `&`).
/// 3. Append APPSECRET directly (no `&` before it).
/// 4. MD5 → 32-char lowercase hex.
pub fn generate_hash(params: &BTreeMap<String, String>, app_secret: &str) -> String {
    let query = params
        .iter()
        .filter(|(k, v)| *k != "hash" && !v.is_empty())
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&");

    let raw = format!("{query}{app_secret}");
    format!("{:x}", md5::compute(raw.as_bytes()))
}

/// Convenience: verify a `hash` against a param map.
pub fn verify_hash(params: &BTreeMap<String, String>, app_secret: &str, hash: &str) -> bool {
    generate_hash(params, app_secret) == hash
}

fn normalize_gateway(raw: &str) -> String {
    let trimmed = raw.trim().trim_end_matches('/');
    for suffix in ["/payment/do.html", "/payment/query.html", "/payment/refund.html"] {
        if let Some(base) = trimmed.strip_suffix(suffix) {
            return base.trim_end_matches('/').to_string();
        }
    }
    trimmed.to_string()
}

fn nonce() -> String {
    format!("{:032x}", uuid::Uuid::new_v4().as_u128())
}

fn json_scalar_to_string(value: &JsonValue) -> Option<String> {
    match value {
        JsonValue::Null => None,
        JsonValue::String(s) if s.is_empty() => None,
        JsonValue::String(s) => Some(s.clone()),
        JsonValue::Number(n) => Some(n.to_string()),
        JsonValue::Bool(b) => Some(b.to_string()),
        other => Some(other.to_string()),
    }
}

fn response_params_from_value(value: &JsonValue) -> Result<(BTreeMap<String, String>, String), String> {
    let obj = value
        .as_object()
        .ok_or_else(|| "response is not a JSON object".to_string())?;
    let hash = obj
        .get("hash")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "missing response hash".to_string())?
        .to_string();

    let mut params = BTreeMap::new();
    for (key, value) in obj {
        if key == "hash" {
            continue;
        }
        if key == "data" {
            if let Some(data_obj) = value.as_object() {
                for (data_key, data_value) in data_obj {
                    if let Some(v) = json_scalar_to_string(data_value) {
                        params.insert(data_key.clone(), v);
                    }
                }
            } else if let Some(v) = json_scalar_to_string(value) {
                params.insert(key.clone(), v);
            }
            continue;
        }
        if let Some(v) = json_scalar_to_string(value) {
            params.insert(key.clone(), v);
        }
    }

    Ok((params, hash))
}

fn verify_response_hash_from_value(value: &JsonValue, app_secret: &str) -> Result<(), String> {
    let (params, hash) = response_params_from_value(value)?;
    if verify_hash(&params, app_secret, &hash) {
        Ok(())
    } else {
        Err("hash mismatch".to_string())
    }
}

fn deserialize_opt_string_or_number<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(match value {
        None | Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::String(s)) => Some(s),
        Some(serde_json::Value::Number(n)) => Some(n.to_string()),
        Some(serde_json::Value::Bool(b)) => Some(b.to_string()),
        Some(other) => Some(other.to_string()),
    })
}

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct PayRequest {
    pub trade_order_id: String,
    pub total_fee: String,
    pub title: String,
    pub notify_url: String,
    pub return_url: Option<String>,
    pub callback_url: Option<String>,
    pub attach: Option<String>,
    pub wap_url: Option<String>,
    pub wap_name: Option<String>,
    /// "WAP" for H5, "JSAPI" for mini-program, None for default
    pub r#type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayResponse {
    #[serde(default)]
    pub errcode: i32,
    #[serde(default)]
    pub errmsg: String,
    /// XunhuPay internal order id (field name in JSON is `openid` due to legacy bug)
    #[serde(alias = "openid", default, deserialize_with = "deserialize_opt_string_or_number")]
    pub order_id: Option<String>,
    /// QR-code URL for PC scanning
    pub url_qrcode: Option<String>,
    /// Redirect URL for mobile
    pub url: Option<String>,
    pub hash: Option<String>,
}

pub enum OrderId {
    TradeOrderId(String),
    OpenOrderId(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResponse {
    #[serde(default)]
    pub errcode: i32,
    #[serde(default)]
    pub errmsg: String,
    pub data: Option<QueryData>,
    pub hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryData {
    pub status: Option<String>,
    pub open_order_id: Option<String>,
    pub trade_order_id: Option<String>,
    pub total_fee: Option<String>,
    pub transaction_id: Option<String>,
}

pub struct RefundRequest {
    pub order_id: OrderId,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    #[serde(default)]
    pub errcode: i32,
    #[serde(default)]
    pub errmsg: String,
    pub trade_order_id: Option<String>,
    pub transaction_id: Option<String>,
    pub out_refund_no: Option<String>,
    pub refund_fee: Option<String>,
    pub reason: Option<String>,
    pub refund_status: Option<String>,
    pub refund_time: Option<String>,
    pub hash: Option<String>,
}

/// Incoming async notification from XunhuPay (form-encoded POST).
/// Uses `flatten` + HashMap to capture unknown extension fields the platform
/// may add in the future (official requirement: must support extra fields).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayNotification {
    pub trade_order_id: String,
    pub total_fee: String,
    pub transaction_id: String,
    pub open_order_id: String,
    pub order_title: String,
    /// OD=paid, CD=refunded, RD=refunding, UD=refund failed
    pub status: String,
    pub appid: String,
    pub time: String,
    pub nonce_str: String,
    pub hash: String,
    #[serde(default)]
    pub plugins: Option<String>,
    #[serde(default)]
    pub attach: Option<String>,
    /// Catch-all for future extension fields
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, String>,
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SECRET: &str = "my_test_secret_key";

    fn make_params(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn test_hash_generation_basic() {
        let params = make_params(&[
            ("appid", "201900001"),
            ("time", "1234567890"),
            ("nonce_str", "abc"),
            ("total_fee", "0.01"),
            ("trade_order_id", "ORDER001"),
        ]);
        let hash = generate_hash(&params, TEST_SECRET);
        assert_eq!(hash.len(), 32, "MD5 must be 32 hex chars");
        assert!(
            hash.chars().all(|c| c.is_ascii_hexdigit()),
            "Must be hex"
        );
    }

    #[test]
    fn test_hash_ignores_hash_field() {
        let p1 = make_params(&[("appid", "X"), ("total_fee", "1")]);
        let mut p2 = p1.clone();
        p2.insert("hash".to_string(), "should_be_ignored".to_string());

        let h1 = generate_hash(&p1, TEST_SECRET);
        let h2 = generate_hash(&p2, TEST_SECRET);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_ignores_empty_values() {
        let p1 = make_params(&[("appid", "X"), ("total_fee", "1")]);
        let mut p2 = p1.clone();
        p2.insert("return_url".to_string(), String::new());

        assert_eq!(
            generate_hash(&p1, TEST_SECRET),
            generate_hash(&p2, TEST_SECRET)
        );
    }

    #[test]
    fn test_hash_ascii_sort_order() {
        // BTreeMap guarantees ASCII sort, verify it produces deterministic output
        let params = make_params(&[
            ("z_field", "last"),
            ("a_field", "first"),
            ("m_field", "middle"),
        ]);
        let hash = generate_hash(&params, TEST_SECRET);
        // Recompute manually
        let raw = format!(
            "a_field=first&m_field=middle&z_field=last{}",
            TEST_SECRET
        );
        let expected = format!("{:x}", md5::compute(raw.as_bytes()));
        assert_eq!(hash, expected);
    }

    #[test]
    fn test_hash_case_sensitive_keys() {
        let p1 = make_params(&[("AppId", "X")]);
        let p2 = make_params(&[("appid", "X")]);
        assert_ne!(
            generate_hash(&p1, TEST_SECRET),
            generate_hash(&p2, TEST_SECRET)
        );
    }

    #[test]
    fn test_verify_hash() {
        let params = make_params(&[("appid", "123"), ("total_fee", "9.90")]);
        let hash = generate_hash(&params, TEST_SECRET);
        assert!(verify_hash(&params, TEST_SECRET, &hash));
        assert!(!verify_hash(&params, TEST_SECRET, "wrong_hash"));
    }

    #[test]
    fn test_hash_with_unknown_extension_fields() {
        let params = make_params(&[
            ("appid", "X"),
            ("total_fee", "1"),
            ("future_field_xyz", "some_value"),
        ]);
        let hash = generate_hash(&params, TEST_SECRET);

        // The extension field must participate in signing
        let without_ext = make_params(&[("appid", "X"), ("total_fee", "1")]);
        assert_ne!(hash, generate_hash(&without_ext, TEST_SECRET));
    }

    #[test]
    fn test_verify_notification_roundtrip() {
        let client = XunhuPayClient::new(
            "TEST_APP".to_string(),
            TEST_SECRET.to_string(),
            None,
        );

        let mut notification = PayNotification {
            trade_order_id: "ORDER001".to_string(),
            total_fee: "9.90".to_string(),
            transaction_id: "TXN001".to_string(),
            open_order_id: "OPEN001".to_string(),
            order_title: "Test".to_string(),
            status: "OD".to_string(),
            appid: "TEST_APP".to_string(),
            time: "1234567890".to_string(),
            nonce_str: "abc123".to_string(),
            hash: String::new(),
            plugins: None,
            attach: Some("user_42".to_string()),
            extra: std::collections::HashMap::new(),
        };

        // Compute expected hash
        let mut params = BTreeMap::new();
        params.insert("trade_order_id".to_string(), "ORDER001".to_string());
        params.insert("total_fee".to_string(), "9.90".to_string());
        params.insert("transaction_id".to_string(), "TXN001".to_string());
        params.insert("open_order_id".to_string(), "OPEN001".to_string());
        params.insert("order_title".to_string(), "Test".to_string());
        params.insert("status".to_string(), "OD".to_string());
        params.insert("appid".to_string(), "TEST_APP".to_string());
        params.insert("time".to_string(), "1234567890".to_string());
        params.insert("nonce_str".to_string(), "abc123".to_string());
        params.insert("attach".to_string(), "user_42".to_string());
        notification.hash = generate_hash(&params, TEST_SECRET);

        assert!(client.verify_notification(&notification));
    }

    #[test]
    fn test_verify_notification_with_extra_fields() {
        let client = XunhuPayClient::new(
            "APP".to_string(),
            TEST_SECRET.to_string(),
            None,
        );

        let mut params = BTreeMap::new();
        params.insert("trade_order_id".to_string(), "O1".to_string());
        params.insert("total_fee".to_string(), "1".to_string());
        params.insert("transaction_id".to_string(), "T1".to_string());
        params.insert("open_order_id".to_string(), "OP1".to_string());
        params.insert("order_title".to_string(), "T".to_string());
        params.insert("status".to_string(), "OD".to_string());
        params.insert("appid".to_string(), "APP".to_string());
        params.insert("time".to_string(), "999".to_string());
        params.insert("nonce_str".to_string(), "n".to_string());
        params.insert("new_future_field".to_string(), "ext_val".to_string());

        let hash = generate_hash(&params, TEST_SECRET);

        let mut extra = std::collections::HashMap::new();
        extra.insert("new_future_field".to_string(), "ext_val".to_string());

        let notification = PayNotification {
            trade_order_id: "O1".to_string(),
            total_fee: "1".to_string(),
            transaction_id: "T1".to_string(),
            open_order_id: "OP1".to_string(),
            order_title: "T".to_string(),
            status: "OD".to_string(),
            appid: "APP".to_string(),
            time: "999".to_string(),
            nonce_str: "n".to_string(),
            hash,
            plugins: None,
            attach: None,
            extra,
        };

        assert!(client.verify_notification(&notification));
    }

    #[test]
    fn test_verify_notification_bad_hash() {
        let client = XunhuPayClient::new(
            "APP".to_string(),
            TEST_SECRET.to_string(),
            None,
        );

        let notification = PayNotification {
            trade_order_id: "O1".to_string(),
            total_fee: "1".to_string(),
            transaction_id: "T1".to_string(),
            open_order_id: "OP1".to_string(),
            order_title: "T".to_string(),
            status: "OD".to_string(),
            appid: "APP".to_string(),
            time: "999".to_string(),
            nonce_str: "n".to_string(),
            hash: "0000000000000000000000000000dead".to_string(),
            plugins: None,
            attach: None,
            extra: std::collections::HashMap::new(),
        };

        assert!(!client.verify_notification(&notification));
    }

    #[test]
    fn test_pay_response_deserialization() {
        let json = r#"{
            "openid": "2019081202",
            "url": "https://api.xunhupay.com/alipay/pay/index.html",
            "url_qrcode": "https://api.xunhupay.com/payments/qrcode/xxx",
            "errcode": 0,
            "errmsg": "success!",
            "hash": "3a91e22ee359c914b0788c6007377638"
        }"#;
        let resp: PayResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.errcode, 0);
        assert_eq!(resp.order_id.as_deref(), Some("2019081202"));
        assert!(resp.url.is_some());
        assert!(resp.url_qrcode.is_some());
    }

    #[test]
    fn test_pay_response_deserialization_with_numeric_openid() {
        let json = r#"{
            "openid": 20294363078,
            "url": "https://api.xunhupay.com/payments/alipay/newQrcode?id=20294363078",
            "url_qrcode": "https://api.xunhupay.com/plugins/newQrcode?data=abc",
            "errcode": 0,
            "errmsg": "success!",
            "hash": "01c81dad665d03a985223597f07568f4"
        }"#;
        let resp: PayResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.errcode, 0);
        assert_eq!(resp.order_id.as_deref(), Some("20294363078"));
        assert!(resp.url.is_some());
        assert!(resp.url_qrcode.is_some());
    }

    #[test]
    fn test_pay_response_error() {
        let json = r#"{"errcode":500,"errmsg":"invalid sign!","hash":"abc"}"#;
        let resp: PayResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.errcode, 500);
        assert!(resp.url.is_none());
    }

    #[test]
    fn test_query_response_deserialization() {
        let json = r#"{
            "errcode": 0,
            "data": { "status": "OD", "open_order_id": "xxx" },
            "errmsg": "success!",
            "hash": "abc"
        }"#;
        let resp: QueryResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.errcode, 0);
        let data = resp.data.unwrap();
        assert_eq!(data.status.as_deref(), Some("OD"));
    }

    #[test]
    fn test_refund_response_deserialization() {
        let json = r#"{
            "errcode": 0,
            "errmsg": "success!",
            "trade_order_id": "O1",
            "refund_status": "CD",
            "refund_fee": "9.90",
            "hash": "abc"
        }"#;
        let resp: RefundResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.errcode, 0);
        assert_eq!(resp.refund_status.as_deref(), Some("CD"));
    }

    #[test]
    fn test_normalize_gateway_accepts_root_url() {
        assert_eq!(
            normalize_gateway("https://api.xunhupay.com"),
            "https://api.xunhupay.com"
        );
        assert_eq!(
            normalize_gateway("https://api.xunhupay.com/"),
            "https://api.xunhupay.com"
        );
    }

    #[test]
    fn test_normalize_gateway_accepts_full_pay_endpoint_url() {
        assert_eq!(
            normalize_gateway("https://api.xunhupay.com/payment/do.html"),
            "https://api.xunhupay.com"
        );
        assert_eq!(
            normalize_gateway("https://api.dpweixin.com/payment/do.html"),
            "https://api.dpweixin.com"
        );
    }

    #[test]
    fn test_endpoint_url_generation_after_normalization() {
        let client = XunhuPayClient::new(
            "APP".to_string(),
            "SECRET".to_string(),
            Some("https://api.xunhupay.com/payment/do.html".to_string()),
        );
        assert_eq!(
            client.endpoint_url("do.html"),
            "https://api.xunhupay.com/payment/do.html"
        );
        assert_eq!(
            client.endpoint_url("query.html"),
            "https://api.xunhupay.com/payment/query.html"
        );
        assert_eq!(
            client.endpoint_url("refund.html"),
            "https://api.xunhupay.com/payment/refund.html"
        );
    }

    #[test]
    fn test_nonce_is_fixed_32_chars() {
        let n = nonce();
        assert_eq!(n.len(), 32);
        assert!(n.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
