# Eng Review（全文存档，2026-06-11，独立评审 agent，逐项对照代码/CI 核实）

通过的关键核查：FK 存在（baseline:3077）；BusinessError 三 arm 映射模式（Display/to_error_code/to_message_cn，BadRequest→400/NotFound→404）；get_group_account_ids 四处调用属实但 :288/:320/:461 为计费冻结、:1483 才是展开；workspace 仅 crates/{api,db}；AI-chat 工具走同 service 漏斗（M2/M3/M4 落点即全覆盖）；UpdatePlanDto 不可改 platform/group；update_group 仅写名；M4 的 D3 担忧属实（分支有未提交 campaign 文件）；前端引用与测试基建属实（test_social_api.py::TestSocialGroups、vitest+Playwright、四个 guard 为真实仓库惯例）。

| ID | Sev | 证据 | 后果 | 补丁 |
|----|-----|------|------|------|
| ENG-1 | P1 | campaign_service.rs:306 `platform_id: dto.platform_id.unwrap_or(existing)`; :320 `.or(existing.social_group_id)`；M4 4.2 只校验 dto.social_group_id Some | 只改平台不带组的 PUT 绕过校验，INV3 violated | M4 4.2 重写：算生效值 eff_gid/eff_pid，`eff_gid.is_some() && (组或平台有变)` 即校验；加 IT6h「只改平台」→400 |
| ENG-2 | P1 | root C1 String 平台名 vs M2 i32 id；纯函数无名可取；invariants 又承诺文案含平台名 | 共享契约分叉，M3/M4 断言互相矛盾 | root C1 定稿 i32 id 变体；要名字则 load_and_check_group 解析（显式多一查）；写明三 arm |
| ENG-3 | P1 | M5 NOT EXISTS 防重但无 (user,name,platform) UNIQUE，撞名跳过 INSERT 后 UPDATE 无目标 id → DO $$ 断言 RAISE → 整体回滚 → 部署迁移 job 失败 | 生产部署失败 | M5：先 SELECT 解析目标组 id，缺再 INSERT，重挂用解析出的 id；IT7 加「目标名已存在」fixture |
| ENG-4 | P1 | deploy-api.yml:43-57 检测 migrations 变更自动 `diesel migration run`（:60-75）且 gate deploy（:84-85） | 合 PR 即自动回填，先于任何人工确认；RAISE NOTICE 报告进 CI 日志非存档 | root 二选一：(a) PR 合并=NC1 确认门，存档 CI 迁移日志+部署后手跑断言与三清单；(b) M5 迁移独立首 PR，经 workflow_dispatch 在窗口执行，再合 M1–M4。同步改 NC1 ledger 与链式门 |
| ENG-5 | P2 | PageRequest 9 处字面量构造无 Default：agent_handler.rs:44、template_service.rs:49、tool_registry.rs:1145,1609,1623,1679,1700,2360,2381 | 加字段即编译炸 9 处；M1 文件清单错 | M1 1.1 实现补第 4 步：9 处补 `platform_id: None`，文件入输出清单 |
| ENG-6 | P2 | :288/:320/:461 是 create_plan 内计费冻结计数 | 「三处统一替换」误导：替换即改计费语义且无测试主张 | M3 3.2 改写：:1483 为展开点；冻结三处显式决策——(a) 同步替换+IT5 断言冻结=匹配数；(b) 留置+一行理由 |
| ENG-7 | P2 | ci.yml:48-88 空库 psql 跑迁移→seed，无中间 hook；seed 组被重放 up.sql 会拆坏（test_social_api.py:666） | IT7「迁移前插 fixture」在标准管线不可实现 | M5 IT7 改自包含 pytest：自插脏 fixture→事务内重放幂等 up.sql→断言→回滚（或 scratch schema） |
| ENG-8 | P2 | TEMP 表事务后消失 vs down.sql 承诺「从审计表恢复」；真实表又违「纯 DML/schema.rs 零改动」 | down 指引不可实现或 guard 冲突 | TEMP 表只用于事务内出报告；down 改「数据迁移不可逆，恢复靠执行前备份/快照」 |
| ENG-9 | P3 | M4 文本 `Option<i32>.or(i32)` 不编译；实际 :306 是 unwrap_or | 文本错误 | 并入 ENG-1 重写 |
| ENG-10 | P3 | 真实路径 /api/v1/social-groups（root.rs:172）；PageRequest.group_id 消费者是 template_service.rs:52 非账号列表 | 文本错误/契约影响面描述错 | M1 修路径、点名 test_social_api.py::TestSocialGroups、修消费者注记 |
| ENG-11 | P3 | social_account_repository.rs:275-292 batch_update_group_by_profile_names 写 group_id 零校验、全仓无调用方（死代码）；其余写路径全覆盖（clear_group 仅置 NULL 安全） | 未来调用方静默绕过 INV2 | M2 禁区：删除或标注 must-validate；入 root C4 |
| ENG-12 | P3 | ledger IT5「400或404」vs M3 404；conftest 单用户 seed | 漂移+用户B数据来源未说明 | ledger 钉 404；M2/M3/M4 RED 节注明用户 B 组经 db_cursor INSERT 构造 |

总评：无 P0；四 P1 均为计划文本修正非重设计；修毕可进入后续步骤。
