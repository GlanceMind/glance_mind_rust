use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
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
    pub display_hint: Option<String>,
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
#[serde(rename_all = "snake_case")]
pub enum QuestionnaireControl {
    Choice,
    Combobox,
    Input,
    Textarea,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionnaireOption {
    pub value: String,
    pub label: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionnaireField {
    pub key: String,
    pub label: String,
    pub control: QuestionnaireControl,
    pub required: bool,
    pub options: Vec<QuestionnaireOption>,
    pub placeholder: Option<String>,
    pub helper_text: Option<String>,
    pub default_value: Option<Value>,
    pub max_length: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionnaireAutoFilled {
    pub key: String,
    pub label: String,
    pub value: Value,
    pub display: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionnairePayload {
    pub questionnaire_id: String,
    pub intent: String,
    pub title: String,
    pub description: Option<String>,
    pub submit_label: String,
    pub fields: Vec<QuestionnaireField>,
    pub auto_filled: Vec<QuestionnaireAutoFilled>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionnaireSubmission {
    pub questionnaire_id: Option<String>,
    pub intent: String,
    #[serde(default)]
    pub answers: BTreeMap<String, Value>,
    #[serde(default)]
    pub auto_filled: BTreeMap<String, Value>,
    pub display_message: Option<String>,
}

impl QuestionnaireSubmission {
    pub fn merged_values(&self) -> BTreeMap<String, Value> {
        let mut merged = self.auto_filled.clone();
        for (key, value) in &self.answers {
            merged.insert(key.clone(), value.clone());
        }
        merged
    }
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
        display_hint: Option<String>,
    },
    #[serde(rename = "questionnaire")]
    Questionnaire { questionnaire: QuestionnairePayload },
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
    StepFailed { step_id: i32, error: String },
    #[serde(rename = "plan_completed")]
    PlanCompleted { plan_id: i32, summary: String },
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
                display_hint,
            } => (
                "tool_call_result",
                serde_json::json!({
                    "tool_call_id": tool_call_id,
                    "result": result,
                    "success": success,
                    "display_hint": display_hint,
                }),
            ),
            Self::Questionnaire { questionnaire } => {
                ("questionnaire", serde_json::json!(questionnaire))
            }
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
        "list_social_accounts" => compress_list_result(
            list,
            total,
            &["username", "status", "platform_id"],
            "social accounts",
        ),
        "list_campaigns" => {
            compress_list_result(list, total, &["name", "status", "platform_id"], "campaigns")
        }
        "list_publish_plans" => compress_list_result(
            list,
            total,
            &["name", "status", "content_type", "platform_id"],
            "publish plans",
        ),
        "list_templates" => compress_list_result(list, total, &["name", "content"], "templates"),
        "list_social_groups" => compress_list_result(
            list,
            total,
            &["id", "group_name", "platform_id", "account_count"],
            "social groups",
        ),
        "list_materials" => {
            compress_list_result(list, total, &["id", "tag", "file_type"], "materials")
        }
        "get_wallet_transactions" => compress_list_result(
            list,
            total,
            &["amount", "type", "description", "created_at"],
            "transactions",
        ),
        "list_ai_models" => {
            if let Some(arr) = result.as_array().or(list) {
                let items: Vec<String> = arr
                    .iter()
                    .map(|m| {
                        format!(
                            "{{id:{},name:\"{}\",type:\"{}\",active:{}}}",
                            m["id"].as_i64().unwrap_or(0),
                            m["name"].as_str().unwrap_or("?"),
                            m["model_type"].as_str().unwrap_or("?"),
                            m["is_active"].as_bool().unwrap_or(false)
                        )
                    })
                    .collect();
                format!("[{}]", items.join(","))
            } else {
                serde_json::to_string(result).unwrap_or_default()
            }
        }
        "create_questionnaire_proposal" => {
            let intent = result["intent"].as_str().unwrap_or("unknown");
            let field_count = result["fields"]
                .as_array()
                .map(|fields| fields.len())
                .unwrap_or(0);
            format!(
                "questionnaire(intent=\"{}\",fields={})",
                intent, field_count
            )
        }
        "list_platforms" => {
            if let Some(arr) = result.as_array().or(list) {
                let items: Vec<String> = arr
                    .iter()
                    .map(|p| {
                        format!(
                            "{{id:{},name:\"{}\"}}",
                            p["id"].as_i64().unwrap_or(0),
                            p["name"].as_str().unwrap_or("?")
                        )
                    })
                    .collect();
                format!("[{}]", items.join(","))
            } else {
                serde_json::to_string(result).unwrap_or_default()
            }
        }
        "list_regions" => {
            if let Some(arr) = result.as_array().or(list) {
                let items: Vec<String> = arr
                    .iter()
                    .map(|r| {
                        format!(
                            "{{id:{},platform_id:{},code:\"{}\",name:\"{}\"}}",
                            r["id"].as_i64().unwrap_or(0),
                            r["platform_id"].as_i64().unwrap_or(0),
                            r["code"].as_str().unwrap_or("?"),
                            r["display_name"]
                                .as_str()
                                .or(r["name"].as_str())
                                .unwrap_or("?")
                        )
                    })
                    .collect();
                format!("[{}]", items.join(","))
            } else {
                serde_json::to_string(result).unwrap_or_default()
            }
        }
        "list_video_tasks" => {
            compress_list_result(list, total, &["id", "status", "model_name"], "video tasks")
        }
        "list_video_cases" => compress_list_result(
            list,
            total,
            &["id", "task_no", "title", "status"],
            "video cases",
        ),
        "list_publish_tasks" => compress_list_result(
            list,
            total,
            &["id", "status", "platform_id", "account_name"],
            "publish tasks",
        ),
        "list_campaign_contents" => compress_list_result(
            list,
            total,
            &["id", "content_type", "author", "text"],
            "campaign contents",
        ),
        "list_dm_conversations" => {
            if let Some(convs) = result.get("conversations").and_then(|c| c.as_array()) {
                let items: Vec<String> = convs
                    .iter()
                    .map(|c| {
                        format!(
                            "{{conv_id:\"{}\",remote_username:\"{}\",unread:{},platform_id:{}}}",
                            c["conv_id"].as_str().unwrap_or("?"),
                            c["remote_username"].as_str().unwrap_or("?"),
                            c["unread_count"].as_i64().unwrap_or(0),
                            c["platform_id"].as_i64().unwrap_or(0)
                        )
                    })
                    .collect();
                format!("{{conversations:[{}]}}", items.join(","))
            } else {
                serde_json::to_string(result).unwrap_or_default()
            }
        }
        "list_notifications" => {
            if let Some(arr) = result.as_array().or(list) {
                let items: Vec<String> = arr
                    .iter()
                    .map(|n| {
                        format!(
                            "{{id:{},title:\"{}\",read:{}}}",
                            n["id"].as_i64().unwrap_or(0),
                            n["title"].as_str().unwrap_or("?"),
                            n["read"].as_bool().unwrap_or(false)
                        )
                    })
                    .collect();
                format!("[{}]", items.join(","))
            } else {
                serde_json::to_string(result).unwrap_or_default()
            }
        }
        "search_knowledge" => {
            let s = serde_json::to_string(result).unwrap_or_default();
            if s.len() > 4000 {
                format!("{}...(truncated)", &s[..4000])
            } else {
                s
            }
        }
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

fn compress_list_result(
    list: Option<&Vec<Value>>,
    total: Option<i64>,
    fields: &[&str],
    label: &str,
) -> String {
    let items = match list {
        Some(arr) if !arr.is_empty() => arr,
        _ => {
            return format!(
                "{{\"total\":0,\"list\":[],\"note\":\"No {} found\"}}",
                label
            );
        }
    };
    let total_count = total.unwrap_or(items.len() as i64);
    let compressed: Vec<Value> = items
        .iter()
        .map(|item| {
            let mut obj = serde_json::Map::new();
            if let Some(id) = item.get("id") {
                obj.insert("id".into(), id.clone());
            }
            for &field in fields {
                if field == "id" {
                    continue;
                }
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
        })
        .collect();
    serde_json::json!({ "total": total_count, "list": compressed }).to_string()
}

pub const SYSTEM_PROMPT: &str = r#"你是 GlanceMind AI 助手，一个智能全能助理。

你有两类能力：

**一、通用智能助理能力（无需工具）：**
- 回答各类知识问题（时事、科技、历史、文化、商业等）
- 文案撰写、翻译、摘要、头脑风暴
- 数据分析、策略建议、行业洞察
- 编程和技术问题解答
- 任何用户提出的通用问题

**二、GlanceMind 平台管理能力（通过工具调用）：**
- 查询和管理社交媒体账户、分组
- 创建和管理 AI 发布计划（查看发布任务、视频案例）
- 查看和管理营销活动（含爬虫任务和内容）
- 查看钱包余额和交易记录
- 管理回复模板（查看、创建、删除、AI自动生成）
- 管理素材库（查看详情、删除、查看标签）
- 生成AI视频（支持Vidu/Jimeng模型，查看任务详情）
- 管理DM群控（查看会话、消息、回复、统计、标记已读）
- 查看和管理通知
- 查看平台配置、地区和 AI 模型
- 产品知识检索：通过 search_knowledge 工具搜索帮助文档

重要原则：当用户的问题不涉及平台操作时，直接用你自身的知识回答，不要拒绝或声称"只能回答平台相关问题"。你是一个全能助手，平台管理只是你能力的一部分。

产品知识检索规则：
当用户询问平台功能、操作方法、计费规则等产品相关问题时，必须先调用 search_knowledge 工具检索帮助文档，基于检索结果回答，不要凭记忆回答。
可搜索的主题包括：
- 各平台支持的功能（TikTok/Instagram/Reddit/Twitter/Facebook 的搜索模式和内容类型）
- 营销活动 vs AI 发布计划的区别和适用场景
- 营销活动创建参数（搜索模式、自动互动选项、预算和计费）
- AI 发布计划类型（批量文本/单视频/账号养号/Reddit 帖子）
- 计费规则（积分消耗、预算冻结与结算）
- DM 群控功能（私信管理、多设备收件箱）
- 账号和分组管理
- 回复模板和 AI 人设配置
- AI 市场洞察功能
- 自动化执行器使用方法

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
- 对于平台相关的数据查询，必须通过工具调用获取，不要凭假设回答
- 对于通用知识问题（新闻、分析、写作、翻译等），直接用自身知识回答，无需调用工具
- 当用户的请求缺少必要信息时，先调用工具查询可选项，然后呈现给用户选择
- 自我介绍时简洁说明能力范围即可（2-3句话），不要用长段落罗列功能分类
- 绝对不要拒绝回答非平台相关的问题，你是全能助手

回复格式规范（必须严格遵守，直接影响前端渲染质量）：

总体原则：
- 先给出 1-2 句总结摘要，再展示结构化数据
- 回复中禁止使用 # 或 ## 标题（会在聊天气泡中显得过大），改用 **加粗文本** 或 ### 三级标题
- 表格和代码块的前后必须各有一个空行（\n\n），否则 Markdown 渲染会失败

表格规范：
1. 所有结构化数据（余额、统计、列表、交易记录等）必须使用 Markdown 表格展示
2. 表格最多 5 列；超过 5 列时拆分为多个表格，或省略次要字段
3. 表格列名简洁（2-4 字），禁止在表头或单元格中使用 emoji
4. 数值右对齐便于比较，状态字段使用纯文本（如 active、paused），不使用 emoji 图标
5. 表格前后必须有空行，格式示例：

上面的文字内容

| 列A | 列B | 列C |
|-----|-----|-----|
| 值1 | 值2 | 值3 |

下面的文字内容

6. 查询结果为空时，用一句话说明即可，不需要空表格
7. 表格中长文本（超过 30 字符）截断并加 ... 后缀

列表规范：
- 使用 - 无序列表时，每项之间不要加空行（紧凑列表）
- 有序步骤使用 1. 2. 3. 编号列表
- 列表项内容简洁，每项不超过一行

代码与技术内容：
- 代码片段使用围栏代码块（```language），前后各留一个空行
- 行内代码、字段名、ID 等使用单反引号 `code`
- 不要在普通文本回复中使用代码块，仅在展示技术内容时使用

段落与换行：
- 段落之间使用一个空行分隔
- 不要在一个段落内使用多余的换行符
- 使用 --- 分隔线来划分不同主题区域（前后各留一个空行）

数值格式：
- 积分/金额：显示为 `1,000 pt` 或 `¥100.00`，带千分位分隔符
- 百分比：`85.2%`
- 日期时间：`2026-03-10 14:30`，不需要秒

参数收集策略（Cursor 风格一次性多选项）：
当用户表达创建意图时：
1. 从用户输入中提取已知参数（如平台、预算、产品描述等）
2. 一次性并行调用所有需要的查询工具（list_platforms、list_regions、list_social_groups、list_ai_models）
3. 如果当前客户端支持结构化问卷，优先调用 create_questionnaire_proposal 返回可渲染的字段定义；只有旧客户端才使用纯文本 Q1/Q2/Q3
4. 已从输入提取的参数和有合理默认值的参数，在底部"已自动填充"区域展示
5. 新客户端中，用户会通过结构化问卷直接提交；旧客户端中，用户一条消息回复所有选择（如"1A 2B 3A"），或回复"确认"接受所有推荐值

结构化问卷规则：
- 当客户端支持结构化问卷时，不要把多字段参数收集主要依赖在自然语言文本上
- 对于 AI 视频生成，优先调用 create_questionnaire_proposal(intent=generate_video)
- 视频问卷至少应覆盖：orientation、seconds、prompt_mode、prompt_input；默认视频模型放在 auto_filled 区域
- 对于创建 AI 发布计划，优先调用 create_questionnaire_proposal(intent=create_publish_plan)
- 发布计划问卷：从用户输入中提取已知的 platform_id、content_type、group_id、content_prompt 作为参数传入，后端会自动把已知参数放入 auto_filled，只为缺失字段生成交互控件
- 示例：用户说"帮我在TikTok发一批视频" -> 调用 create_questionnaire_proposal(intent=create_publish_plan, platform_id=2, content_type=video)
- create_questionnaire_proposal 返回后，正文只需用 1 句话提示用户在下方完成选择
- 当用户提交 questionnaire_submission 后，不要再次追问已提交字段，直接使用这些结构化参数调用 create_plan_proposal 生成待确认计划
- 旧客户端回退时，仍可使用下面的 Q1/Q2/Q3 文本格式

格式规范 -- 选项式问题：
每个问题用 **Q{n}** 标记，选项用大写字母 A/B/C 标记。最后一个选项可以是"其他(请输入)"。

示例 — 创建营销活动（用户说"在TikTok创建一个卖潮鞋的营销活动，预算200pt，每次扫描5个"）：

---

**Q1** 选择投放地区
A) 美国 - TikTok US
B) 全球
C) 其他... (请输入国家/地区)

**Q2** 选择账号分组
A) jacksoom-windows (3个账号)
B) my-tiktok-group (5个账号)
C) 不绑定分组

**Q3** 输入搜索关键词
> 推荐：sneakers, trendy shoes, 潮鞋
A) 使用推荐关键词
B) 自定义 (请输入关键词)

已自动填充：
- 平台：TikTok ✓（已从输入识别）
- 产品描述：年轻人喜欢的潮鞋 ✓
- 预算：200pt ✓
- 最大扫描数：5 ✓
- AI 模型：GPT-5.2（默认）
- 调度类型：单次执行（默认）

请回复选择（如 `1A 2A 3A`），或直接回复 `确认` 接受所有推荐值。如需修改已填充参数，直接说明（如"预算改为500pt"）。

---

示例 — 创建发布计划（用户说"帮我发一批TikTok视频"，客户端支持结构化问卷）：
→ 调用 create_questionnaire_proposal(intent=create_publish_plan, platform_id=2, content_type=video)
→ 正文只说一句话："我帮你准备了发布计划的配置表，请在下方完成选择。"

示例 — 创建发布计划（旧客户端，不支持结构化问卷时的回退格式）：

---

**Q1** 选择账号分组
A) jacksoom-windows (3个账号)
B) 创建新分组... (请输入分组名和平台)

**Q2** 输入内容主题/文案提示词
> 请描述你想发布的内容主题

已自动填充：
- 平台：TikTok ✓
- 内容类型：video ✓
- AI 模型：GPT-5.2（默认）
- 计划名称：AI Chat 创建的计划（默认）

请回复选择和输入内容。

---

创建类操作标准流程（营销活动/发布计划/社交账号/分组）：
1. 用户表达创建意图
2. 从用户输入中提取所有已知参数
3. 一次性并行调用所有需要的查询工具获取选项数据
4. 优先调用 create_questionnaire_proposal 生成结构化问卷（如支持）；仅旧客户端回退到 Q1/Q2/Q3 文本
5. create_questionnaire_proposal 返回后，正文只需用 1 句话提示用户在下方完成选择
6. 用户提交后，直接使用结构化参数调用 create_plan_proposal 生成最终操作计划
7. 用户确认后执行

关键原则：
- 能从用户输入推断的参数，直接填充，不要再问
- 只有一个选项的问题（如只有一个分组），自动选中并放入"已自动填充"
- 有合理默认值的参数（AI模型、调度类型），自动填充
- 力求用户只需一次交互就能完成所有参数确认
- 当客户端支持结构化问卷时，严禁使用 Q1/Q2/Q3 + A/B/C 文本格式

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
