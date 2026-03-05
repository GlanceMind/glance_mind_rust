use super::types::*;
use crate::error::api_error::ApiError;
use crate::state::user_state::UserState;
use serde_json::{json, Value};

pub struct ToolRegistry;

impl ToolRegistry {
    pub fn definitions() -> Vec<ToolDefinition> {
        vec![
            // ── Query tools (ReadOnly) ──────────────────────────
            Self::def("get_dashboard_stats", "获取仪表盘统计数据，包括活跃任务数、总花费、互动数、回复数", SafetyLevel::ReadOnly, json!({ "type": "object", "properties": {}, "required": [] })),
            Self::def("list_social_accounts", "列出当前用户的社交媒体账户", SafetyLevel::ReadOnly, json!({ "type": "object", "properties": { "page": { "type": "integer", "description": "页码，默认1" }, "page_size": { "type": "integer", "description": "每页数量，默认20" }, "status": { "type": "string", "description": "筛选状态: active/expired/banned" }, "group_id": { "type": "integer", "description": "按分组筛选" }, "platform_id": { "type": "integer", "description": "按平台筛选" } }, "required": [] })),
            Self::def("get_account_statistics", "获取社交账户统计数据", SafetyLevel::ReadOnly, json!({ "type": "object", "properties": { "group_id": { "type": "integer", "description": "按分组筛选" } }, "required": [] })),
            Self::def("list_social_groups", "列出当前用户的社交账户分组，包含分组名称、平台ID和账号数量。创建发布计划前需要调用此工具让用户选择目标分组", SafetyLevel::ReadOnly, json!({ "type": "object", "properties": { "page": { "type": "integer" }, "page_size": { "type": "integer" } }, "required": [] })),
            Self::def("list_campaigns", "列出当前用户的营销活动", SafetyLevel::ReadOnly, json!({ "type": "object", "properties": { "page": { "type": "integer" }, "page_size": { "type": "integer" }, "status": { "type": "string", "description": "筛选状态" } }, "required": [] })),
            Self::def("get_campaign_detail", "获取营销活动详情", SafetyLevel::ReadOnly, json!({ "type": "object", "properties": { "campaign_id": { "type": "integer", "description": "活动ID" } }, "required": ["campaign_id"] })),
            Self::def("list_publish_plans", "列出AI发布计划", SafetyLevel::ReadOnly, json!({ "type": "object", "properties": { "page": { "type": "integer" }, "page_size": { "type": "integer" }, "status": { "type": "string", "description": "状态筛选: pending/ai_processing/ready/publishing/completed/failed" }, "platform_id": { "type": "integer" } }, "required": [] })),
            Self::def("get_publish_plan_detail", "获取发布计划详情，包括AI任务和发布任务", SafetyLevel::ReadOnly, json!({ "type": "object", "properties": { "plan_id": { "type": "integer", "description": "计划ID" } }, "required": ["plan_id"] })),
            Self::def("list_templates", "列出回复模板", SafetyLevel::ReadOnly, json!({ "type": "object", "properties": { "page": { "type": "integer" }, "page_size": { "type": "integer" } }, "required": [] })),
            Self::def("get_wallet_balance", "获取钱包余额（积分）", SafetyLevel::ReadOnly, json!({ "type": "object", "properties": {}, "required": [] })),
            Self::def("get_wallet_transactions", "获取钱包交易记录", SafetyLevel::ReadOnly, json!({ "type": "object", "properties": { "page": { "type": "integer" }, "page_size": { "type": "integer" } }, "required": [] })),
            Self::def("list_materials", "列出用户素材库", SafetyLevel::ReadOnly, json!({ "type": "object", "properties": { "page": { "type": "integer" }, "page_size": { "type": "integer" } }, "required": [] })),
            Self::def("list_ai_models", "列出可用的AI模型，包含名称、提供商和类型。创建发布计划时可让用户选择文案模型(chat)或视频模型(video)", SafetyLevel::ReadOnly, json!({ "type": "object", "properties": { "model_type": { "type": "string", "description": "按类型筛选: chat(文案)/video(视频)/image(图片)，不填则返回全部" } }, "required": [] })),
            Self::def("list_platforms", "列出支持的社交媒体平台及其ID。创建发布计划或营销活动前必须先调用此工具获取平台ID和名称", SafetyLevel::ReadOnly, json!({ "type": "object", "properties": {}, "required": [] })),
            Self::def("list_regions", "列出可用投放地区。可选传入 platform_id 按平台过滤。创建营销活动时需要 region_id", SafetyLevel::ReadOnly, json!({ "type": "object", "properties": { "platform_id": { "type": "integer", "description": "按平台过滤地区" } }, "required": [] })),
            Self::def("list_video_tasks", "列出AI视频生成任务", SafetyLevel::ReadOnly, json!({ "type": "object", "properties": { "page": { "type": "integer" }, "page_size": { "type": "integer" } }, "required": [] })),
            Self::def("get_plan_stats", "获取发布计划统计数据", SafetyLevel::ReadOnly, json!({ "type": "object", "properties": {}, "required": [] })),
            Self::def("search_knowledge", "搜索产品帮助文档和使用指南。当用户询问平台功能、操作方法、计费规则、功能区别等产品相关问题时调用此工具", SafetyLevel::ReadOnly, json!({ "type": "object", "properties": { "query": { "type": "string", "description": "搜索关键词，如: TikTok搜索方式、营销活动创建、计费规则、发布计划类型" } }, "required": ["query"] })),
            // ── Mutation tools ──────────────────────────────────
            Self::def("create_social_account", "创建社交媒体账户", SafetyLevel::Create, json!({ "type": "object", "properties": { "platform_id": { "type": "integer", "description": "平台ID" }, "username": { "type": "string", "description": "账户用户名" }, "device_id": { "type": "string", "description": "设备ID" }, "group_id": { "type": "integer", "description": "所属分组ID" } }, "required": ["platform_id", "username"] })),
            Self::def("create_social_group", "创建社交账户分组", SafetyLevel::Create, json!({ "type": "object", "properties": { "group_name": { "type": "string", "description": "分组名称" }, "platform_id": { "type": "integer", "description": "平台ID" } }, "required": ["group_name", "platform_id"] })),
            Self::def("create_campaign", "创建营销活动。schedule_type 默认 ONCE，ai_model_id 默认 GPT-5.2(id=2)", SafetyLevel::Create, json!({ "type": "object", "properties": { "name": { "type": "string", "description": "活动名称" }, "platform_id": { "type": "integer", "description": "平台ID，通过 list_platforms 获取" }, "region_id": { "type": "integer", "description": "地区ID，通过 list_regions 获取" }, "ai_model_id": { "type": "integer", "description": "AI模型ID，默认2(GPT-5.2)" }, "schedule_type": { "type": "string", "description": "调度类型: ONCE/INTERVAL/CRON，默认ONCE" }, "product_prompt": { "type": "string", "description": "产品/营销描述提示词" }, "keyword": { "type": "string", "description": "搜索关键词" }, "social_group_id": { "type": "integer", "description": "账号分组ID" }, "max_scan_count": { "type": "integer", "description": "最大扫描数量" }, "budget_cap": { "type": "number", "description": "预算上限(积分)" }, "target_audience": { "type": "string", "description": "目标受众描述" } }, "required": ["name", "platform_id", "region_id", "product_prompt"] })),
            Self::def("update_campaign_status", "更新营销活动状态", SafetyLevel::Modify, json!({ "type": "object", "properties": { "campaign_id": { "type": "integer" }, "status": { "type": "string", "description": "新状态: active/paused/stopped/completed" } }, "required": ["campaign_id", "status"] })),
            Self::def("create_publish_plan", "创建AI发布计划。调用前必须先通过 list_platforms 获取平台ID，通过 list_social_groups 获取分组ID。必须提供 group_id 或 social_account_id 作为发布目标", SafetyLevel::Create, json!({ "type": "object", "properties": { "platform_id": { "type": "integer", "description": "平台ID，通过 list_platforms 获取" }, "content_type": { "type": "string", "description": "内容类型，不同平台支持不同类型。TikTok: video; Instagram: reel/post/story; Facebook: post/reel; Twitter: post; Reddit: reddit_text/reddit_image/reddit_link" }, "group_id": { "type": "integer", "description": "账号分组ID，通过 list_social_groups 获取。批量发布时必填" }, "social_account_id": { "type": "integer", "description": "单个账号ID，单独发布时使用（与 group_id 二选一）" }, "chat_ai_model_id": { "type": "integer", "description": "文案AI模型ID，通过 list_ai_models(model_type=chat) 获取" }, "video_ai_model_id": { "type": "integer", "description": "视频AI模型ID，视频类型时需要，通过 list_ai_models(model_type=video) 获取" }, "image_ai_model_id": { "type": "integer", "description": "图片AI模型ID" }, "name": { "type": "string", "description": "计划名称" }, "content_prompt": { "type": "string", "description": "文案提示词，描述要生成的内容主题和风格" }, "video_prompt": { "type": "string", "description": "视频提示词，描述视频内容" } }, "required": ["platform_id", "content_type"] })),
            Self::def("retry_publish_plan", "重试失败的发布计划", SafetyLevel::Modify, json!({ "type": "object", "properties": { "plan_id": { "type": "integer" } }, "required": ["plan_id"] })),
            Self::def("delete_campaign", "删除营销活动", SafetyLevel::Destructive, json!({ "type": "object", "properties": { "campaign_id": { "type": "integer" } }, "required": ["campaign_id"] })),
            Self::def("create_plan_proposal", "为需要确认的操作创建执行计划。当需要执行创建、修改、删除等操作时，先调用此工具生成计划让用户确认", SafetyLevel::ReadOnly, json!({ "type": "object", "properties": { "title": { "type": "string", "description": "计划标题" }, "description": { "type": "string", "description": "计划描述" }, "steps": { "type": "array", "items": { "type": "object", "properties": { "tool_name": { "type": "string", "description": "要执行的工具名称" }, "tool_params": { "type": "object", "description": "工具参数" }, "description": { "type": "string", "description": "步骤描述" } }, "required": ["tool_name", "tool_params", "description"] } } }, "required": ["title", "description", "steps"] })),
        ]
    }

    fn def(name: &str, desc: &str, safety: SafetyLevel, params: Value) -> ToolDefinition {
        ToolDefinition {
            name: name.to_string(),
            description: desc.to_string(),
            safety_level: safety,
            parameters: params,
        }
    }

    pub fn get_safety_level(tool_name: &str) -> SafetyLevel {
        Self::definitions()
            .iter()
            .find(|d| d.name == tool_name)
            .map(|d| d.safety_level)
            .unwrap_or(SafetyLevel::ReadOnly)
    }

    pub fn openai_tools() -> Vec<Value> {
        Self::definitions()
            .iter()
            .map(|d| d.to_openai_function())
            .collect()
    }

    pub async fn execute(
        tool_name: &str,
        params: Value,
        user_id: i32,
        state: &UserState,
    ) -> Result<Value, ApiError> {
        let result: Value = match tool_name {
            "get_dashboard_stats" => {
                let stats = state.dashboard_service.get_overview_stats(user_id).await?;
                serde_json::to_value(&stats).unwrap_or_default()
            }
            "list_social_accounts" => {
                let page = params.get("page").and_then(|v| v.as_i64()).unwrap_or(1);
                let page_size = params.get("page_size").and_then(|v| v.as_i64()).unwrap_or(20);
                let req = crate::dto::social_account_dto::AccountListRequest {
                    page,
                    page_size,
                    status: params.get("status").and_then(|v| v.as_str()).map(String::from),
                    group_id: params.get("group_id").and_then(|v| v.as_i64()).map(|v| v as i32),
                    platform_id: params.get("platform_id").and_then(|v| v.as_i64()).map(|v| v as i32),
                    username: None,
                    device_id: None,
                };
                let accounts = state.social_account_service.list_accounts(user_id, req).await?;
                serde_json::to_value(&accounts).unwrap_or_default()
            }
            "get_account_statistics" => {
                let group_id = params.get("group_id").and_then(|v| v.as_i64()).map(|v| v as i32);
                let stats = state.social_account_service.get_statistics(user_id, group_id).await?;
                serde_json::to_value(&stats).unwrap_or_default()
            }
            "list_social_groups" => {
                let page = params.get("page").and_then(|v| v.as_i64()).unwrap_or(1);
                let page_size = params.get("page_size").and_then(|v| v.as_i64()).unwrap_or(20);
                let req = crate::dto::common::PageRequest { page, page_size, group_id: None };
                let groups = state.social_group_service.list_groups(user_id, req).await?;
                serde_json::to_value(&groups).unwrap_or_default()
            }
            "list_campaigns" => {
                let page = params.get("page").and_then(|v| v.as_i64()).unwrap_or(1);
                let page_size = params.get("page_size").and_then(|v| v.as_i64()).unwrap_or(20);
                let req = crate::dto::common::PageRequest { page, page_size, group_id: None };
                let campaigns = state.campaign_service.list_campaigns(user_id, req).await?;
                serde_json::to_value(&campaigns).unwrap_or_default()
            }
            "get_campaign_detail" => {
                let id = params["campaign_id"].as_i64().ok_or_else(|| ApiError::BadRequest("campaign_id required".into()))? as i32;
                let campaign = state.campaign_service.get_campaign(id, user_id).await?;
                serde_json::to_value(&campaign).unwrap_or_default()
            }
            "list_publish_plans" => {
                let page = params.get("page").and_then(|v| v.as_i64()).unwrap_or(1);
                let page_size = params.get("page_size").and_then(|v| v.as_i64()).unwrap_or(20);
                let query = crate::dto::aipub_dto::PlanListQueryDto {
                    page: Some(page),
                    page_size: Some(page_size),
                    status: params.get("status").and_then(|v| v.as_str()).map(String::from),
                    platform_id: params.get("platform_id").and_then(|v| v.as_i64()).map(|v| v as i32),
                    content_type: None,
                    plan_type: None,
                };
                let plans = state.aipub_service.list_plans(user_id, query).await?;
                serde_json::to_value(&plans).unwrap_or_default()
            }
            "get_publish_plan_detail" => {
                let id = params["plan_id"].as_i64().ok_or_else(|| ApiError::BadRequest("plan_id required".into()))? as i32;
                let plan = state.aipub_service.get_plan(user_id, id, Some("ai_tasks,publish_tasks".into())).await?;
                serde_json::to_value(&plan).unwrap_or_default()
            }
            "list_templates" => {
                let page = params.get("page").and_then(|v| v.as_i64()).unwrap_or(1);
                let page_size = params.get("page_size").and_then(|v| v.as_i64()).unwrap_or(20);
                let req = crate::dto::common::PageRequest { page, page_size, group_id: None };
                let templates = state.template_service.get_all_templates(user_id, req).await?;
                serde_json::to_value(&templates).unwrap_or_default()
            }
            "get_wallet_balance" => {
                let balance = state.wallet_service.get_balance(user_id).await?;
                serde_json::to_value(&balance).unwrap_or_default()
            }
            "get_wallet_transactions" => {
                let page = params.get("page").and_then(|v| v.as_i64()).unwrap_or(1);
                let page_size = params.get("page_size").and_then(|v| v.as_i64()).unwrap_or(20);
                let req = crate::dto::common::PageRequest { page, page_size, group_id: None };
                let txns = state.wallet_service.get_transactions(user_id, req).await?;
                serde_json::to_value(&txns).unwrap_or_default()
            }
            "list_materials" => {
                let page = params.get("page").and_then(|v| v.as_i64()).unwrap_or(1);
                let page_size = params.get("page_size").and_then(|v| v.as_i64()).unwrap_or(20);
                let query = crate::dto::material_dto::MaterialListQuery {
                    page: Some(page as i32),
                    page_size: Some(page_size as i32),
                    tag: None,
                    search: None,
                };
                let materials = state.material_service.list_materials(user_id, query).await?;
                serde_json::to_value(&materials).unwrap_or_default()
            }
            "list_ai_models" => {
                let model_type = params.get("model_type").and_then(|v| v.as_str());
                let models = if let Some(mt) = model_type {
                    state.config_service.get_ai_models_by_type(mt).await
                        .map_err(|e| ApiError::DatabaseError(e.to_string()))?
                } else {
                    state.config_service.get_active_ai_models().await
                        .map_err(|e| ApiError::DatabaseError(e.to_string()))?
                };
                serde_json::to_value(&models).unwrap_or_default()
            }
            "list_platforms" => {
                let platforms = state.platform_service.get_all_platforms().await
                    .map_err(|e| ApiError::DatabaseError(e.to_string()))?;
                serde_json::to_value(&platforms).unwrap_or_default()
            }
            "list_regions" => {
                let platform_id = params.get("platform_id").and_then(|v| v.as_i64()).map(|v| v as i32);
                let regions = if let Some(pid) = platform_id {
                    state.platform_service.get_regions_by_platform(pid).await
                        .map_err(|e| ApiError::DatabaseError(e.to_string()))?
                } else {
                    state.platform_service.get_all_regions().await
                        .map_err(|e| ApiError::DatabaseError(e.to_string()))?
                };
                serde_json::to_value(&regions).unwrap_or_default()
            }
            "list_video_tasks" => {
                let page = params.get("page").and_then(|v| v.as_i64()).unwrap_or(1) as i32;
                let page_size = params.get("page_size").and_then(|v| v.as_i64()).unwrap_or(20) as i32;
                let tasks = state.video_service.get_user_tasks(user_id, page, page_size).await?;
                serde_json::to_value(&tasks).unwrap_or_default()
            }
            "get_plan_stats" => {
                let stats = state.aipub_service.get_plan_stats(user_id).await?;
                serde_json::to_value(&stats).unwrap_or_default()
            }
            "search_knowledge" => {
                let query = params["query"].as_str().unwrap_or("");
                let results = super::knowledge::search(query, 3).await;
                serde_json::to_value(&results).unwrap_or_default()
            }
            "create_social_account" => {
                let dto = crate::dto::social_account_dto::CreateSocialAccountDto {
                    platform_id: params["platform_id"].as_i64().ok_or_else(|| ApiError::BadRequest("platform_id required".into()))? as i32,
                    username: params["username"].as_str().ok_or_else(|| ApiError::BadRequest("username required".into()))?.to_string(),
                    device_id: params.get("device_id").and_then(|v| v.as_str()).map(String::from),
                    cookie: None,
                    proxy_url: None,
                    daily_max_replies: None,
                    profile_name: None,
                };
                let account = state.social_account_service.create_account(user_id, dto).await?;
                serde_json::to_value(&account).unwrap_or_default()
            }
            "create_social_group" => {
                let dto = crate::dto::social_account_dto::CreateSocialGroupDto {
                    group_name: params["group_name"].as_str().ok_or_else(|| ApiError::BadRequest("group_name required".into()))?.to_string(),
                    platform_id: params["platform_id"].as_i64().ok_or_else(|| ApiError::BadRequest("platform_id required".into()))? as i32,
                };
                let group = state.social_group_service.create_group(user_id, dto).await?;
                serde_json::to_value(&group).unwrap_or_default()
            }
            "create_campaign" => {
                let mut p = params.clone();
                if let Some(obj) = p.as_object_mut() {
                    if !obj.contains_key("schedule_type") {
                        obj.insert("schedule_type".into(), json!("ONCE"));
                    }
                    if !obj.contains_key("ai_model_id") {
                        obj.insert("ai_model_id".into(), json!(2));
                    }
                }
                let dto: crate::dto::campaign_dto::CampaignCreateDto = serde_json::from_value(p)
                    .map_err(|e| ApiError::BadRequest(format!("Invalid campaign params: {}", e)))?;
                let campaign = state.campaign_service.create_campaign(user_id, dto).await?;
                serde_json::to_value(&campaign).unwrap_or_default()
            }
            "update_campaign_status" => {
                let id = params["campaign_id"].as_i64().ok_or_else(|| ApiError::BadRequest("campaign_id required".into()))? as i32;
                let status_str = params["status"].as_str().ok_or_else(|| ApiError::BadRequest("status required".into()))?;
                let status: crate::dto::campaign_dto::CampaignStatus = status_str.parse()
                    .map_err(|_| ApiError::BadRequest(format!("Invalid status: {}", status_str)))?;
                let campaign = state.campaign_service.update_status(id, user_id, status).await?;
                serde_json::to_value(&campaign).unwrap_or_default()
            }
            "create_publish_plan" => {
                let mut dto: crate::dto::aipub_dto::CreatePlanDto = serde_json::from_value(params.clone())
                    .map_err(|e| ApiError::BadRequest(format!("Invalid plan params: {}", e)))?;
                if dto.name.is_none() {
                    dto.name = Some("AI Chat 创建的计划".into());
                }
                let plan = state.aipub_service.create_plan(user_id, dto).await?;
                serde_json::to_value(&plan).unwrap_or_default()
            }
            "retry_publish_plan" => {
                let id = params["plan_id"].as_i64().ok_or_else(|| ApiError::BadRequest("plan_id required".into()))? as i32;
                let dto: crate::dto::aipub_dto::RetryPlanDto = serde_json::from_value(
                    json!({ "retry_scope": "all" })
                ).unwrap();
                let result = state.aipub_service.retry_plan(user_id, id, dto).await?;
                serde_json::to_value(&result).unwrap_or_default()
            }
            "delete_campaign" => {
                let id = params["campaign_id"].as_i64().ok_or_else(|| ApiError::BadRequest("campaign_id required".into()))? as i32;
                let _campaign = state.campaign_service.get_campaign(id, user_id).await?;
                let status: crate::dto::campaign_dto::CampaignStatus = "stopped".parse()
                    .map_err(|_| ApiError::BadRequest("Invalid status".into()))?;
                let campaign = state.campaign_service.update_status(id, user_id, status).await?;
                json!({ "deleted": true, "campaign_id": id, "final_status": campaign.status })
            }
            "create_plan_proposal" => {
                let proposal = PlanProposal {
                    title: params["title"].as_str().unwrap_or("操作计划").to_string(),
                    description: params["description"].as_str().unwrap_or("").to_string(),
                    steps: params.get("steps")
                        .and_then(|v| serde_json::from_value::<Vec<PlanStep>>(v.clone()).ok())
                        .unwrap_or_default(),
                };
                serde_json::to_value(proposal).unwrap_or_default()
            }
            _ => {
                return Err(ApiError::BadRequest(format!("Unknown tool: {}", tool_name)));
            }
        };

        Ok(redact_sensitive_fields(result))
    }
}
