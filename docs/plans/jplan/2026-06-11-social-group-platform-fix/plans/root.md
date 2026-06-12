# Root Plan — social-group-platform-fix

> 反作弊地板（注入，全文见 `../constraints/testing-constraints.md`）：测试是契约。实现者不得改弱/删除断言、不得加 skip/xfail/ignore、不得伪造通过；失败只能修生产代码，错误期望须 `ASSERTION-CHANGE-JUSTIFIED` 并重走 RED→GREEN。每个测试先红（正确原因）后绿，留证据。

## 目标

对任一平台的社媒任务（campaign）或发布任务（aipub plan），用户能且只能选到自己在该平台下的账户组，且任务触达的每个账号确属该平台。（01-first-principles.md）

## 共享契约（root 拥有，模块只消费；改动须回到 root 重审）

### C1 错误枚举（M2 实现，M2/M3/M4 复用）

`crates/api/src/error/business_error.rs` 新增：

```rust
GroupPlatformMismatch { group_platform_id: i32, expected_platform_id: i32 }
// HTTP 400；Display: "Group platform {group_platform_id} does not match required platform {expected_platform_id}"
```

定稿为 **i32 id**（ENG-2 决议）：纯校验函数只持有 id，不做名字解析；若调用方要平台名文案，由 `load_and_check_group` 装配层显式多查 `gm_platforms` 解析（不强制）。`business_error.rs` 新变体需三个 arm：`#[error(...)]` Display、`to_error_code() → ErrorCode::BadRequest`（→400）、`to_message_cn()`。

测试断言锚（Protocol-F5）：`code==2001（BadRequest 通用码）+ msg 含 "does not match required platform"`。**禁止为此新增 ErrorCode 数字**——`From<i32>`（unified_response.rs:149-196）是穷举白名单，新增枚举不配套会使 http_status 回落 500。成功断言统一 `HTTP 200 + code==1000`（api_ok! 无 201，Protocol-F4）。

归属错误复用现有 `BusinessError::GroupNotFound`（404 语义，他人组与不存在组同响应，避免组 id 枚举探测；IT5b/IT6b 统一断言 404）。

### C2 校验函数（M2 实现，M3/M4 复用）

```rust
// crates/api/src/service/validation/group_platform.rs（新文件）
/// 校验组归属与平台一致性。组不存在或不属于 user → GroupNotFound；
/// 平台不一致 → GroupPlatformMismatch。纯逻辑部分与 DB 取数分离以便属性测试。
pub fn check_group_platform(group: &SocialGroup, user_id: i32, expected_platform_id: i32)
    -> Result<(), BusinessError>;   // 纯函数（属性测试对象）
pub async fn load_and_check_group(repo: &SocialGroupRepository, group_id: i32,
    user_id: i32, expected_platform_id: i32) -> Result<SocialGroup, ApiError>; // 装配
```

### C3 列表过滤参数（M1 实现，M6 消费）

`PageRequest` 增加 `platform_id: Option<i32>`；None=不过滤（旧契约不变）。`gm_social_groups` 查询的 items 与 total 必须同条件。

### C4 状态键单写者

| 状态键 | 唯一写者 | 读者 |
|--------|----------|------|
| `gm_social_groups.platform_id` | 建组（M1 校验后写入）+ M5 回填迁移（一次性） | M1 list、M2 校验、M3/M4 校验、前端 |
| `gm_social_accounts.group_id` | M2 绑定路径 + M5 拆组重挂（一次性）+ `clear_group`（仅置 NULL，INV2-safe，SM-F4 补录） | aipub expand、agent_service 回复账号装配、组 account_count |

补注（Step 06）：
- ENG-11：`social_account_repository::batch_update_group_by_profile_names`（:275-292）写 group_id 零校验且全仓无调用方（死代码）——M2 中**删除**，防未来调用方绕过 INV2。
- FK `social_accounts_group_id_fkey` 为 `ON DELETE CASCADE`（baseline:3054）：删组会**连带删账号**——既有行为，本计划不改，但 M2/M3 测试 teardown 与实现者须知悉（SM-F4）。
- ai_chat 工具（create_social_group/update_social_account/create_campaign/create_publish_plan）经同一 service 漏斗进入上述写者——被 M1–M4 校验传递覆盖，无旁路（SM-F5）；`create_social_account` 工具 schema 里的 `group_id` 是死参数（executor 丢弃），M2 顺手删除或注记。

## 模块顺序与链式验收门（03-split-map.md 定稿；Step 07 P-1 修订）

1. **M5 草案/演练** 与 **M1** 可并行；其余严格依序 M1→M2→M3→M4→M6。
2. 每模块独立门：见各模块 plan 的验证命令；全部含 RED→GREEN 证据。
3. 后端合并门：`cargo test -p glance-mind-api` 全绿（CI）+ **pytest 本地全绿并存档输出**（Dep-F1 决议：CI 不跑 pytest——无 workflow 跑它且其需运行中 API/:8081 + 测试 DB + NATS；本地跑，输出片段入 PR 描述/执行记录；d655284 惯例实为 cargo test 口径）+ `rust-verify-change` + api-contract-guard（M1）/db-migration-guard（M5）。
4. 生产回填门（NC1，见下方上线顺序）：副本演练报告通过 → **PR-A 合并即确认**（用户合并动作=窗口确认）→ deploy 管线自动执行迁移（事务性，失败即部署中止）→ 存档 migrations job 日志（尽力）+ **权威证据=部署后导出查询**（断言 SQL 与三份清单，Dep-F6）。
5. **PR-B 合并门（SM-F6 修订）**：合并前**新鲜**跑一次生产 INV2 断言 SQL（非沿用 PR-A 部署时的旧存档）——PR-A→PR-B 窗口内 update_account/批量创建/ai_chat 绑定路径仍零校验，可再脏；PR-B 部署后**即刻**再复检一次。
6. 前端门（M6）：vitest/build 绿 + E2E1（建 facebook 组→建 facebook 发帖任务→组可选→plan 创建成功）。
7. M6 上线后复检（CEO-5；语义经 SM-F1 修订）：复跑 INV2 一致性报告；非 0 → 执行**补救脚本**（见 M5「补救脚本」节）——**不是重跑 up.sql**：校验上线后组 platform_id 已是权威（用户显式选择+M1 校验），重跑多数规则会翻转干净组的平台、打断有效 plan/campaign。补救语义=组平台冻结，仅拆/解绑 `platform != 组平台` 的账号。
8. 计划收口：traceability compile（Step 08）通过。

## 上线顺序（D2 决议；Step 07 P-1 重写，闭 CEO-1/ENG-4）

事实前提：`deploy-api.yml:43-57` 检测到 `crates/db/migrations/` 变更即自动 `diesel migration run` 并 gate deploy——迁移随部署自动执行，不存在独立人工窗口。

**两段后端发布（采纳 ENG-4 方案 b 变体）：**

1. **PR-A = M1 + M5**：组列表过滤/建组校验（向后兼容）+ 回填迁移。合并 PR-A 即触发迁移自动执行——**合并动作本身就是 NC1 的窗口确认**（副本演练报告必须先通过并附在 PR 描述）。部署后：手跑生产断言 SQL（=0）+ 导出三份清单存档。此时存量组已洗净，而校验代码尚未上线——旧数据在窗口内无新增 400 风险。
2. **PR-B = M2 + M3 + M4**：全部一致性校验。**前置门：PR-A 的生产断言=0 证据已存档。** 此后新校验面对的是已一致的数据。
3. **前端 PR（glance_mind_front）= M6**：与 PR-B 同日或随后尽快发布（压缩旧前端默认值污染窗口，CEO-5）。

全部后端改动向后兼容（Optional 参数、校验仅拒绝本就错误的请求），无 feature flag。

未来注记（CEO-7）：「一次发布到多平台」的正确形态是发布任务支持多组选择（每平台一组），不是跨平台组——勿以此需求复活跨平台组设计。

## 风险与回退

- 迁移单事务，失败自动回滚；纯 DML 无结构变更，down=no-op，恢复依赖执行前快照（M5/ENG-8 口径）。拆组属数据变更，回滚依赖事务原子性（演练必须验证）。
- 若回填后仍出现 INV2 违规（理论不应），升级方案：DB 层约束（assumptions A010 的复活条件）。
- M4 与本分支未提交 campaign 改动的冲突面：M4 实现前 rebase 并确认（D3）。
- 旧版前端在后端发布后的窗口期仍以 platform_id=1 建组：合法请求，接受（invariants 失败模式表）。

## 最终验收（Step 09 交付检查单）

- [ ] R001–**R012** 全部有 owning task + GREEN 证据（traceability matrix）
- [ ] 变异门：校验模块 cargo-mutants ≥80%（本地模块完成门，输出存档；工具 `cargo install cargo-mutants --locked`，Dep-F2 决议）
- [ ] 属性测试 UT1 三性质绿（cargo test，CI）
- [ ] 副本演练报告（附 PR-A 描述）+ 生产断言输出与三清单存档（权威=导出查询）
- [ ] PR-B 合并前置门：合并前**新鲜** INV2 断言=0 证据在档；部署后即刻复检记录
- [ ] pytest 本地全量绿输出存档（CI 无 pytest，Dep-F1 决议）
- [ ] E2E1 通过记录
- [ ] M6 上线后 INV2 复检报告（非 0 → 执行补救脚本并复验，语义见链式门 7）
- [ ] 测试改动全部独立 commit，无未解释的「生产代码+断言」混合 diff；绿基线锁已按 anti-gaming 豁免清单留存实现前绿证据
