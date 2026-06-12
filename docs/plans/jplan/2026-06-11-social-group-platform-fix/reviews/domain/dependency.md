# Dependency Reviewer（2026-06-11，对照 workflows/Cargo/conftest 核实）
- F1 P1: production-deps ledger 谎称 CI 跑 pytest——无任何 workflow 跑 pytest（ci.yml 只 cargo test；d655284 是 rust 测试）；pytest 还需运行中 API（:8081）+ 测试 DB + NATS。root 门 3「pytest 全绿（CI）」不可满足。选 (a) 本地跑+存档输出+写明环境前提，或 (b) 加 CI pytest job（须有 owning task）
- F2 P2: cargo-mutants 无 CI 配置无 owning task 无版本钉——降级为「模块完成本地门+输出存档」，M2 验证块加 cargo install cargo-mutants --locked --version 钉
- F3 P2: -f 只限变异对象不限测试集——整包测试含 DB-backed rust 集成测试。改 `-- -- --lib` 限测或写明 DATABASE_URL 前提
- F4 P2: 生产演练 dump 无产出机制——钉 pg_dump -Fc（凭据持有人=用户/运维）→ postgres:15-alpine 本地还原；同份 dump 兼作 NC1 快照；注意 PII
- F5 P3: proptest 确认缺席将新增，钉版本 proptest = "1"（dev-only 已围栏）
- F6 P3: ci.yml 迁移输出重定向 /dev/null；NOTICE 存档过度承诺——权威报告=执行后导出查询，NOTICE 为尽力补充
通过：provider/LLM 不适用实质成立（aipub_service 无 provider 调用，grep 零命中）；PG 版本一致；IT7 与 conftest 回滚惯例匹配。
