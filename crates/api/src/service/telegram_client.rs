//! Minimal one-way Telegram Bot API notifier.
//!
//! We only ever need `sendMessage` to push internal notifications (user
//! registration, campaign / publish-plan creation) into an internal Telegram
//! channel, so there is no reason to pull in a full bot framework. This mirrors
//! the lightweight HTTP-client pattern already used by `xunhupay_client` /
//! `laozhang_client`: hold a shared `reqwest::Client` plus the credentials and
//! expose a couple of small async methods.
//!
//! Notifications are best-effort and fire-and-forget — see [`TelegramClient::notify`].

use chrono::{DateTime, Utc};

const TELEGRAM_API_BASE: &str = "https://api.telegram.org";

#[derive(Clone, Debug)]
pub struct TelegramClient {
    bot_token: String,
    chat_id: String,
    http: reqwest::Client,
}

impl TelegramClient {
    pub fn new(bot_token: String, chat_id: String) -> Self {
        Self {
            bot_token: bot_token.trim().to_string(),
            chat_id: chat_id.trim().to_string(),
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("reqwest client"),
        }
    }

    /// Send an HTML-formatted message to the configured chat/channel.
    ///
    /// Returns `Err` on transport failure or a non-2xx Telegram response; the
    /// caller decides what to do with it (notification paths just log it).
    pub async fn send_html(&self, text: String) -> Result<(), String> {
        let url = format!("{TELEGRAM_API_BASE}/bot{}/sendMessage", self.bot_token);
        let resp = self
            .http
            .post(&url)
            .json(&serde_json::json!({
                "chat_id": self.chat_id,
                "text": text,
                "parse_mode": "HTML",
                "disable_web_page_preview": true,
            }))
            .send()
            .await
            .map_err(|e| format!("telegram send http error: {e}"))?;

        let status = resp.status();
        if !status.is_success() {
            // Telegram returns the failure reason in the JSON body; keep a bounded,
            // char-boundary-safe slice so a huge/odd body can never panic or spam logs.
            let body = resp.text().await.unwrap_or_default();
            let snippet: String = body.chars().take(300).collect();
            return Err(format!("telegram api error {status}: {snippet}"));
        }
        Ok(())
    }

    /// Fire-and-forget: spawn the send on the tokio runtime and log on failure.
    /// Never blocks the caller and never propagates an error into the request path.
    pub fn notify(&self, text: String) {
        let client = self.clone();
        tokio::spawn(async move {
            if let Err(e) = client.send_html(text).await {
                tracing::warn!("Telegram notify failed: {e}");
            }
        });
    }
}

/// Escape the three characters that Telegram's HTML `parse_mode` treats as
/// markup (`&`, `<`, `>`). `&` must be replaced first so an injected `&lt;`
/// is not double-escaped. Applied to every user-controlled field we render.
pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Display width of a string in monospace cells: CJK / full-width characters
/// count as 2, everything else as 1. Lets us pad label columns so values line
/// up inside the `<pre>` card regardless of mixed CJK/ASCII labels.
fn display_width(s: &str) -> usize {
    s.chars().map(|c| if is_wide(c) { 2 } else { 1 }).sum()
}

fn is_wide(c: char) -> bool {
    matches!(
        c as u32,
        0x1100..=0x115F
            | 0x2E80..=0x303E
            | 0x3041..=0x33FF
            | 0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xA000..=0xA4CF
            | 0xAC00..=0xD7A3
            | 0xF900..=0xFAFF
            | 0xFE30..=0xFE4F
            | 0xFF00..=0xFF60
            | 0xFFE0..=0xFFE6
            | 0x1F300..=0x1FAFF
            | 0x20000..=0x3FFFD
    )
}

/// Build a "card": a bold title line whose leading emoji is the per-business
/// colored logo, followed by a monospace `<pre>` block whose label column is
/// right-padded to equal display width so values align. `rows` values must
/// already be HTML-escaped; the static labels are safe.
fn build_card(title: &str, rows: &[(&str, String)]) -> String {
    let max_w = rows
        .iter()
        .map(|(l, _)| display_width(l))
        .max()
        .unwrap_or(0);
    let body = rows
        .iter()
        .map(|(label, value)| {
            let pad = " ".repeat(max_w - display_width(label));
            format!("{label}{pad}{value}")
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("{title}\n<pre>{body}</pre>")
}

/// Beijing time (UTC+8) — registration timestamps are rendered in this zone.
fn beijing_offset() -> chrono::FixedOffset {
    chrono::FixedOffset::east_opt(8 * 3600).expect("valid UTC+8 offset")
}

/// 🌱 New-user registration (green logo) — detailed (user / email / phone / IP / time).
pub fn format_user_registered(
    username: &str,
    email: &str,
    phone: &str,
    ip: &str,
    registered_at: DateTime<Utc>,
) -> String {
    let beijing = registered_at.with_timezone(&beijing_offset());
    build_card(
        "🌱 <b>新用户注册</b>",
        &[
            ("用户：", html_escape(username)),
            ("邮箱：", html_escape(email)),
            ("手机：", html_escape(phone)),
            ("IP：", html_escape(ip)),
            (
                "时间：",
                format!("{} 北京时间", beijing.format("%Y-%m-%d %H:%M:%S")),
            ),
        ],
    )
}

/// 🎯 New social-media task (campaign, red logo) — brief (user + task name).
pub fn format_campaign_created(username: &str, campaign_name: &str) -> String {
    build_card(
        "🎯 <b>新建社媒任务</b>",
        &[
            ("用户：", html_escape(username)),
            ("任务：", html_escape(campaign_name)),
        ],
    )
}

/// ⚡ New publish task (plan, yellow logo) — brief + social-media platform.
pub fn format_plan_created(username: &str, plan_name: &str, platform: &str) -> String {
    build_card(
        "⚡ <b>新建发布任务</b>",
        &[
            ("用户：", html_escape(username)),
            ("平台：", html_escape(platform)),
            ("任务：", html_escape(plan_name)),
        ],
    )
}

/// 💬 User feedback / contact form (blue logo) — email / phone / problem / IP / time.
pub fn format_feedback(
    email: &str,
    phone: &str,
    description: &str,
    ip: &str,
    submitted_at: DateTime<Utc>,
) -> String {
    let beijing = submitted_at.with_timezone(&beijing_offset());
    build_card(
        "💬 <b>用户反馈</b>",
        &[
            ("邮箱：", html_escape(email)),
            ("手机：", html_escape(phone)),
            ("问题：", html_escape(description)),
            ("IP：", html_escape(ip)),
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

    #[test]
    fn html_escape_replaces_all_three_special_chars() {
        assert_eq!(html_escape("a & b"), "a &amp; b");
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("plain text"), "plain text");
    }

    #[test]
    fn html_escape_processes_ampersand_first_no_double_escape() {
        // If `<` were escaped before `&`, the `&` it introduces would be
        // re-escaped into `&amp;lt;`. Order must keep a literal `&lt;` as
        // exactly `&amp;lt;` (ampersand of the INPUT escaped once).
        assert_eq!(html_escape("&lt;"), "&amp;lt;");
        assert_eq!(html_escape("&"), "&amp;");
    }

    #[test]
    fn user_registered_contains_all_fields_and_escapes_user_input() {
        let ts = Utc.with_ymd_and_hms(2026, 6, 25, 10, 0, 0).unwrap();
        let msg = format_user_registered("al<ice", "a@x.com", "13800138000", "1.2.3.4", ts);

        assert!(msg.contains("新用户注册"), "title present");
        assert!(msg.contains("a@x.com"), "email present");
        assert!(msg.contains("13800138000"), "phone present");
        assert!(msg.contains("1.2.3.4"), "ip present");
        // ASSERTION-CHANGE-JUSTIFIED: registration time spec changed UTC → Beijing (UTC+8);
        // 10:00:00 UTC now renders as 18:00:00 北京时间.
        assert!(
            msg.contains("2026-06-25 18:00:00"),
            "registration time shown in Beijing time (UTC+8)"
        );
        assert!(msg.contains("北京时间"), "timezone label present");
        // user-controlled username must be HTML-escaped, never raw
        assert!(msg.contains("al&lt;ice"), "username must be escaped");
        assert!(
            !msg.contains("al<ice"),
            "raw injected markup must not survive"
        );
    }

    #[test]
    fn campaign_created_is_brief_with_user_and_task() {
        let msg = format_campaign_created("bob", "双十一推广");
        assert!(msg.contains("新建社媒任务"));
        assert!(msg.contains("bob"));
        assert!(msg.contains("双十一推广"));
    }

    #[test]
    fn plan_created_includes_the_platform() {
        let msg = format_plan_created("bob", "国庆发布计划", "TikTok");
        assert!(msg.contains("新建发布任务"));
        assert!(msg.contains("bob"));
        assert!(msg.contains("国庆发布计划"));
        assert!(
            msg.contains("TikTok"),
            "publish task notification must carry the platform"
        );
    }

    #[test]
    fn plan_created_escapes_platform_and_name() {
        let msg = format_plan_created("u", "<b>name", "<i>plat");
        assert!(msg.contains("&lt;b&gt;name"));
        assert!(msg.contains("&lt;i&gt;plat"));
        assert!(!msg.contains("<b>name"));
        assert!(!msg.contains("<i>plat"));
    }

    #[test]
    fn display_width_counts_cjk_and_fullwidth_as_two() {
        assert_eq!(display_width("ab"), 2);
        assert_eq!(display_width("用户"), 4);
        assert_eq!(display_width("用户："), 6); // full-width colon counts as 2
        assert_eq!(display_width("IP："), 4);
    }

    #[test]
    fn cards_render_body_inside_a_pre_block() {
        let msg = format_campaign_created("bob", "双十一推广");
        assert!(
            msg.contains("<pre>") && msg.contains("</pre>"),
            "card body must be a <pre> monospace block"
        );
        let title = msg.split("\n<pre>").next().unwrap();
        assert!(
            title.contains("<b>新建社媒任务</b>"),
            "title sits above the card"
        );
    }

    #[test]
    fn registration_card_value_columns_align() {
        let ts = Utc.with_ymd_and_hms(2026, 6, 25, 9, 55, 49).unwrap();
        let msg = format_user_registered(
            "alice_chen",
            "alice@example.com",
            "13900139000",
            "203.0.113.7",
            ts,
        );
        let body = msg
            .split("<pre>")
            .nth(1)
            .unwrap()
            .split("</pre>")
            .next()
            .unwrap();
        // Each row's value begins after the full-width colon `：` plus any padding
        // spaces; that start column (in display cells) must be equal across every
        // row, otherwise the columns don't line up.
        let cols: Vec<usize> = body
            .lines()
            .map(|line| {
                let colon_end = line
                    .char_indices()
                    .find(|(_, c)| *c == '：')
                    .map(|(i, c)| i + c.len_utf8())
                    .expect("each row has a full-width colon");
                let spaces = line[colon_end..].chars().take_while(|c| *c == ' ').count();
                display_width(&line[..colon_end]) + spaces
            })
            .collect();
        // ASSERTION-CHANGE-JUSTIFIED: added 手机 (phone) row — registration card now has 5 rows
        assert_eq!(
            cols.len(),
            5,
            "registration card has 5 rows (user/email/phone/ip/time)"
        );
        assert!(
            cols.windows(2).all(|w| w[0] == w[1]),
            "all value columns must align, got {cols:?}"
        );
    }

    #[test]
    fn feedback_card_contains_all_fields_and_escapes_input() {
        // 02:00 UTC == 10:00 Beijing (UTC+8)
        let ts = Utc.with_ymd_and_hms(2026, 6, 25, 2, 0, 0).unwrap();
        let msg = format_feedback("u@x.com", "13800138000", "页面<打不开>", "1.2.3.4", ts);
        assert!(msg.contains("用户反馈"), "title present");
        assert!(msg.contains("u@x.com"), "email present");
        assert!(msg.contains("13800138000"), "phone present");
        assert!(msg.contains("1.2.3.4"), "ip present");
        assert!(
            msg.contains("2026-06-25 10:00:00"),
            "submit time shown in Beijing"
        );
        assert!(msg.contains("北京时间"), "timezone label present");
        // user-controlled description must be HTML-escaped, never raw
        assert!(
            msg.contains("页面&lt;打不开&gt;"),
            "description must be escaped"
        );
        assert!(!msg.contains("页面<打不开>"), "raw markup must not survive");
        assert!(
            msg.contains("<pre>") && msg.contains("</pre>"),
            "card body in <pre>"
        );
    }
}
