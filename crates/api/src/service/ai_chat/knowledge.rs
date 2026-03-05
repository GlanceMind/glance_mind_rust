use scraper::{Html, Selector};
use serde::Serialize;
use tokio::sync::OnceCell;

const DOCS_BASE_URL: &str = "https://docs.glancemind.org";

struct DocSource {
    topic: &'static str,
    title: &'static str,
    path: &'static str,
    keywords: &'static [&'static str],
}

static DOC_SOURCES: &[DocSource] = &[
    DocSource { topic: "platform_overview", title: "产品概览", path: "/guide/introduction", keywords: &["platform", "tiktok", "instagram", "reddit", "twitter", "facebook", "平台", "支持", "概览", "功能"] },
    DocSource { topic: "account_management", title: "社交账户管理", path: "/guide/account-management", keywords: &["account", "group", "device", "profile", "账号", "分组", "设备", "批量"] },
    DocSource { topic: "campaign_creation", title: "创建获客任务", path: "/guide/create-task", keywords: &["campaign", "create", "keyword", "region", "budget", "search", "营销", "活动", "搜索", "计费", "预算", "互动", "获客", "任务"] },
    DocSource { topic: "template_guide", title: "配置回复模板", path: "/guide/create-template", keywords: &["template", "persona", "reply", "dm_prompt", "模板", "回复", "人设", "话术"] },
    DocSource { topic: "publish_plan", title: "创建发布计划", path: "/guide/ai-publish", keywords: &["publish", "plan", "batch_text", "single_video", "content_type", "发布", "计划", "视频", "内容类型", "分发"] },
    DocSource { topic: "dm_group_control", title: "DM 私信群控", path: "/guide/dm-group-control", keywords: &["dm", "message", "inbox", "conversation", "group_control", "私信", "群控", "收件箱", "聊天"] },
    DocSource { topic: "ai_insights", title: "AI 市场洞察", path: "/guide/ai-insights", keywords: &["insight", "analysis", "market", "洞察", "分析", "市场", "情报"] },
    DocSource { topic: "executor_manual", title: "自动化执行器", path: "/guide/executor-manual", keywords: &["executor", "automation", "browser", "执行器", "自动化", "浏览器", "安装"] },
    DocSource { topic: "tiktok_guide", title: "TikTok 使用要求", path: "/guide/tiktok/tiktok-requirements", keywords: &["tiktok", "proxy", "us_account", "要求", "代理", "美国"] },
];

struct DocChunk {
    topic: String,
    title: String,
    content: String,
    keywords: Vec<String>,
    url: String,
}

#[derive(Serialize)]
pub struct SearchResult {
    pub topic: String,
    pub title: String,
    pub content: String,
    pub url: String,
}

static DOC_CACHE: OnceCell<Vec<DocChunk>> = OnceCell::const_new();

async fn init_cache() -> Vec<DocChunk> {
    tracing::info!("Initializing knowledge base cache from {}", DOCS_BASE_URL);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .unwrap_or_default();

    let fetches = DOC_SOURCES.iter().map(|src| {
        let client = client.clone();
        let url = format!("{}{}.html", DOCS_BASE_URL, src.path);
        let topic = src.topic;
        let title = src.title;
        let keywords: Vec<String> = src.keywords.iter().map(|k| k.to_string()).collect();
        async move {
            match client.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    match resp.text().await {
                        Ok(html) => {
                            let content = extract_text_from_html(&html);
                            tracing::info!("Loaded doc: {} ({} chars)", title, content.len());
                            Some(DocChunk {
                                topic: topic.to_string(),
                                title: title.to_string(),
                                content,
                                keywords,
                                url: url.clone(),
                            })
                        }
                        Err(e) => {
                            tracing::warn!("Failed to read body from {}: {}", url, e);
                            None
                        }
                    }
                }
                Ok(resp) => {
                    tracing::warn!("Non-200 from {}: {}", url, resp.status());
                    None
                }
                Err(e) => {
                    tracing::warn!("Failed to fetch {}: {}", url, e);
                    None
                }
            }
        }
    });

    let results = futures::future::join_all(fetches).await;
    let chunks: Vec<DocChunk> = results.into_iter().flatten().collect();
    tracing::info!("Knowledge base loaded: {} docs cached", chunks.len());
    chunks
}

fn extract_text_from_html(html: &str) -> String {
    let document = Html::parse_document(html);

    let selectors = [".vp-doc", "main", ".content", "article", "body"];
    for sel_str in &selectors {
        if let Ok(selector) = Selector::parse(sel_str) {
            if let Some(element) = document.select(&selector).next() {
                let text: String = element
                    .text()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ");
                if !text.is_empty() {
                    return text;
                }
            }
        }
    }
    String::new()
}

pub async fn search(query: &str, top_k: usize) -> Vec<SearchResult> {
    let chunks = DOC_CACHE.get_or_init(|| init_cache()).await;
    if chunks.is_empty() {
        return vec![];
    }

    let query_lower = query.to_lowercase();
    let query_tokens: Vec<&str> = query_lower
        .split(|c: char| c.is_whitespace() || c == ',' || c == '，' || c == '、')
        .filter(|s| !s.is_empty())
        .collect();

    let mut scored: Vec<(f32, &DocChunk)> = chunks
        .iter()
        .map(|chunk| {
            let score = calculate_relevance(&query_tokens, &query_lower, chunk);
            (score, chunk)
        })
        .filter(|(score, _)| *score > 0.0)
        .collect();

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(top_k);

    scored
        .into_iter()
        .map(|(_, chunk)| {
            let content = if chunk.content.len() > 1500 {
                find_relevant_snippet(&chunk.content, &query_tokens, 1500)
            } else {
                chunk.content.clone()
            };
            SearchResult {
                topic: chunk.topic.clone(),
                title: chunk.title.clone(),
                content,
                url: chunk.url.clone(),
            }
        })
        .collect()
}

fn calculate_relevance(tokens: &[&str], query_lower: &str, chunk: &DocChunk) -> f32 {
    let mut score: f32 = 0.0;
    let title_lower = chunk.title.to_lowercase();
    let content_lower = chunk.content.to_lowercase();

    for token in tokens {
        if chunk
            .keywords
            .iter()
            .any(|k| k.contains(token) || token.contains(k.as_str()))
        {
            score += 3.0;
        }
        if title_lower.contains(token) {
            score += 2.0;
        }
        if content_lower.contains(token) {
            score += 1.0;
        }
    }

    if content_lower.contains(query_lower) {
        score += 2.0;
    }

    score
}

fn find_relevant_snippet(content: &str, tokens: &[&str], max_len: usize) -> String {
    let content_lower = content.to_lowercase();
    let mut best_pos = 0;
    let mut best_score = 0;

    let window = max_len.min(content.len());
    let step = 200;
    let mut pos = 0;
    while pos + window <= content.len() {
        let slice = &content_lower[pos..pos + window];
        let score: usize = tokens.iter().map(|t| slice.matches(t).count()).sum();
        if score > best_score {
            best_score = score;
            best_pos = pos;
        }
        pos += step;
    }

    let end = (best_pos + max_len).min(content.len());
    let snippet = &content[best_pos..end];
    if best_pos > 0 || end < content.len() {
        format!("...{}...", snippet)
    } else {
        snippet.to_string()
    }
}
