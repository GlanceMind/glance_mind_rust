# Step 03 — Scope Split & Module Map

## Split 决策：root + 6 modules

触发器证据（任一即 split，此处命中四项）：

- 实现任务 >7（R001–R010 至少 10 个任务面 + 迁移 + 前端）。
- 生产文件 >5（dto/common.rs、social_group_service/repository/handler、social_account_service、aipub_service、campaign_service、error 枚举、迁移、schema.rs、前端 2 文件）。
- 独立子系统 >2（组 CRUD、账号绑定、aipub、campaign、DB 迁移、前端独立仓库）。
- 评审专业面不同（DB 迁移/回填 vs API 契约 vs 前端 UX）。

## Root Plan 职责（plans/root.md）

- 共享接口契约：`BusinessError::GroupPlatformMismatch` 错误枚举（一次新增，多处复用）；校验函数 `check_group_platform` 的签名与语义（M2 产出，M3/M4 复用）；`PageRequest.platform_id: Option<i32>` 字段语义。
- 跨模块不变量 INV1–INV4 的归属表。
- 执行顺序与链式验收门（见下）。
- 上线顺序（D2）：后端含迁移先行，前端跟进。
- 最终验收：traceability compile + `rust-verify-change` + E2E1。

## Module Map

| module_id | 范围（owner surface） | 输入文件 | 输出（改动）文件 | Reqs | 依赖 | 独立验证命令 |
|---|---|---|---|---|---|---|
| M1-group-crud | 组 CRUD 平台化：list 服务端过滤 + create 平台合法性校验 | dto/common.rs, social_group_{service,repository,handler}.rs, social_account_dto.rs | 同左 + agent_handler.rs/template_service.rs/tool_registry.rs（9 处 PageRequest 字面量补 None，ENG-5）+ 新增集成测试 | R001,R002,R003 | 无（首发） | `cargo test -p glance-mind-api` + pytest IT1–IT3(c) + api-contract-guard |
| M2-binding-invariant | 校验函数提取 + 账号↔组绑定校验 | social_account_service.rs, social_group_repository.rs, error/business_error.rs | 新增 `service/validation/group_platform.rs`（校验纯函数+repo 装配）、error 枚举、绑定路径 | R004（+校验函数供 M3/M4） | M1（错误枚举/DTO 已就位） | cargo test（UT1 属性测试）+ pytest IT4 + cargo-mutants ≥80% |
| M3-aipub-guard | aipub create_plan 组归属+平台校验；expand 平台过滤 | aipub_service.rs, aipub_repository.rs | 同左 + 单测/集成测试 | R005,R007 | M2（复用校验函数） | cargo test（UT2）+ pytest IT5 + aipub 回归 |
| M4-campaign-guard | campaign create/update 组校验 + 运行时装配平台过滤 | campaign_service.rs, agent_service.rs（D3：先确认分支未提交 campaign 改动已落地） | 同左 + test_campaign_api.py 新用例 + agent_service rust 测试 | R006,R011 | M2；rebase 前置 | pytest IT6 + test_campaign_api.py 全量 + `cargo test -p glance-mind-api`（IT6i/j） |
| M5-data-backfill | 回填迁移：同平台回填/混平台拆组/空组保留 + 断言 + 报告 | schema.rs（只读）、迁移样例 2026-06-03 | `crates/db/migrations/2026-06-11-000001_social_group_platform_backfill/{up,down}.sql` + 演练报告 | R008 | 独立可先行；须在 M2–M4 校验上生产**之前**执行（旧数据先洗净） | 库副本演练 IT7 + 断言 SQL=0 + db-migration-guard |
| M6-frontend | 建组弹窗平台选择器 + 组列表平台列 + 发布表单/账号弹窗防呆 | glance_mind_front: Accounts.tsx, AIPubPlanCreate.tsx；只读参照 platformContentTypes.ts | Accounts.tsx, AIPubPlanCreate.tsx, i18n locales en.ts/zh.ts + 前端测试 FT1–FT7 | R009,R010,R012 | M1（R002 服务端过滤生效后验收 E2E1） | 前端仓库 vitest/build + E2E1 |

共享需求标注：校验函数与 GroupPlatformMismatch 由 M2 拥有、M3/M4 只消费（接口冻结在 root plan）；R002 由 M1 拥有、M6 消费。

## 执行顺序与链式门

```text
（Step 07 P-1 修订：两段后端发布）
M1(组CRUD) ─┬─→ PR-A = M1+M5 ──合并即触发──→ deploy 自动跑回填迁移 ──→ 生产断言=0 存档（PR-B 前置门）
M5(迁移+演练)┘                                                              ↓
M2(校验函数+绑定) ─→ M3(aipub) ─→ M4(campaign+运行时过滤) ─→ PR-B 合并部署 ─→ M6(前端 PR) ─→ E2E1
                                                                              ↓
                                                            M6 后复检 INV2（非0→幂等重跑回填） → 计划收口
```

- M5 与 M1 可并行起草/实现；回填经 PR-A 的 deploy 管线自动执行（ENG-4 事实），先于 PR-B 校验代码。
- 链式门：每模块绿 → PR-A 部署+断言存档 → PR-B 全量回归绿 → 前端发布 → E2E1 → 复检。

## Manifest 更新

module queue 置为 M1→M2→M3→M4→M5→M6 的起草顺序（M5 草案可与 M1 并行，但单次 Step 04 仅起草一个）。首个 Step 04 目标：**root plan**，随后 M1。
