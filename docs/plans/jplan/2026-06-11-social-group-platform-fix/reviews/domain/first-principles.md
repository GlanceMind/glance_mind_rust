# First-Principles Reviewer（2026-06-11，独立 agent，全前提对码核实）
- F1 P1: M4 Task 4.3 无 owning requirement（建 R011，配 A-row DERIVED←B3,B4；invariants INV2 强制点+observability 收录；IT6i/j 改挂 R011）
- F2 P1: 计费冻结=匹配数是计费语义变更，无 R 行（扩 R007 文本含冻结计数 + A-row DERIVED，记被拒分支 freeze=total）
- F3 P2: deploy 管线自动迁移是 load-bearing 事实但未分类（加 B5 bedrock；D2=DERIVED←B5,B1；NC1 加合并前复核管线未变更）
- F4 P2: M6 6.1b/6.4 无 requirement 链接（建 R012 或扩 R009/R004）
- F5 P2: 幂等重跑无生产执行载体（diesel 不重跑已应用迁移）→ 指定 psql --single-transaction 手跑或后续迁移 PR
- F6 P3: C2 参数顺序漂移（test-suite UT1 行）
- F7 P3: A007 措辞过时 + R002 路径 /api/v1
通过：目标句/分解/分类/重构/门适用性/反作弊地板完整性全过；M4 4.2 与 M5 幂等逻辑对抗性核验成立。
