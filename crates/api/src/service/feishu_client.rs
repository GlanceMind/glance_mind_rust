//! Minimal Feishu (Lark) custom-bot webhook notifier.
//!
//! Posts interactive cards to a custom-bot webhook URL (no signature configured).
//! Replaces the previous Telegram notifier; same best-effort, fire-and-forget
//! semantics — a webhook failure never blocks or fails the originating request.

use chrono::{DateTime, Utc};
use serde_json::{json, Value};

#[derive(Clone, Debug)]
pub struct FeishuClient {
    webhook_url: String,
    http: reqwest::Client,
}

impl FeishuClient {
    pub fn new(webhook_url: String) -> Self {
        Self {
            webhook_url: webhook_url.trim().to_string(),
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("reqwest client"),
        }
    }

    /// Send an interactive card to the webhook. Returns `Err` on transport
    /// failure, a non-2xx HTTP status, or a non-zero Feishu response code.
    pub async fn send_card(&self, card: Value) -> Result<(), String> {
        let body = json!({ "msg_type": "interactive", "card": card });
        let resp = self
            .http
            .post(&self.webhook_url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("feishu send http error: {e}"))?;

        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        let snippet: String = text.chars().take(300).collect();
        if !status.is_success() {
            return Err(format!("feishu http error {status}: {snippet}"));
        }
        // Feishu returns {"code":0,...} (or legacy {"StatusCode":0,...}) on success.
        let code = serde_json::from_str::<Value>(&text).ok().and_then(|v| {
            v.get("code")
                .and_then(Value::as_i64)
                .or_else(|| v.get("StatusCode").and_then(Value::as_i64))
        });
        if matches!(code, Some(c) if c != 0) {
            return Err(format!("feishu api error: {snippet}"));
        }
        Ok(())
    }

    /// Fire-and-forget: spawn the send on the tokio runtime and log on failure.
    /// Never blocks the caller and never propagates an error into the request path.
    pub fn notify(&self, card: Value) {
        let client = self.clone();
        tokio::spawn(async move {
            if let Err(e) = client.send_card(card).await {
                tracing::warn!("Feishu notify failed: {e}");
            }
        });
    }
}

/// Escape the characters Feishu's `lark_md` treats as markup, so user-controlled
/// values can't break formatting or inject links. Backslash is escaped first.
pub fn lark_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if matches!(c, '\\' | '*' | '_' | '~' | '`' | '[' | ']') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Beijing time (UTC+8) — timestamps are rendered in this zone.
fn beijing_offset() -> chrono::FixedOffset {
    chrono::FixedOffset::east_opt(8 * 3600).expect("valid UTC+8 offset")
}

/// Build an interactive card: a colored header (`template`) plus a single
/// markdown body of `**label**value` rows. `rows` values must already be
/// `lark_escape`d; the static labels are safe.
fn build_card(template: &str, title: &str, rows: &[(&str, String)]) -> Value {
    let content = rows
        .iter()
        .map(|(label, value)| format!("**{label}**{value}"))
        .collect::<Vec<_>>()
        .join("\n");
    json!({
        "header": {
            "template": template,
            "title": { "tag": "plain_text", "content": title }
        },
        "elements": [
            { "tag": "div", "text": { "tag": "lark_md", "content": content } }
        ]
    })
}

/// 🌱 New-user registration (green card) — user / email / phone / IP / time.
pub fn format_user_registered(
    username: &str,
    email: &str,
    phone: &str,
    ip: &str,
    registered_at: DateTime<Utc>,
) -> Value {
    let beijing = registered_at.with_timezone(&beijing_offset());
    build_card(
        "green",
        "🌱 新用户注册",
        &[
            ("用户：", lark_escape(username)),
            ("邮箱：", lark_escape(email)),
            ("手机：", lark_escape(phone)),
            ("IP：", lark_escape(ip)),
            (
                "时间：",
                format!("{} 北京时间", beijing.format("%Y-%m-%d %H:%M:%S")),
            ),
        ],
    )
}

/// 🎯 New social-media task / campaign (red card) — user + task name.
pub fn format_campaign_created(username: &str, campaign_name: &str) -> Value {
    build_card(
        "red",
        "🎯 新建社媒任务",
        &[
            ("用户：", lark_escape(username)),
            ("任务：", lark_escape(campaign_name)),
        ],
    )
}

/// ⚡ New publish task / plan (yellow card) — user + platform + task.
pub fn format_plan_created(username: &str, plan_name: &str, platform: &str) -> Value {
    build_card(
        "yellow",
        "⚡ 新建发布任务",
        &[
            ("用户：", lark_escape(username)),
            ("平台：", lark_escape(platform)),
            ("任务：", lark_escape(plan_name)),
        ],
    )
}

/// 💬 User feedback / contact form (blue card) — email / phone / problem / IP / time.
pub fn format_feedback(
    email: &str,
    phone: &str,
    description: &str,
    ip: &str,
    submitted_at: DateTime<Utc>,
) -> Value {
    let beijing = submitted_at.with_timezone(&beijing_offset());
    build_card(
        "blue",
        "💬 用户反馈",
        &[
            ("邮箱：", lark_escape(email)),
            ("手机：", lark_escape(phone)),
            ("问题：", lark_escape(description)),
            ("IP：", lark_escape(ip)),
            (
                "时间：",
                format!("{} 北京时间", beijing.format("%Y-%m-%d %H:%M:%S")),
            ),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn body(card: &Value) -> String {
        card["elements"][0]["text"]["content"]
            .as_str()
            .expect("card body content")
            .to_string()
    }

    #[test]
    fn lark_escape_neutralizes_markdown_chars() {
        assert_eq!(lark_escape("a*b"), "a\\*b");
        assert_eq!(lark_escape("a_b"), "a\\_b");
        assert_eq!(lark_escape("a[b"), "a\\[b");
        assert_eq!(lark_escape("plain text"), "plain text");
    }

    #[test]
    fn registration_card_is_green_with_all_fields_escaped() {
        // 10:00 UTC == 18:00 Beijing (UTC+8)
        let ts = Utc.with_ymd_and_hms(2026, 6, 25, 10, 0, 0).unwrap();
        let card = format_user_registered("al*ice", "a@x.com", "13800138000", "1.2.3.4", ts);
        assert_eq!(card["header"]["template"], "green");
        assert_eq!(card["header"]["title"]["content"], "🌱 新用户注册");
        let b = body(&card);
        assert!(b.contains("a@x.com"), "email present");
        assert!(b.contains("13800138000"), "phone present");
        assert!(b.contains("1.2.3.4"), "ip present");
        assert!(b.contains("2026-06-25 18:00:00"), "time shown in Beijing");
        assert!(b.contains("北京时间"), "tz label present");
        // markdown-significant char in username must be escaped, never raw
        assert!(b.contains("al\\*ice"), "username must be escaped");
        assert!(!b.contains("al*ice"), "raw markdown must not survive");
    }

    #[test]
    fn campaign_card_is_red_brief() {
        let card = format_campaign_created("bob", "双十一推广");
        assert_eq!(card["header"]["template"], "red");
        assert_eq!(card["header"]["title"]["content"], "🎯 新建社媒任务");
        let b = body(&card);
        assert!(b.contains("bob") && b.contains("双十一推广"));
    }

    #[test]
    fn plan_card_is_yellow_with_platform() {
        let card = format_plan_created("bob", "国庆发布计划", "TikTok");
        assert_eq!(card["header"]["template"], "yellow");
        let b = body(&card);
        assert!(b.contains("国庆发布计划"));
        assert!(b.contains("TikTok"), "publish card must carry the platform");
    }

    #[test]
    fn feedback_card_is_blue_with_all_fields_escaped() {
        // 02:00 UTC == 10:00 Beijing (UTC+8)
        let ts = Utc.with_ymd_and_hms(2026, 6, 25, 2, 0, 0).unwrap();
        let card = format_feedback("u@x.com", "13800138000", "页面*打不开*", "1.2.3.4", ts);
        assert_eq!(card["header"]["template"], "blue");
        assert_eq!(card["header"]["title"]["content"], "💬 用户反馈");
        let b = body(&card);
        assert!(b.contains("u@x.com"), "email present");
        assert!(b.contains("13800138000"), "phone present");
        assert!(b.contains("1.2.3.4"), "ip present");
        assert!(b.contains("2026-06-25 10:00:00"), "time shown in Beijing");
        assert!(b.contains("北京时间"), "tz label present");
        assert!(
            b.contains("页面\\*打不开\\*"),
            "description must be escaped"
        );
        assert!(!b.contains("页面*打不开*"), "raw markdown must not survive");
    }
}
