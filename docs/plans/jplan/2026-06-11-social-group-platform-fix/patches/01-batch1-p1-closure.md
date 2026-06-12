# Patch Note — Batch 1（Step 07，2026-06-11）

来源：reviews/autoplan/summary.md。P1×7 全闭 + 同爆炸半径 P2×9、P3 文本修正并入。

## 闭合记录

| Finding | 修改文件 | 闭合理由 | 需复核者 |
|---------|----------|----------|----------|
| CEO-1+ENG-4 (P1) | root.md 上线顺序/链式门、M5 依赖行+禁区、non-code-exceptions NC1、03-split-map 链图、requirements D2 | 与 deploy-api.yml 自动迁移事实对齐：两段发布 PR-A(M1+M5)→断言存档→PR-B(M2–M4)；「合并即确认」消除不可实现的人工窗口叙事；回填先于校验的矛盾消除 | Eng |
| CEO-2 (P1) | M4 新增 Task 4.3（agent_service 装配平台过滤+IT6i/j）、requirements D1 兜底说明 | campaign 运行时获得 R007 对称防线，存量错配引用不再永久错发 | CEO/Eng |
| ENG-1+ENG-9 (P1/P3) | M4 Task 4.2 重写（eff_gid/eff_pid 生效值校验+IT6h；unwrap_or 类型修正） | 「只改平台不带组」旁路被显式用例覆盖 | Eng |
| ENG-2 (P1) | root C1 定稿 i32+三 arm、M2 Task 2.1 同步、invariants 文案行 | 契约单一来源恢复；纯函数无名字解析负担 | Eng |
| ENG-3+CEO-6幂等分支 (P1) | M5 拆组 SELECT-then-INSERT+撞名重挂语义、requirements D1、IT7 fixture 加撞名组 | 名冲突路径不再触发断言回滚；重跑收敛 | Eng |
| DES-1 (P1) | M6 新增 Task 6.1b（组表平台列+FT6） | E2E1 断言有了实现载体；迁移结果对用户可见 | Design |
| CEO-3 (P2) | M6 FT4b（account_count 显示+空组禁选） | 空组死胡同在选择时拦截 | CEO |
| CEO-4 (P2) | M6 禁区更正+known-broken 决策、test-suite gap 更正 | 错误前提移除；桌面修复显式 out-of-scope+开票 | CEO |
| CEO-5 (P2) | root 链式门第6条+最终验收项、NC1 第(5)条 | 窗口污染有复检+幂等重跑闭环 | CEO |
| ENG-5 (P2) | M1 实现第5步（9 处字面量+文件清单） | 编译影响面如实入计划 | Eng |
| ENG-6 (P2) | M3 调用点处置改写（:1483 展开 vs 三处计费冻结，采纳方案 a+IT5h） | 计费语义变化显式化并有 owning test | Eng |
| ENG-7 (P2) | M5 Task 5.2 重写（自包含 pytest 事务重放） | IT7 在真实 CI 管线可执行 | Eng |
| ENG-8 (P2) | M5 down.sql 口径（备份恢复，TEMP 表仅事务内） | 矛盾消除，schema.rs 零改动前提保住 | Eng |
| DES-2 (P2) | M6 FT5 三断言细化（自动开弹窗/非法参数/参数消费） | 深链行为完全规约 | Design |
| DES-3 (P2) | M6 新增 Task 6.4（账号弹窗组过滤+错误透传） | R004 上线后的新死胡同消除 | Design |
| ENG-10/11/12, DES-4/5, CEO-7 (P3) | M1 路径/测试文件/消费者注记；M2 Task 2.4 删死代码+root C4；IT5b 钉404+db_cursor 说明（M2/M3/M4/ledger）；M6 Combobox/i18n 措辞；root 未来注记 | 文本与事实对齐 | — |

## 未并入（不阻塞）

DES-6/7（可选 polish，已在 M6 文本中标注「可选」）。

## 状态

P0=0，P1=0（全闭）。下一步：聚焦复核（确认闭合）→ Step 06 域评审。

## 聚焦复核结果（Step 07 内 re-review，独立 agent）

**PASS** —— 7 个 P1 逐项确认 CLOSED；两处推理敏感补丁（M4 4.2 生效值条件、M5 IT7 事务重放机制）经对抗性阅读成立；IT5h 的「Task 3.1 不拦直插脏组」推理被确认正确。

复核新发现 NEW-1～NEW-6（全部 P3 文本陈旧，无一重开 P1）已当场修复：
- NEW-1 invariants「暂停建组入口」残留 → 改为事务性执行+复检兜底
- NEW-2 root/invariants/requirements 的「down 还原结构变更」措辞 → 统一纯 DML/快照恢复口径
- NEW-3 test-suite IT7 fixture 清单补齐（撞名/平票/跨用户）
- NEW-4 IT6i/j 归位为 rust 测试行
- NEW-5 M6 验证命令 FT 范围 FT1–FT7
- NEW-6 校验函数命名统一为 check_group_platform（split-map、test-suite）

裁定：进入 Step 06 域评审。
