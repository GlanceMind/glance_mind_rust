# Invariant / Failure Matrix — social-group-platform-fix

## 核心不变量

| ID | Invariant | 强制点 | 验证方式 |
|----|-----------|--------|----------|
| INV1 | `gm_social_groups.platform_id` ∈ `gm_platforms.id` | DB FK（已有）+ R001 应用层校验（给出友好 400 而非 500） | FK 已存在；测试断言 400 |
| INV2 | 组内每个账号：`gm_social_accounts.platform_id == group.platform_id` | R004（绑定时）+ R008（存量回填）+ R007（expand/冻结兜底）+ **R011（campaign 装配兜底）** | 一致性 SQL（迁移断言 + 可重复跑的检查脚本）：违规行=0 |
| INV3 | 任务/计划引用的组：`group.user_id == 任务.user_id` 且 `group.platform_id == 任务.platform_id` | R005/R006（创建/更新时） | 集成测试矩阵 |
| INV4 | 组列表过滤语义：`platform_id` 参数命中则结果全部等于该平台；缺省=不过滤（兼容） | R002 | 集成测试 + total 计数一致 |

## 失败模式矩阵

| 场景 | 状态/重试/幂等 | 处理 |
|------|----------------|------|
| 建组传非法 platform_id | 无状态写入，直接拒 | 400 `Invalid platform_id`（复用 aipub 现成错误文案风格，aipub_service.rs:60-66） |
| 绑定账号到他人组 / 不存在组 | 同上 | 404 GroupNotFound（复用 BusinessError::GroupNotFound） |
| 绑定账号到平台不一致组 | 同上 | 400 新错误：`GroupPlatformMismatch { group_platform_id, expected_platform_id }`（i32 id，ENG-2 定稿；文案含两侧平台 id） |
| plan/campaign 引用平台不一致组 | 同上 | 400 GroupPlatformMismatch |
| 存量脏数据：旧 plan 引用回填后平台不一致的组 | expand 时 R007 过滤；不 fail 整个 plan | 跳过不匹配账号 + `warn!` 日志（含 plan_id/group_id/account_id）；若过滤后 0 账号→plan 失败并写明原因 |
| 回填迁移中途失败 | diesel 迁移事务性（单 up.sql 事务） | 整体回滚；纯 DML 迁移无结构变更，down=no-op，恢复依赖执行前快照（NEW-2 口径）；拆组产生的新组在事务内，随回滚消失 |
| 迁移与在线写入竞争 | 低风险（组表写入频率低；迁移单事务、行级锁） | 迁移随 PR-A 部署事务性执行（NEW-1 修订：无人工暂停步骤）；竞争残留由 M6 上线后 INV2 复检 + 幂等重跑兜底 |
| 旧版前端（未更新）继续以 platform_id=1 建组 | 请求合法（reddit 组），不被拦 | 接受：D2 决策下前端尽快跟进；后端无法区分「真想建 reddit 组」与「默认值污染」 |

## 安全边界

- 所有校验在服务端（B4）；`group_id`/`platform_id`/`social_group_id` 一律视为不可信输入。
- 归属校验防 IDOR：R004/R005/R006 必须含「他人组→404/400」用例（现状 aipub 仅查存在性，存在跨用户引用风险——M3 顺带修复并测试）。

## 可观测性

- R007 跳过账号必须 `warn!` 结构化日志（plan_id, group_id, account_id, account_platform, plan_platform）；expand 0-match 失败原因 `error!` 日志（载体决策见 M3 IT5g——无 DB 列）。
- R011 跳过账号 `warn!`（campaign_id, group_id, skipped_account_id）。
- 迁移产出报告文件（回填计数、拆组清单、空组清单、plan/campaign 平台不一致引用清单——权威=导出查询）。
- **PR-A→PR-B 窗口写者（SM-F6）**：update_account / 批量创建 / ai_chat update_social_account 在窗口内仍零校验可再脏数据——由 PR-B 合并前新鲜断言 + 部署后即刻复检闭环。

## 集成门

- `rust-verify-change`（仓库规则）在每模块完成、commit 前必跑。
- `db-migration-guard`：M5 改 schema.rs 时；`api-contract-guard`：M1 DTO 改动时；`db-field-validation-guard`：所有 DB 写入集成测试。
- CI：迁移先行 + seed（commit d655284 惯例）。
