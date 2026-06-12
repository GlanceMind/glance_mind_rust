# 实现交接 — social-group-platform-fix（Step 09 Final Handoff）

> 给实现者（人或 subdriven 子 agent）的唯一入口。读本文件 + root plan + 当前要做的模块 plan 即可开工；不需要读评审/补丁历史。

## 你要解决的问题（一句话）

对任一平台的社媒任务（campaign）或发布任务（aipub plan），用户能且只能选到自己在该平台下的账户组，且任务触达的每个账号确属该平台。根因：Web 建组弹窗硬编码 `platformId: 1`（全部组被建成 Reddit）+ 后端列表不过滤、绑定/消费零校验 + 生产存量 68/69 组平台标错。

## 执行顺序（严格）

```text
1. M1-group-crud + M5-data-backfill 并行实现（互不依赖）
2. → PR-A = M1+M5 合并（合并=回填执行确认；演练报告+快照先就绪）→ 生产断言=0 存档
3. M2-binding-invariant → M3-aipub-guard → M4-campaign-guard（依赖 M2 的校验函数）
4. → PR-B = M2+M3+M4 合并（前置门：合并前新鲜断言=0）→ 部署后即刻复检
5. M6-frontend（glance_mind_front 仓库，独立 PR，与 PR-B 同日或尽快）
6. → E2E1 验收 → M6 后 INV2 复检（非 0 → 跑 M5 的补救脚本，不是重跑迁移）
```

## 计划文件

- 共享契约/链式门/上线序：`plans/root.md`（C1 错误枚举、C2 校验函数签名、C3 过滤参数、C4 状态键单写者——契约冻结，改动须回 root 重审）
- 模块（每个含 RED 测试/预期失败形态/实现要点/GREEN 命令/边界禁区）：
  `plans/modules/M1-group-crud.md`（R001–R003）/ `M2-binding-invariant.md`（R004+共享件）/ `M3-aipub-guard.md`（R005,R007 含计费与退款）/ `M4-campaign-guard.md`（R006,R011 含运行时过滤）/ `M5-data-backfill.md`（R008+补救脚本）/ `M6-frontend.md`（R009,R010,R012）
- 需求/测试/约束底册：`ledgers/requirements.md`（R001–R012+D1–D3 决策）、`ledgers/test-suite.md`、`ledgers/anti-gaming-test-quality.md`（RED 表+绿基线豁免清单+变异门）、`constraints/testing-constraints.md`（**不可违背**）

## 硬规则提醒（实现者必读）

1. 每个测试先红（预期失败形态在模块 plan 与 anti-gaming ledger 里钉死）后绿，留双输出证据；绿基线锁清单内的测试在首个实现 commit 前跑一次存绿证据。
2. 不得改弱/删除断言、不得 skip/ignore；既有测试断言只读；测试改动单独 commit。
3. 成功断言=HTTP 200+code==1000（无 201）；GroupPlatformMismatch 断言锚=code==2001+msg 子串；**禁止新增 ErrorCode 数字**。
4. pytest 是本地门（CI 不跑它）：跑完存档输出；cargo test 在 CI。变异门：`cargo install cargo-mutants --locked && cargo mutants -p glance-mind-api -f '*validation/group_platform*' -- -- --lib` ≥80%。
5. M4 动工前先确认本分支未提交的 campaign 文件改动已落地/隔离（D3）。
6. M5 的 up.sql 不得含 BEGIN/COMMIT；生产数据分布执行前复核（记录称 68/69=reddit）。

## 状态/验证总览

- 计划族 9 步全部完成：init → first-principles → ledgers → split → 7 份草案 → CEO/Eng/Design 评审（P1×7 修复+复核 PASS）→ 7 域评审（P1×7 修复）→ traceability compile **PASS**。
- 外部待办（用户/运维）：PR-A 合并人负责合并前快照（pg_dump，归档路径写入 PR 描述）；pg_dump 生产凭据持有人确认；gm_desktop 建组已坏（独立修复票，不在本计划）。
