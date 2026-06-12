# Test-Gate Reviewer（2026-06-11，对照 aipub 计费与测试基建核实）
- P1-1: IT5h「冻结 3→2」无可观测字段——无 count 字段，须钉 frozen_cost==2*P（P=seed 单价回读，不写死；无 image_generations fixture），镜像入 anti-gaming RED 表
- P1-2: UT3 仅在 ledger、无模块拥有——加给 M1 或删行注明由 IT3a/b 覆盖（当下决定）
- P2-1: providers/llm INAPPLICABLE 证据过时（M4 4.3 触 agent_service）——实质核验仍成立（仅 agent_repo 查询+纯函数限额，LLM 上游），补一行再证据；IT6i/j 以 redis=None 跑
- P2-2: IT6j 无 RED 形态且与现行 keep-comment-on-empty 回退矛盾——钉死「0 匹配⇒排除 comment+warn（有意偏离 legacy）；真空组（total==0）保持 legacy 直通并回归锁」+ RED 形态
- P2-3: IT7 的 RED 是测试内前置非「实现前失败」——补空壳 up.sql RED 记录；up.sql 禁显式 BEGIN/COMMIT；命名 test_backfill_migration.py
- P2-4: FT1–FT7 不在 ledger；serde edge 行无 owner——补 FT 表；edge 给 IT3c 归 M1
- P3-1: 绿基线锁（IT2/IT3b/IT4e/IT5d/IT6c/IT6f/FT4）需登记豁免清单并「实现前跑一次留绿证据」
- P3-2: UT2/IT5f 拆分、IT4 行补 f/g、-k 过滤串补 social_account/backfill、UT1 签名顺序、空组回归给 ID
通过：变异门/属性测试/反作弊注入/真 DB 覆盖/确定性门无凭据 全过。
