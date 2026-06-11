# Requirement Ledger — social-group-platform-fix

Origin 取值：`user`（用户明示）/ `derived`（由 bedrock 推导，引 assumptions.md）。无 `assumed` 遗留。

| ID | Requirement | Source | Decision | Owner(module) | Acceptance Evidence | Origin | FP justification |
|----|-------------|--------|----------|---------------|---------------------|--------|------------------|
| R001 | 建组请求的 `platform_id` 必须存在于 `gm_platforms`，否则 400 | S1 | create_group 服务层校验（查 gm_platforms） | M1 | 集成测试：非法 platform_id→400「Invalid platform_id」；合法→**200+code==1000**（api_ok! 恒 200，Protocol-F4）且落库值一致 | derived | A002←B1,B2 |
| R002 | `GET /api/v1/social-groups` 支持可选 `platform_id` 查询参数，服务端过滤；缺省行为不变 | S2 | `PageRequest` 加 `platform_id: Option<i32>`，repo 条件过滤（A012：不建新端点） | M1 | 集成测试：带参只返回该平台组（total 同步正确）；不带参返回全部（回归）；edge IT3c（0/负→200 空列表；非数字→400 axum 纯文本，框架行为） | derived | A003←B1,B4 |
| R003 | 组列表/详情响应继续携带 `platform_id`，现有响应 shape 不破坏 | S2 | DTO 不变，过 `api-contract-guard` | M1 | contract guard 通过 + 既有列表测试回归绿 | derived | A003；api-contract-guard 仓库规则 |
| R004 | 账号创建/更新绑定 `group_id` 时：组必须存在、属于该用户、组平台==账号平台，否则 400/404 | S3 | social_account_service create/update 两路径统一走校验函数 | M2 | 集成测试 IT4a–g（含批量创建原子性）：不存在组/他人组/平台不一致→拒绝；一致→成功 | derived | A004←B2,B3,B4 |
| R005 | aipub `create_plan` 携带 `group_id` 时：组属于用户且组平台==`dto.platform_id`。平台不一致→400(GroupPlatformMismatch)；不存在/他人组→404(GroupNotFound，防枚举) | S4 | create_plan 校验块新增（紧邻现有 plan_type 校验） | M3 | IT5a 400 / IT5b 他人组 404 / IT5c 不存在 404 / IT5d 成功 200+code==1000 | derived | A005←B3,B4 |
| R006 | campaign create/update 携带 `social_group_id` 时：组属于用户且组平台==campaign 生效 `platform_id`。平台不一致→400；不存在/他人组→404 | S4 | campaign_service create/update **生效值**校验（ENG-1） | M4 | IT6a–IT6h（含只改平台旁路 IT6h） | derived | A005←B3,B4 |
| R007 | aipub 计划**展开与计费冻结计数**均只计 `账号.platform_id == plan.platform_id` 的账号；跳过记 warning；create 时 0 匹配→400 拒绝并回滚（FR-F6）；expand 时 0 匹配（存量兜底）→finalize_plan(failed) 原子退款 | S4 | expand+冻结三处过滤（ENG-6 方案 a）+ create 前置拒绝 + finalize 失败路径（FR-F1） | M3 | UT2、IT5f/g/h/i；IT5g 含 billing settled 断言 | derived | B3 + 冻结额度=可展开任务数守恒（A014） |
| R008 | 存量 `gm_social_groups.platform_id` 回填：按组内账号真实平台回填；混平台组拆组（详见 D1 决策）；迁移后一致性断言=0 违规行 | S5 | diesel 迁移（up 含回填+断言；纯 DML，down=no-op，恢复靠执行前快照）+ 迁移报告 | M5 | 库副本演练记录：回填前后计数、拆组清单、断言 SQL 输出 0 | derived | A006,A007←B1,B2 |
| R009 | Web 建组弹窗必须有平台选择器（必选，无默认偏向），提交真实选择的 `platform_id` | S6 | Accounts.tsx 弹窗加 Select（复用平台列表数据源）；移除 `platformId: 1` 默认 | M6 | 前端测试：未选平台不可提交；选 facebook 提交 payload platform_id=3 | user | A002；用户可感知的根因修复 |
| R010 | 发布任务表单的组选择继续按平台过滤（服务端参数 R002 生效后客户端 filter 保留为兜底） | S6 | AIPubPlanCreate fetchGroups 参数已在传，后端生效即可；filter 不删 | M6 | 前端回归：facebook 任务只显示 facebook 组 | derived | A003,A009 |
| R011 | campaign 回复账号装配只装配 `账号.platform_id == campaign.platform_id` 的账号；跳过记 warn（campaign_id, group_id, skipped_account_id）；0 匹配则该 campaign 不装配且不影响其他 campaign | S4 | agent_service 装配处 SQL 级过滤（M4 Task 4.3，R007 的 campaign 对称物，CEO-2） | M4 | IT6i/IT6j（rust 测试，redis=None） | derived | A015←B3,B4 |
| R012 | 前端平台模型可见与防呆：组列表展示平台列（FT6）；账号弹窗组下拉按所选平台过滤+错误信息透传（FT7）；发布表单组选择器显示 account_count 且空组禁选（FT4b） | S6 | M6 Task 6.1b/6.4/FT4b | M6 | FT4b/FT6/FT7/FT7b + E2E1 平台标识断言 | derived | A016←B1,B4（DES-1/3、CEO-3） |

## 已落锤决策（来自 01-to-02 handoff 的 D1–D3）

- **D1 回填细则**：组内账号按平台分布回填。(a) 全部同平台→直接回填该平台；(b) 混平台→组 id 保留**多数平台**（平票取组内最早创建账号的平台，确定性 tie-break），少数平台账号迁入组 `<原组名>-<platform_name>`（同 user；目标组 id **先 SELECT 解析、缺才 INSERT**——撞名/重跑时重挂到既有组 id，ENG-3）；(c) **空组**→保留原 platform_id 不动（无账号则不违反不变量），列入迁移报告供人工复核。存量 plan/campaign 引用的 group_id 不变（多数账号留在原组，引用语义最大保真）；回填后组平台与存量 plan.platform_id 不一致的引用列入迁移报告，**不自动改 plan**——其运行时危害由 R007（aipub expand 过滤）与 M4 Task 4.3（campaign 装配过滤，CEO-2）双向兜底。
- **D2 上线顺序**（Step 07 重写闭 CEO-1/ENG-4）：两段后端发布——**PR-A = M1+M5**（迁移随 deploy 管线自动执行=NC1，合并即确认，演练报告先附 PR）→ 生产断言=0 存档 → **PR-B = M2+M3+M4** 校验 → **前端 PR = M6**（与 PR-B 同日或尽快，压缩污染窗口）。M6 后复跑 INV2 报告，非 0 幂等重跑回填。无需 feature flag。
- **D3 campaign 落点**：校验加在 `campaign_service` create/update 路径。本分支存在未提交的 campaign 文件改动（属其他任务）；M4 实现前先 rebase/确认这些改动已落地，不与之混编。
