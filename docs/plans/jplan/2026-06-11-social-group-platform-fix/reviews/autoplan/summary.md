# Step 05 Review Summary — manual equivalent（CEO/Eng/Design 三独立评审 agent，2026-06-11）

评审源：`/autoplan` 手动等价（CEO 战略、Eng 架构/正确性、Design 聚焦 M6）。Eng/CEO/Design 均对照两仓真实代码核实了证据（含 agent_service.rs、deploy-api.yml、ci.yml、gm_desktop、i18n locale 文件）。全文见 `ceo.md` / `eng.md` / `design.md`。

## P1（必须修，路由 Step 07）— 7 项，无 P0

| ID | 一句话 | 修哪 |
|----|--------|------|
| CEO-1/ENG-4 | 上线顺序自相矛盾：M5 要求回填先于校验发布，但 root D2 把 M1–M5 打包一个 PR；且 deploy-api.yml 自动跑迁移，「NC1 人工窗口」叙事不可实现 | root.md D2 + M5 + NC1 ledger + split-map 链式门 |
| CEO-2 | campaign 运行时消费组无平台过滤：agent_service.rs:129-187 从组取回复账号，存量错配 campaign 永久错发——直接违反目标句 | M4 扩任务（R007 等价物） |
| ENG-1 | campaign update 旁路：`platform_id.unwrap_or(existing)` + `social_group_id.or(existing)`——只改平台不带组的请求绕过校验 | M4 Task 4.2 重写（生效值校验）+ IT6h |
| ENG-2 | 错误契约漂移：root C1 用 String 平台名，M2 用 i32 id；纯函数拿不到名字 | root C1 定稿 i32 + 三处映射 arm 说明，M2 同步 |
| ENG-3 | 拆组名冲突路径：NOT EXISTS 跳过 INSERT 后账号重挂无目标 id → 断言炸 → 生产迁移回滚 | M5：先 SELECT 解析目标组 id；IT7 加冲突 fixture |
| DES-1 | E2E1 断言组列表有平台标识，但无任务实现它；且平台徽标是用户理解迁移结果的唯一 UI 面 | M6 加 6.1b（组列表平台列/徽标） |

## P2（应修，并入 Step 07 同批）— 9 项

CEO-3 空组死胡同（M6 选择器显示 account_count+禁选 0 账号组）；CEO-4 M6 桌面端假设错误（gm_desktop createGroup 根本不发 platform_id，已坏——更正记录+决策）；CEO-5 发布窗口污染需 M6 后复检+幂等重跑；ENG-5 PageRequest 9 处字面量构造需补 None（agent_handler/template_service/tool_registry）；ENG-6 :288/:320/:461 是计费冻结非展开，须显式决策；ENG-7 IT7 在 CI 的执行机制（自包含 pytest+事务回滚重放 up.sql）；ENG-8 审计临时表与 down 恢复指引矛盾；DES-2 深链需自动开弹窗+非法参数处理；DES-3 账号弹窗组下拉按平台过滤（否则 R004 后出现无解释 400）。

## P3（顺手修文本）— 8 项

CEO-6 拆组幂等的「已存在组」分支（并入 ENG-3）+发布说明视迁移量；CEO-7 无需改（可加一行未来多组发布注记）；ENG-9 `unwrap_or` 类型修正（并入 ENG-1）；ENG-10 路径 `/api/v1/social-groups`+既有测试文件名（test_social_api.py::TestSocialGroups）+PageRequest.group_id 真实消费者是 template；ENG-11 死代码 `batch_update_group_by_profile_names` 处置；ENG-12 IT5b 统一 404+用户 B 数据 db_cursor 构造说明；DES-4 用 Combobox 惯例措辞；DES-5 复用既有 i18n 键；DES-6/7 可选 polish（tips 一行/编辑弹窗只读徽标，随 DES-1 顺带）。

## 总评

三方一致：根因链真实、service 层卡点策略正确（AI-chat 工具与 HTTP 同漏斗）、拆分合理、无过度工程。无 P0。7 个 P1 全部是计划文本修正（含一个范围扩展 CEO-2），不需要重新设计。

## 评审确认为正确的关键事实（后续步骤可直接引用）

- gm_social_groups.platform_id 有 FK（baseline:3077）。
- workspace 只有 crates/{api,db}，展开逻辑全在 aipub_service（:1483 唯一展开点）。
- UpdatePlanDto 不能改 platform_id/group_id → aipub 建时校验即足。
- update_group 只写 group_name（A008 已成立）。
- 既有组测试在 test_social_api.py::TestSocialGroups；conftest 单用户 seed。
- vitest+Playwright 前端基建存在；selectPlatform* i18n 键中英已存在。
