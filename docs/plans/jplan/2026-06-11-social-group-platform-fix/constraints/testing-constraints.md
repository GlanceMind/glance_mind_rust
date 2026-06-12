# Testing Constraints — 摘要（源：~/.claude/TESTING_CONSTRAINTS.md + ~/.claude/CLAUDE.md，已于 Step 00 加载）

本约束注入本 plan family 的每个 root/module plan，对所有实现者（人/agent/子 agent）生效。

## 硬规则（不可违背）

1. **禁止为过测试而改弱/改写/注释/删除已有断言**。测试失败只能：(a) 修生产代码（首选）；(b) 断言编码了错误期望 → 停下说明、改正、重走 RED→GREEN，改动带 `ASSERTION-CHANGE-JUSTIFIED: <原因>`；(c) 测试确已废弃 → 整条删除并说明。
2. **禁止 skip 绕过**：不新增 `skip/xfail`、`#[ignore]`、`unittest.skip`、`.skip(`、`@Disabled` 等。
3. **禁止伪造通过**：不 `sys.exit(0)`、不吞异常、不 `assert True`、不把 expected 改成 actual、生产代码不特判测试输入。
4. **先红后绿**：每个测试先为「正确的原因」失败（保留预期失败信息），再变绿；留 RED→GREEN 证据。
5. **如实报告**：断言未全部显式验证不得标记 passed。
6. **职责分离**：写测试的上下文 ≠ 改生产代码的上下文；实现者默认不得改测试断言。subdriven 执行时测试任务与实现任务分属不同子 agent。
7. **断言改动高危**：任何断言改动要书面理由，单独 commit、单独 review。

## 对本 plan family 的具体落地

- **Rust 单元/集成测试**：组平台校验、绑定校验、plan/campaign 校验各自先写 RED 测试（错误平台组 → 预期 400/BusinessError），再实现。
- **Python API 集成测试**（`crates/api/tests/` 惯例）：DB-backed 测试遵循 `db-field-validation-guard`；CI 先跑迁移 + seed。
- **数据回填迁移**：迁移本身要有可验证断言（回填后无 NULL/不一致行的 SQL 检查），down.sql 可回滚；混平台组的处理结果要可审计。
- **变异/属性测试**：组平台一致性校验属核心逻辑，模块计划中为校验函数安排属性测试（任意 (账号平台, 组平台) 组合：不一致必拒绝、一致必放行）。
- **前端**：建组弹窗平台选择器走现有前端测试惯例；不得删除/绕过现有 e2e 断言。

## 强制层级提示

本地 hook（assertion_guard）只是减速带；真·强制 = CI required checks + PR diff 审查（生产代码与断言同 PR 改动须人工解释）。
