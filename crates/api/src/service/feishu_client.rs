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

const FOOTER_SOURCE: &str = "GlanceMind 自动通知";

/// Build a "card C" layout interactive card:
/// - a colored header (`template`) with a title and subtitle,
/// - a two-column field grid (`is_short`) for the core fields,
/// - an optional full-width field (e.g. a long description),
/// - a divider, and a footer `note` (IP / time / source as small gray text).
///
/// `fields`/`full_field`/`footer` values must already be `lark_escape`d; the
/// static labels are safe.
fn build_card(
    template: &str,
    title: &str,
    subtitle: &str,
    fields: &[(&str, String)],
    full_field: Option<(&str, String)>,
    footer: &str,
) -> Value {
    let mut elements: Vec<Value> = Vec::new();

    let field_objs: Vec<Value> = fields
        .iter()
        .map(|(label, value)| {
            json!({
                "is_short": true,
                "text": { "tag": "lark_md", "content": format!("**{label}**\n{value}") }
            })
        })
        .collect();
    elements.push(json!({ "tag": "div", "fields": field_objs }));

    if let Some((label, value)) = full_field {
        elements.push(json!({
            "tag": "div",
            "text": { "tag": "lark_md", "content": format!("**{label}**\n{value}") }
        }));
    }

    elements.push(json!({ "tag": "hr" }));
    elements.push(json!({
        "tag": "note",
        "elements": [ { "tag": "lark_md", "content": footer } ]
    }));

    json!({
        "header": {
            "template": template,
            "title": { "tag": "plain_text", "content": title },
            "subtitle": { "tag": "plain_text", "content": subtitle }
        },
        "elements": elements
    })
}

/// 🌱 New-user registration (green) — user / email / phone, with IP + time in the footer.
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
        "GlanceMind 用户增长",
        &[
            ("👤 用户", lark_escape(username)),
            ("📧 邮箱", lark_escape(email)),
            ("📱 手机", lark_escape(phone)),
        ],
        None,
        &format!(
            "🌐 {} · 🕒 {} 北京时间 · {}",
            lark_escape(ip),
            beijing.format("%Y-%m-%d %H:%M:%S"),
            FOOTER_SOURCE
        ),
    )
}

/// 🎯 New social-media task / campaign (orange) — user + task.
pub fn format_campaign_created(username: &str, campaign_name: &str) -> Value {
    build_card(
        "orange",
        "🎯 新建社媒任务",
        "GlanceMind 营销活动",
        &[
            ("👤 用户", lark_escape(username)),
            ("📌 任务", lark_escape(campaign_name)),
        ],
        None,
        &format!("🔔 {FOOTER_SOURCE}"),
    )
}

/// 🚀 New publish task / plan (violet) — user + platform, task as a full-width field.
pub fn format_plan_created(username: &str, plan_name: &str, platform: &str) -> Value {
    build_card(
        "violet",
        "🚀 新建发布任务",
        "GlanceMind 内容发布",
        &[
            ("👤 用户", lark_escape(username)),
            ("📱 平台", lark_escape(platform)),
        ],
        Some(("📌 任务", lark_escape(plan_name))),
        &format!("🔔 {FOOTER_SOURCE}"),
    )
}

/// 💬 User feedback / contact form (indigo) — email + phone, problem as a full-width
/// field, with IP + time in the footer.
pub fn format_feedback(
    email: &str,
    phone: &str,
    description: &str,
    ip: &str,
    submitted_at: DateTime<Utc>,
) -> Value {
    let beijing = submitted_at.with_timezone(&beijing_offset());
    build_card(
        "indigo",
        "💬 用户反馈",
        "来自官网联系表单",
        &[
            ("📧 邮箱", lark_escape(email)),
            ("📱 手机", lark_escape(phone)),
        ],
        Some(("📝 问题描述", lark_escape(description))),
        &format!(
            "🌐 {} · 🕒 {} 北京时间 · {}",
            lark_escape(ip),
            beijing.format("%Y-%m-%d %H:%M:%S"),
            FOOTER_SOURCE
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    // ASSERTION-CHANGE-JUSTIFIED: card layout changed from a single lark_md block
    // to the "C" layout (header subtitle + is_short field grid + hr + note footer).
    // Tests updated to assert the new structure and that EVERY field is still present
    // (IP/time moved into the footer but are not dropped). No assertion weakened.

    /// Collect every rendered text fragment in the card (field grid + full-width
    /// fields + footer note) so a test can assert a value is present regardless of
    /// which element it lives in.
    fn all_text(card: &Value) -> String {
        let mut s = String::new();
        if let Some(elems) = card["elements"].as_array() {
            for e in elems {
                if let Some(fields) = e["fields"].as_array() {
                    for f in fields {
                        if let Some(t) = f["text"]["content"].as_str() {
                            s.push_str(t);
                            s.push('\n');
                        }
                    }
                }
                if let Some(t) = e["text"]["content"].as_str() {
                    s.push_str(t);
                    s.push('\n');
                }
                if let Some(notes) = e["elements"].as_array() {
                    for n in notes {
                        if let Some(t) = n["content"].as_str() {
                            s.push_str(t);
                            s.push('\n');
                        }
                    }
                }
            }
        }
        s
    }

    #[test]
    fn lark_escape_neutralizes_markdown_chars() {
        assert_eq!(lark_escape("a*b"), "a\\*b");
        assert_eq!(lark_escape("a_b"), "a\\_b");
        assert_eq!(lark_escape("a[b"), "a\\[b");
        assert_eq!(lark_escape("plain text"), "plain text");
    }

    #[test]
    fn card_uses_c_layout_structure() {
        let card = format_feedback(
            "u@x.com",
            "13800138000",
            "x",
            "1.1.1.1",
            Utc.with_ymd_and_hms(2026, 6, 25, 2, 0, 0).unwrap(),
        );
        assert!(
            card["header"]["subtitle"]["content"].is_string(),
            "has subtitle"
        );
        let elems = card["elements"].as_array().unwrap();
        assert!(
            elems.iter().any(|e| e["fields"].is_array()),
            "has a fields grid"
        );
        assert!(elems.iter().any(|e| e["tag"] == "hr"), "has a divider");
        assert!(
            elems.iter().any(|e| e["tag"] == "note"),
            "has a footer note"
        );
    }

    #[test]
    fn registration_card_green_has_all_fields_and_escapes() {
        // 10:00 UTC == 18:00 Beijing (UTC+8)
        let ts = Utc.with_ymd_and_hms(2026, 6, 25, 10, 0, 0).unwrap();
        let card = format_user_registered("al*ice", "a@x.com", "13800138000", "1.2.3.4", ts);
        assert_eq!(card["header"]["template"], "green");
        assert_eq!(card["header"]["title"]["content"], "🌱 新用户注册");
        let t = all_text(&card);
        // every field must still be present (确保字段都在)
        assert!(t.contains("a@x.com"), "email present");
        assert!(t.contains("13800138000"), "phone present");
        assert!(t.contains("1.2.3.4"), "ip present (footer)");
        assert!(
            t.contains("2026-06-25 18:00:00"),
            "Beijing time present (footer)"
        );
        assert!(t.contains("北京时间"), "tz label present");
        assert!(t.contains("al\\*ice"), "username escaped");
        assert!(!t.contains("al*ice"), "raw markdown must not survive");
    }

    #[test]
    fn campaign_card_orange_has_user_and_task() {
        let card = format_campaign_created("bob", "双十一推广");
        assert_eq!(card["header"]["template"], "orange");
        assert_eq!(card["header"]["title"]["content"], "🎯 新建社媒任务");
        let t = all_text(&card);
        assert!(t.contains("bob"), "user present");
        assert!(t.contains("双十一推广"), "task present");
    }

    #[test]
    fn plan_card_violet_has_platform_and_task() {
        let card = format_plan_created("bob", "国庆发布计划", "TikTok");
        assert_eq!(card["header"]["template"], "violet");
        let t = all_text(&card);
        assert!(t.contains("bob"), "user present");
        assert!(t.contains("TikTok"), "platform present");
        assert!(t.contains("国庆发布计划"), "task present");
    }

    #[test]
    fn feedback_card_indigo_has_all_fields_and_escapes() {
        // 02:00 UTC == 10:00 Beijing (UTC+8)
        let ts = Utc.with_ymd_and_hms(2026, 6, 25, 2, 0, 0).unwrap();
        let card = format_feedback("u@x.com", "13800138000", "页面*打不开*", "1.2.3.4", ts);
        assert_eq!(card["header"]["template"], "indigo");
        assert_eq!(card["header"]["title"]["content"], "💬 用户反馈");
        let t = all_text(&card);
        assert!(t.contains("u@x.com"), "email present");
        assert!(t.contains("13800138000"), "phone present");
        assert!(t.contains("1.2.3.4"), "ip present (footer)");
        assert!(
            t.contains("2026-06-25 10:00:00"),
            "Beijing time present (footer)"
        );
        assert!(t.contains("页面\\*打不开\\*"), "description escaped");
        assert!(!t.contains("页面*打不开*"), "raw markdown must not survive");
    }
}
