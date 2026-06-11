# Test Suite Ledger — social-group-platform-fix

测试惯例：rust 单测（`cargo test -p glance-mind-api`）+ python 真 DB 集成测试（`crates/api/tests/`，pytest，CI 先迁移+seed）。前端：glance_mind_front 既有 vitest/e2e 惯例。

## Unit / Component（确定性，mock/纯函数）

| ID | 对象 | 用例 | Req |
|----|------|------|-----|
| UT1 | 平台一致性校验函数（M2 纯函数 `check_group_platform(group, user_id, expected_platform_id)`——参数序以 root C2 为准） | 属性测试：任意组合——归属不符必拒、平台不符必拒、双符必放 | R004/R005/R006 |
| UT2 | aipub expand 平台过滤（rust 层；pytest 侧对应 IT5f/g） | 混平台账号集展开只产出匹配账号；warn 日志字段齐全 | R007 |
| UT3 | ~~组列表 repo 过滤独立单测~~ **删行（TG-P1-2 决议）**：Some/None 两分支已由 IT3a/IT3b 在真 DB 上覆盖（items+total 同条件断言），无独立 rust 单测必要 | — | R002（由 IT3a/b 覆盖） |

## Integration（真 DB，python）

| ID | 场景 | 断言 | Req |
|----|------|------|-----|
| IT1 | POST /social-groups 非法 platform_id=999 | 400 + 错误文案；库中无新行 | R001 |
| IT2 | POST /social-groups 合法（facebook=3） | **200+code==1000**（Protocol-F4）；查库 platform_id=3 | R001 |
| IT3 | GET /social-groups?platform_id=3；**IT3c** edge：0/负→200 空列表；非数字/超界→400 axum 纯文本（非信封，框架行为，M1 注明） | 仅返回 facebook 组；total 正确；不带参返回全部（回归） | R002/R003 |
| IT4 | PUT /social-accounts/{id} 绑定（IT4a–e）+ **批量创建 IT4f/g**（错平台组 400 且零落库原子性 / 一致组成功逐行落库） | 404 / 404 / 400(GroupPlatformMismatch) / 200 且落库 | R004 |
| IT5 | POST aipub plan：IT5a 错平台 400 / IT5b 他人组 404（ENG-12）/ IT5c 不存在 404 / **IT5e** reddit 计划配 facebook 组 400 / IT5d 成功 200+code==1000 / **IT5f** 混平台展开只产匹配任务 / **IT5g** expand 0-match→failed+billing settled+0 任务 / **IT5h** frozen_cost==2*P / **IT5i** create 0-match 400+回滚 / **IT5j** 真空组绿基线锁 | 见 M3 | R005/R007 |
| IT6 | POST/PUT campaign：social_group_id 平台不一致 / 他人组 / 一致；**IT6h 只改平台旁路→400（ENG-1）** | 400 / 404 / 成功 | R006 |
| —— | （IT6i/j campaign 运行时装配平台过滤为 **rust 测试**，cargo test agent_service，redis=None，见 M4 Task 4.3） | 仅匹配账号入选 / 0 匹配排除 comment（钉死契约，偏离 legacy 回退） | **R011** |
| IT7 | 回填迁移演练（自包含 pytest `test_backfill_migration.py` 事务重放，fixture：同平台错标组、混平台组、平票组、空组、**目标拆组名已存在组**、跨用户同名组） | 回填值正确；拆组与重挂正确（撞名挂既有组 id）；空组未动；断言 SQL=0 违规 | R008 |

## 前端组件测试（M6，vitest；Step 06 补表）

| ID | 用例 | Req |
|----|------|-----|
| FT1–FT3 | 建组弹窗：选择器存在无默认 / 未选禁提交 / 选 facebook payload=3 | R009 |
| FT4 | 发布表单组下拉按平台过滤（绿基线锁） | R010 |
| FT4b | account_count 显示 + 空组禁选 | R012 |
| FT5 | 深链三断言（自动开弹窗/非法参数忽略/参数消费） | R009 |
| FT6 | 组列表平台列渲染 | R012 |
| FT7/FT7b | 账号弹窗组下拉平台过滤 / 错误 toast=信封 msg 且恰一次（_silent 决策） | R012 |

## E2E / Workflow

| ID | 场景 | Req |
|----|------|-----|
| E2E1 | 前端：建组弹窗选 facebook → 建 facebook 发帖任务 → 组出现在候选列表 → 创建 plan 成功 | R009/R010/R002/R005（前端仓库执行，作为 M6 验收） |

## Regression

- 既有 `test_campaign_api.py` 全量绿（M4 改动面）。
- 组 CRUD 既有测试（若有）+ IT3 不带参分支保证旧契约。
- aipub 既有创建/展开测试全量绿（M3 改动面）。

## Edge / Failure

- IT5j：真空组（0 账号）行为绿基线锁——R007 改动不得改变「空组」语义（只过滤平台不匹配者）；campaign 侧真空组保持 legacy 直通并回归锁（M4 IT6j 注）。
- IT3c（归 M1）：platform_id=0/负数→200 空列表（axum Query 反序列化成功）；非数字/超 i32 字符串→400 axum 纯文本拒绝体（只断状态码，不断信封——Protocol-F3 钉死）。

## Justified Gaps

- 桌面端（gm_desktop/Flutter）建组路径：**已核实为坏**（Step 07 更正 CEO-4：createGroup 只发 `{'name': name}`，无 platform_id 且键名错，今天即反序列化失败）。本计划不改其代码；记录 known-broken + 独立跟进票。GAP 理由：既有缺陷独立于本计划，修复属独立桌面任务。
- 旧版前端兼容窗口内的「默认 reddit 组」行为：后端无法测「用户意图」，见 invariants 失败模式表，接受项。
