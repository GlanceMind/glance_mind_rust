# Context Inventory — social-group-platform-fix

所有路径为绝对路径或相对各自仓库根。每条带一行相关性说明。证据均来自 2026-06-11 的只读分析会话（已逐文件验证）。

## 后端 (glance_mind_rust, 本仓库)

### 数据模型 / Schema

- `crates/db/src/schema.rs:1461-1468` — `gm_social_groups` 表定义：`platform_id Int4` 非空。组在模型上是平台维度的。
- `crates/db/src/schema.rs:1952` — `gm_social_groups -> gm_platforms (platform_id)` joinable。
- `crates/db/src/schema.rs:1887` — `gm_aipub_plans -> gm_social_groups (group_id)`。
- `crates/db/src/schema.rs:1899` — `gm_campaigns -> gm_social_groups (social_group_id)`。
- `crates/db/src/schema.rs:1950` — `gm_social_accounts -> gm_social_groups (group_id)`；账号的真实平台在 `gm_social_accounts.platform_id`。
- `crates/db/src/entity/social_group.rs` — `SocialGroup` / `NewSocialGroup` 实体。
- `crates/db/migrations/2026-01-20-134354_00000000000000_baseline/up.sql:905-908` — `gm_platforms` 表；平台 ID 映射：reddit=1, tiktok=2, facebook=3, instagram=4, twitter=5（与前端 `platformContentTypes.ts` 硬编码一致）。

### 组 CRUD（缺陷点 1、2）

- `crates/api/src/dto/common.rs:4-11` — `PageRequest { page, page_size, group_id }`，**无 platform_id** → list 接口静默丢弃前端的平台过滤参数。
- `crates/api/src/repository/social_group_repository.rs:27-47` — `find_by_user` 只按 user_id 过滤，无平台条件。
- `crates/api/src/service/social_group_service.rs:23-71` — `list_groups` 无平台过滤；`create_group` 对 `dto.platform_id` 不做校验直接入库。
- `crates/api/src/handler/social_group_handler.rs` — 4 个 handler（list/create/update/delete），路由层。
- `crates/api/src/dto/social_account_dto.rs:47-55` — `CreateSocialGroupDto { platform_id, group_name }`；`UpdateSocialGroupDto { group_name }`（update 不能改平台——保持）。
- `crates/api/src/routes/social_group.rs` — 路由挂载。

### 账号↔组绑定（缺陷点 3）

- `crates/api/src/service/social_account_service.rs:127-136` — update 账号时 `group_id` 只判 `gid==0`，不校验组存在/归属/平台一致。
- `crates/api/src/service/social_account_service.rs:290` — create 账号路径同样直传 `dto.group_id`。
- `crates/api/src/repository/social_account_repository.rs` — `count_by_group` 等组关联查询。

### aipub 发布工作流（缺陷点 4a）

- `crates/api/src/service/aipub_service.rs:60-235` — `create_plan` 校验：plan_type 决定 group_id/social_account_id 互斥；校验了 plan 的 `platform_id` 合法性（Invalid platform_id），**但不校验 group 的归属与组平台==plan 平台**。
- `crates/api/src/service/aipub_service.rs:1459-1490` — `expand` 按 `get_group_account_ids(gid)` 展开任务；组内混平台账号会被原样展开到错误平台。
- `crates/api/src/repository/aipub_repository.rs` — `get_group_name` / `get_group_account_ids`。
- `crates/api/src/dto/aipub_dto.rs:19-57` — plan DTO 同时携带 `group_id` 与 `platform_id`。

### campaign 社媒任务（缺陷点 4b）

- `crates/api/src/service/campaign_service.rs:115,320,542` — create/update 直传 `social_group_id`，无归属/平台校验。
- `crates/api/src/dto/campaign_dto.rs` — campaign DTO（本分支有未提交改动，注意 rebase 面）。
- `crates/api/src/repository/campaign_repository.rs` — campaign 持久化（本分支有未提交改动）。

### 测试面

- `crates/api/tests/test_campaign_api.py` — campaign API 集成测试（python，本分支有未提交改动）。
- `crates/api/tests/` — 现有集成测试惯例（python + 真 DB）；DB-writing 集成测试须遵循 `db-field-validation-guard`。
- CI：迁移先行 + seed data（见 commit d655284 惯例）。

### 迁移惯例

- `crates/db/migrations/` — diesel 迁移目录；命名 `YYYY-MM-DD-NNNNNN_slug/up.sql+down.sql`；改 schema 须同步 `schema.rs`（`db-migration-guard`）。
- 参考样例：`crates/db/migrations/2026-06-03-000001_instagram_post_campaign_scoped_uniqueness/`（未提交，同类「约束+回填」迁移）。

## 前端 (glance_mind_front, 独立仓库 /Users/jacksoom/programer/aihub/glance_mind_front)

- `apps/web/src/pages/Accounts.tsx:105-107` — 建组 state 默认 `platformId: 1`（Reddit）——**根因**。
- `apps/web/src/pages/Accounts.tsx:2009-2026` — 建组弹窗 UI 只有组名输入，无平台选择器。
- `apps/web/src/pages/Accounts.tsx:561-567` — 提交 `platform_id: newGroup.platformId`（恒为 1）。
- `apps/web/src/pages/AIPubPlanCreate.tsx:765-779` — fetchGroups 携带 `platform_id` 参数（后端忽略）。
- `apps/web/src/pages/AIPubPlanCreate.tsx:468` — 客户端兜底过滤 `g.platform_id === formData.platformId`。
- `apps/web/src/pages/aipubTargetContract.ts:13-19` — 内容类型→目标类型契约：video/reel→account，其余→group（Facebook post/story 强制选组）。
- `apps/web/src/config/platformContentTypes.ts:39-241` — 平台 ID 硬编码映射 + 各平台内容类型。

## 生产数据事实（来自既有排查记录，执行回填前须重新核实）

- 生产 DB：47.236.115.179 / aihub_db。
- `gm_social_groups.platform_id` 不可信：68/69 组 = reddit。
- 账号真实平台在 `gm_social_accounts.platform_id`，组内可能混平台。

## 既有未提交工作（rebase/冲突面）

- 本分支 `claude/oauth-authorize-grant-endpoint` 有未提交改动：campaign_dto.rs / campaign_repository.rs / campaign_service.rs / test_campaign_api.py / 一个未提交迁移目录。M4（campaign 校验）规划时须假设这些文件可能先行变化。
