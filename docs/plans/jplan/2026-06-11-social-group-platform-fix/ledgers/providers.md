# Provider Test Ledger — social-group-platform-fix

**INAPPLICABLE（有据）**：本任务不新增、不修改任何 provider-backed 行为。

证据（Step 01 gate applicability，逐改动面核对）：

- M1/M2/M3/M4 改动面 = 请求 DTO、服务层校验、repo 查询条件 —— 全部在 API↔DB 层（00-context-inventory.md 列举的文件均无外部 provider 调用点被触碰）。
- M3 涉及 aipub_service 的 create_plan 校验块与 expand 过滤，不触碰 AI 任务派发 / provider 调用路径（ai_task_types 推断逻辑不改）。
- M5 = SQL 迁移；M6 = 前端表单。
- 无 LLM、无 TikHub/社媒平台 API、无支付/消息等第三方调用变更。

若 Step 04 起草时发现任一任务实际触碰 provider 调用路径，本判定作废，必须回到本 ledger 补 mock-provider + real-provider 两门后才能继续。

## Step 06 再证据（TG-P2-1）

Step 07 追加的 M4 Task 4.3 触碰 `agent_service.rs:129-187`（AI 回复编排链路上游）——已核实该段仅为 `agent_repo.get_group_active_accounts` DB 查询 + 纯函数 `enforce_daily_limits_inner`（redis 限额，None 直通），无 provider/LLM 调用；**INAPPLICABLE 判定维持**。IT6i/j 以 `redis=None` 或测试 Redis 运行。M3 触点（create_plan 校验块、expand/repo 查询）grep tikhub/deepseek/provider/http 零命中（Dep 复核）。
