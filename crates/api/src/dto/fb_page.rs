//! Facebook Pages value object for social accounts.
//!
//! A Facebook account can manage one or more Pages, each identified by a numeric
//! Page ID (e.g. "100082341853837") with an optional display name. The set is
//! stored as a JSON array in the `gm_social_accounts.fb_pages_id` TEXT column.
//!
//! Normalization is **lenient** (no 400): invalid ids (empty / non-digit) are
//! silently dropped, pages are deduped by id (last name wins), and the list is
//! capped at [`MAX_PAGES`]. The client provides inline validation feedback.

use serde::{Deserialize, Serialize};

/// Loose upper bound on pages per account — a DoS guard against an unbounded
/// TEXT blob, NOT a business cap (a real FB user manages a handful of pages).
pub const MAX_PAGES: usize = 200;

/// A single Facebook Page: numeric id + optional display name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FbPage {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Normalize a page list before persisting:
/// - drop pages whose id is empty or contains a non-ASCII-digit byte,
/// - dedup by id (last occurrence's name wins),
/// - cap at [`MAX_PAGES`].
pub fn normalize_pages(pages: Vec<FbPage>) -> Vec<FbPage> {
    let mut out: Vec<FbPage> = Vec::with_capacity(pages.len().min(MAX_PAGES));
    for page in pages {
        if page.id.is_empty() || !page.id.bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        if let Some(existing) = out.iter_mut().find(|p| p.id == page.id) {
            existing.name = page.name; // last name wins
        } else {
            out.push(page);
        }
    }
    out.truncate(MAX_PAGES);
    out
}

/// Serialize a normalized page list to the DB column value. Returns `None`
/// (⇒ SQL NULL) when the list is empty, so "no pages" is NULL rather than the
/// literal string `"[]"`.
pub fn serialize_pages(pages: &[FbPage]) -> Option<String> {
    if pages.is_empty() {
        None
    } else {
        serde_json::to_string(pages).ok()
    }
}

/// Parse the DB column value back into a page list. `None` / empty / malformed
/// ⇒ `[]` (lenient read; the write path always emits valid JSON, so malformed
/// only arises from legacy or manually-edited data).
pub fn parse_pages(raw: Option<&str>) -> Vec<FbPage> {
    match raw {
        None => Vec::new(),
        Some(s) if s.trim().is_empty() => Vec::new(),
        Some(s) => serde_json::from_str(s).unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(id: &str, name: Option<&str>) -> FbPage {
        FbPage {
            id: id.to_string(),
            name: name.map(|s| s.to_string()),
        }
    }

    #[test]
    fn fb_page_rejects_non_digit() {
        assert!(normalize_pages(vec![p("abc", None)]).is_empty());
        assert!(normalize_pages(vec![p("12a", None)]).is_empty());
        assert!(normalize_pages(vec![p(" 1", None)]).is_empty());
        assert!(normalize_pages(vec![p("1 2", None)]).is_empty());
        assert!(normalize_pages(vec![p("12 ", None)]).is_empty());
        assert!(normalize_pages(vec![p("", None)]).is_empty());
    }

    #[test]
    fn fb_page_accepts_numeric() {
        let out = normalize_pages(vec![p("100082341853837", Some("Shop"))]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id, "100082341853837");
        assert_eq!(out[0].name.as_deref(), Some("Shop"));
    }

    #[test]
    fn fb_pages_dedup_by_id() {
        let out = normalize_pages(vec![p("1", None), p("1", Some("x"))]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name.as_deref(), Some("x")); // last name wins
    }

    #[test]
    fn fb_pages_ceiling_exact_boundary() {
        let mk = |n: usize| (0..n).map(|i| p(&i.to_string(), None)).collect::<Vec<_>>();
        assert_eq!(normalize_pages(mk(199)).len(), 199);
        assert_eq!(normalize_pages(mk(200)).len(), 200);
        assert_eq!(normalize_pages(mk(201)).len(), 200);
    }

    #[test]
    fn fb_page_name_preserves_unicode() {
        let out = normalize_pages(vec![p("1", Some("店铺 🏪 测试"))]);
        assert_eq!(out[0].name.as_deref(), Some("店铺 🏪 测试"));
    }

    #[test]
    fn fb_page_serializes_none_name_omitted() {
        let json = serde_json::to_string(&p("100082341853837", None)).unwrap();
        assert_eq!(json, r#"{"id":"100082341853837"}"#);
        assert!(!json.contains("name"));
    }

    #[test]
    fn fb_page_golden_roundtrip() {
        let golden = r#"[{"id":"100082341853837","name":"Shop"},{"id":"61556000000000"}]"#;
        let parsed = parse_pages(Some(golden));
        assert_eq!(
            parsed,
            vec![
                p("100082341853837", Some("Shop")),
                p("61556000000000", None)
            ]
        );
        // round-trips back to the same JSON (name omitted when None)
        let reserialized = serialize_pages(&parsed).unwrap();
        assert_eq!(reserialized, golden);
    }

    #[test]
    fn parse_rejects_malformed_json() {
        assert_eq!(parse_pages(Some("{not json")), Vec::new());
        assert_eq!(parse_pages(Some("not json at all")), Vec::new());
        assert_eq!(parse_pages(None), Vec::new());
        assert_eq!(parse_pages(Some("   ")), Vec::new());
    }

    #[test]
    fn serialize_empty_is_none() {
        assert_eq!(serialize_pages(&[]), None);
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    // Mix valid numeric ids, junk ids, forced duplicates, and empty ids, with
    // optional names — so the round-trip/dedup/cap properties actually exercise
    // the drop + dedup + truncate paths (not just clean input).
    fn any_pages() -> impl Strategy<Value = Vec<FbPage>> {
        let id = prop_oneof![
            "[0-9]{1,18}",
            "[a-zA-Z0-9 ]{0,6}",
            Just("1".to_string()),
            Just(String::new()),
        ];
        let name = prop_oneof![Just(None::<String>), ".*".prop_map(Some)];
        prop::collection::vec(
            (id, name).prop_map(|(id, name)| FbPage { id, name }),
            0..260,
        )
    }

    proptest! {
        #[test]
        fn prop_roundtrip(pages in any_pages()) {
            let norm = normalize_pages(pages);
            let serialized = serialize_pages(&norm);
            let back = parse_pages(serialized.as_deref());
            prop_assert_eq!(back, norm);
        }

        #[test]
        fn prop_dedup_idempotent(pages in any_pages()) {
            let once = normalize_pages(pages);
            let twice = normalize_pages(once.clone());
            prop_assert_eq!(twice, once);
        }

        #[test]
        fn prop_all_persisted_ids_numeric(pages in any_pages()) {
            for page in normalize_pages(pages) {
                prop_assert!(!page.id.is_empty() && page.id.bytes().all(|b| b.is_ascii_digit()));
            }
        }

        #[test]
        fn prop_capped_at_max(pages in any_pages()) {
            prop_assert!(normalize_pages(pages).len() <= MAX_PAGES);
        }
    }
}
