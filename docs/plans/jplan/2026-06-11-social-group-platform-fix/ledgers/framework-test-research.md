# Framework Test Research Ledger — social-group-platform-fix

无新框架/SDK/队列选型决策——全部沿用仓库既有栈，故无需 `dependency-expert`；官方文档检索仅一处轻量需求（cargo-mutants/proptest 用法），仓库内已有惯例可循则免。

| 技术面 | 选型状态 | 测试方式（既有惯例） | 研究需求 |
|--------|----------|----------------------|----------|
| axum 0.x handler/Query 提取 | 已在用 | `Query<PageRequest>` 加 Optional 字段，serde 缺省即兼容 | 无（仓库内大量同型用法） |
| diesel 查询/迁移 | 已在用 | 条件过滤 `.filter()` 可选拼接；迁移 up/down + schema.rs 同步（db-migration-guard） | 无（参照 2026-06-03 迁移样例） |
| pytest 真 DB 集成测试 | 已在用 | conftest/seed 惯例（test_campaign_api.py） | 无 |
| proptest（属性测试） | **确认缺席，将新增**（Dep-F5：全 workspace 零命中）：`proptest = "1"` 仅入 `crates/api/Cargo.toml [dev-dependencies]` | UT1 三性质 | 已闭：官方标准用法，dev-only 围栏 |
| cargo-mutants（变异门） | 工具非依赖；**本地模块完成门**（Dep-F2 决议：仓库无 mutants CI 配置，不另建 CI job）`cargo install cargo-mutants --locked` | 命令含 `-- -- --lib` 限测试集（Dep-F3） | 已闭 |
| 前端 React/vitest（glance_mind_front） | 已在用 | 组件测试 + 既有 e2e 惯例 | 无（M6 在前端仓库内遵其惯例） |
