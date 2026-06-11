# Production Dependency Test Ledger — social-group-platform-fix

本任务唯一真实生产外部依赖：**PostgreSQL**（含生产实例 47.236.115.179/aihub_db 的存量数据形态）。无新增第三方 API/SDK/队列。

| 维度 | 内容 |
|------|------|
| 真实依赖路径 | diesel → Postgres（本地/CI 容器 = 与生产同主版本；迁移在真实 PG 上执行） |
| 真实测试 | IT1–IT7（`crates/api/tests/`，pytest 打真 API + 真 DB）；IT7=`test_backfill_migration.py` 专测迁移回填 |
| 命令 | **更正（Dep-F1）：CI 不跑 pytest**（无任何 workflow 跑它；ci.yml 迁移+seed 后只跑 cargo test；d655284 实为 rust 测试口径）。pytest 为**本地门**：前提=运行中 API（API_BASE_URL，默认 :8081）+ 测试 DB + NATS（run_tests.sh 口径）；命令 `pytest crates/api/tests/ -k "social_group or social_account or aipub or campaign or backfill"`；全量绿输出存档为合并证据。rust 侧 `cargo test` 在 CI 真 DB 上跑 |
| 凭据/env 门 | CI 已具备 DATABASE_URL（cargo test 用）；pytest 本地 env 见上；生产回填凭据=运维执行步骤（见 non-code-exceptions） |
| fixtures | IT7 自包含 pytest 事务内直插（ENG-7）：同平台错标/混平台/平票/空组/**撞名目标组**/跨用户同名组；测试自回滚不污染 seed |
| 幂等性 | 回填 up.sql 设计为可重入安全（重跑断言部分仍=0 违规）；测试 fixture 每次重建 |
| 成本/限流 | 无（自有 DB） |
| 清理 | CI 容器即弃；本地测试库 teardown 沿用现有 conftest 惯例 |
| 生产数据演练 | 机制钉死（Dep-F4）：`pg_dump -Fc` 自生产（凭据持有人=用户/运维，执行人记名；可 scoped 缩 PII 面）→ 本地 `postgres:15-alpine` 还原 → 跑迁移 → **导出查询**产出报告（权威口径，NOTICE/管线日志仅尽力）；同份 dump 兼作 NC1 执行前快照（owner=PR-A 合并人） |
| 通过证据要求 | 模块 M5 完成时附：副本演练报告 + 断言 SQL 输出 + `diesel migration redo` 验证记录（down=no-op 不破坏）+ IT7 RED（空壳）→GREEN 输出 |
