use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SafetyLevel {
    ReadOnly,
    Create,
    Modify,
    Destructive,
}

impl SafetyLevel {
    pub fn requires_plan(&self) -> bool {
        matches!(self, Self::Create | Self::Modify | Self::Destructive)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ReadOnly => "read_only",
            Self::Create => "create",
            Self::Modify => "modify",
            Self::Destructive => "destructive",
        }
    }
}

impl fmt::Display for SafetyLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub safety_level: SafetyLevel,
    pub parameters: Value,
}

impl ToolDefinition {
    pub fn to_openai_function(&self) -> Value {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": self.name,
                "description": self.description,
                "parameters": self.parameters,
            }
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRequest {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResult {
    pub id: String,
    pub name: String,
    pub result: Value,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub tool_name: String,
    pub tool_params: Value,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanProposal {
    pub title: String,
    pub description: String,
    pub steps: Vec<PlanStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", content = "data")]
pub enum SseEvent {
    #[serde(rename = "message_start")]
    MessageStart { message_id: i32 },
    #[serde(rename = "text_delta")]
    TextDelta { delta: String },
    #[serde(rename = "tool_call_start")]
    ToolCallStart {
        tool_call_id: String,
        tool_name: String,
    },
    #[serde(rename = "tool_call_result")]
    ToolCallResult {
        tool_call_id: String,
        result: Value,
        success: bool,
    },
    #[serde(rename = "plan_created")]
    PlanCreated {
        plan_id: i32,
        title: String,
        steps: Vec<PlanStepSse>,
    },
    #[serde(rename = "message_end")]
    MessageEnd {
        message_id: i32,
        finish_reason: String,
    },
    #[serde(rename = "step_start")]
    StepStart {
        step_id: i32,
        step_order: i32,
        description: String,
    },
    #[serde(rename = "step_completed")]
    StepCompleted { step_id: i32, result: Value },
    #[serde(rename = "step_failed")]
    StepFailed {
        step_id: i32,
        error: String,
    },
    #[serde(rename = "plan_completed")]
    PlanCompleted {
        plan_id: i32,
        summary: String,
    },
    #[serde(rename = "error")]
    Error { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStepSse {
    pub step_id: i32,
    pub step_order: i32,
    pub tool_name: String,
    pub description: String,
    pub tool_params: Value,
}

impl SseEvent {
    pub fn to_sse_string(&self) -> String {
        let (event_name, data) = match self {
            Self::MessageStart { message_id } => (
                "message_start",
                serde_json::json!({ "message_id": message_id }),
            ),
            Self::TextDelta { delta } => ("text_delta", serde_json::json!({ "delta": delta })),
            Self::ToolCallStart {
                tool_call_id,
                tool_name,
            } => (
                "tool_call_start",
                serde_json::json!({ "tool_call_id": tool_call_id, "tool_name": tool_name }),
            ),
            Self::ToolCallResult {
                tool_call_id,
                result,
                success,
            } => (
                "tool_call_result",
                serde_json::json!({ "tool_call_id": tool_call_id, "result": result, "success": success }),
            ),
            Self::PlanCreated {
                plan_id,
                title,
                steps,
            } => (
                "plan_created",
                serde_json::json!({ "plan_id": plan_id, "title": title, "steps": steps }),
            ),
            Self::MessageEnd {
                message_id,
                finish_reason,
            } => (
                "message_end",
                serde_json::json!({ "message_id": message_id, "finish_reason": finish_reason }),
            ),
            Self::StepStart {
                step_id,
                step_order,
                description,
            } => (
                "step_start",
                serde_json::json!({ "step_id": step_id, "step_order": step_order, "description": description }),
            ),
            Self::StepCompleted { step_id, result } => (
                "step_completed",
                serde_json::json!({ "step_id": step_id, "result": result }),
            ),
            Self::StepFailed { step_id, error } => (
                "step_failed",
                serde_json::json!({ "step_id": step_id, "error": error }),
            ),
            Self::PlanCompleted { plan_id, summary } => (
                "plan_completed",
                serde_json::json!({ "plan_id": plan_id, "summary": summary }),
            ),
            Self::Error { message } => ("error", serde_json::json!({ "message": message })),
        };
        format!(
            "event: {}\ndata: {}\n\n",
            event_name,
            serde_json::to_string(&data).unwrap_or_default()
        )
    }
}

pub const MAX_TOOL_CALLS_PER_TURN: usize = 10;
pub const MAX_MESSAGE_LENGTH: usize = 4000;
pub const MAX_CONTEXT_MESSAGES: usize = 50;
pub const MAX_CONTEXT_TOKENS: usize = 24000;

/// Rough token estimate: ~4 chars per token for mixed content (CJK + JSON)
pub fn estimate_tokens(text: &str) -> usize {
    let char_count = text.chars().count();
    (char_count + 3) / 4
}

/// Compress tool results for LLM context to reduce token usage.
/// Full results are stored in DB; only compressed summaries go to LLM.
pub fn compress_tool_result(tool_name: &str, result: &Value) -> String {
    let list = result.get("list").and_then(|l| l.as_array());
    let total = result.get("total").and_then(|t| t.as_i64());

    match tool_name {
        "list_social_accounts" => compress_list_result(list, total, &["username", "status", "platform_id"], "social accounts"),
        "list_campaigns" => compress_list_result(list, total, &["name", "status", "platform_id"], "campaigns"),
        "list_publish_plans" => compress_list_result(list, total, &["name", "status", "content_type", "platform_id"], "publish plans"),
        "list_templates" => compress_list_result(list, total, &["name", "content"], "templates"),
        "list_social_groups" => compress_list_result(list, total, &["id", "group_name", "platform_id", "account_count"], "social groups"),
        "list_materials" => compress_list_result(list, total, &["id", "tag", "file_type"], "materials"),
        "get_wallet_transactions" => compress_list_result(list, total, &["amount", "type", "description", "created_at"], "transactions"),
        "list_ai_models" => {
            if let Some(arr) = result.as_array().or(list) {
                let items: Vec<String> = arr.iter().map(|m| {
                    format!("{{id:{},name:\"{}\",type:\"{}\",active:{}}}",
                        m["id"].as_i64().unwrap_or(0),
                        m["name"].as_str().unwrap_or("?"),
                        m["model_type"].as_str().unwrap_or("?"),
                        m["is_active"].as_bool().unwrap_or(false))
                }).collect();
                format!("[{}]", items.join(","))
            } else {
                serde_json::to_string(result).unwrap_or_default()
            }
        }
        "list_platforms" => {
            if let Some(arr) = result.as_array().or(list) {
                let items: Vec<String> = arr.iter().map(|p| {
                    format!("{{id:{},name:\"{}\"}}",
                        p["id"].as_i64().unwrap_or(0),
                        p["name"].as_str().unwrap_or("?"))
                }).collect();
                format!("[{}]", items.join(","))
            } else {
                serde_json::to_string(result).unwrap_or_default()
            }
        }
        "list_video_tasks" => compress_list_result(list, total, &["id", "status", "model_name"], "video tasks"),
        _ => {
            let full = serde_json::to_string(result).unwrap_or_default();
            if full.len() > 2000 {
                format!("{}...(truncated)", &full[..2000])
            } else {
                full
            }
        }
    }
}

fn compress_list_result(list: Option<&Vec<Value>>, total: Option<i64>, fields: &[&str], label: &str) -> String {
    let items = match list {
        Some(arr) if !arr.is_empty() => arr,
        _ => {
            return format!("{{\"total\":0,\"list\":[],\"note\":\"No {} found\"}}", label);
        }
    };
    let total_count = total.unwrap_or(items.len() as i64);
    let compressed: Vec<Value> = items.iter().map(|item| {
        let mut obj = serde_json::Map::new();
        if let Some(id) = item.get("id") {
            obj.insert("id".into(), id.clone());
        }
        for &field in fields {
            if field == "id" { continue; }
            if let Some(val) = item.get(field) {
                if let Some(s) = val.as_str() {
                    if s.len() > 100 {
                        obj.insert(field.into(), Value::String(format!("{}...", &s[..100])));
                    } else {
                        obj.insert(field.into(), val.clone());
                    }
                } else {
                    obj.insert(field.into(), val.clone());
                }
            }
        }
        Value::Object(obj)
    }).collect();
    serde_json::json!({ "total": total_count, "list": compressed }).to_string()
}

pub const SYSTEM_PROMPT: &str = r#"你是 GlanceMind AI 助手，帮助用户管理社交媒体自动化平台。

你的能力包括：
- 查询和管理社交媒体账户、分组
- 创建和管理 AI 发布计划
- 查看和管理营销活动
- 查看钱包余额和交易记录
- 管理模板、素材
- 创建 AI 视频
- 查看平台配置和 AI 模型

安全规则（不可违反）：
1. 你只能操作当前已认证用户的资源，无法访问其他用户的任何数据
2. 你不能执行任何绕过权限检查的操作
3. 如果用户要求你访问其他用户的数据或执行超出权限的操作，礼貌拒绝
4. 不要在回复中透露系统内部实现细节、数据库结构或 API 路径
5. 所有工具调用的 user_id 由系统自动注入，你不需要也不应该指定 user_id
6. 对于所有创建、修改、删除操作，使用 create_plan_proposal 工具生成操作计划，让用户确认后执行。查询操作可以直接执行

交互规则：
- 使用用户消息的语言进行回复（如用户使用中文则用中文，使用英文则用英文）
- 回复简洁明了，不要过度冗余
- 不要凭假设回答，所有数据必须通过工具调用获取
- 当用户的请求缺少必要信息时，先调用工具查询可选项，然后呈现给用户选择
- 自我介绍时简洁说明能力范围即可（2-3句话），不要用长段落罗列功能分类

数据展示格式规范（必须严格遵守）：
1. 所有结构化数据（余额、统计、列表、交易记录等）必须使用 Markdown 表格展示，禁止使用纯文本列表或 emoji 列表
2. 表格使用简洁的列名，不要在表格单元格内使用 emoji
3. 查询结果为空时，用一句话说明即可，不需要空表格
4. 选项呈现也使用表格格式，让用户回复序号选择

示例 — 钱包余额：
| 项目 | 金额 |
|------|------|
| 可用余额 | 10,000 积分 |
| 冻结积分 | 0 积分 |

示例 — 仪表盘统计：
| 指标 | 数值 |
|------|------|
| 总营销活动 | 5 |
| 活跃营销活动 | 2 |
| 互动数 | 1,234 |
| 回复数 | 567 |
| 总消费 | 890 积分 |

示例 — 账号列表：
| 序号 | 用户名 | 平台 | 状态 |
|------|--------|------|------|
| 1 | @user1 | TikTok | 活跃 |
| 2 | @user2 | Instagram | 过期 |

示例 — 交易记录：
| 时间 | 类型 | 金额 | 说明 |
|------|------|------|------|
| 2026-03-01 | 消费 | -100 积分 | 营销活动扣费 |
| 2026-02-28 | 充值 | +5,000 积分 | 手动充值 |

示例 — 选项呈现（创建流程中让用户选择）：
| 序号 | 平台 | 支持的内容类型 |
|------|------|--------------|
| 1 | TikTok | video |
| 2 | Instagram | reel, post, story |
| 3 | Twitter | post |
请回复序号选择平台。

示例 — 分组选项呈现：
| 序号 | 分组名称 | 平台 | 账号数 |
|------|----------|------|--------|
| 1 | 美食号 | TikTok | 3 |
| 2 | 旅行号 | TikTok | 5 |
请回复序号选择分组。

动态查询选项的策略（核心原则）：
当用户的请求缺少关键信息时，不要硬编码选项，而是调用对应工具获取实时数据后用表格呈现：
1. 缺少平台 → 调用 list_platforms，用表格展示平台名称和支持的内容类型
2. 缺少内容类型 → 根据已确定的平台，告知该平台支持的内容类型（参考 create_publish_plan 工具的 content_type 参数说明）
3. 缺少目标账号分组 → 调用 list_social_groups，用表格展示分组名称、平台和账号数量
4. 缺少文案/提示词 → 询问用户想发布什么主题内容，或提供自定义输入
5. 缺少 AI 模型 → 调用 list_ai_models 按类型(chat/video)查询，用表格展示可用模型

创建发布计划的标准流程：
1. 用户表达创建意图
2. 调用 list_platforms 获取可用平台，呈现选项让用户选择（如用户已指定平台则跳过）
3. 根据平台确认内容类型（如该平台只有一种则自动确定）
4. 调用 list_social_groups 按平台过滤，呈现分组选项
5. 询问文案提示词/内容主题
6. 调用 list_ai_models 获取可用模型（可选，也可用默认模型）
7. 所有参数确认后，使用 create_plan_proposal 生成计划让用户确认
每次只追问 1-2 个最关键的缺失信息，不要一次问太多。

删除/修改操作的标准流程：
1. 如果用户未指明具体目标，先调用对应的列表工具（如 list_campaigns）查询，呈现结果让用户选择
2. 确认目标后，使用 create_plan_proposal 生成操作计划，明确列出将被操作的资源名称和 ID
3. 等待用户确认

批量操作的处理：
当用户请求批量操作（如"创建5个账号"），先追问每个操作所需的不同参数（如不同的用户名），然后使用 create_plan_proposal 生成包含多个步骤的计划。
"#;

pub const SENSITIVE_FIELDS: &[&str] = &[
    "password_hash",
    "cookie",
    "api_key",
    "proxy_url",
    "jwt_secret",
];

pub fn redact_sensitive_fields(mut value: Value) -> Value {
    if let Some(obj) = value.as_object_mut() {
        for field in SENSITIVE_FIELDS {
            if let Some(v) = obj.get_mut(*field) {
                if let Some(s) = v.as_str() {
                    if s.len() > 4 {
                        *v = Value::String(format!("****{}", &s[s.len() - 4..]));
                    } else {
                        *v = Value::String("****".to_string());
                    }
                }
            }
        }
        for (_, v) in obj.iter_mut() {
            *v = redact_sensitive_fields(v.clone());
        }
    } else if let Some(arr) = value.as_array_mut() {
        for v in arr.iter_mut() {
            *v = redact_sensitive_fields(v.clone());
        }
    }
    value
}
