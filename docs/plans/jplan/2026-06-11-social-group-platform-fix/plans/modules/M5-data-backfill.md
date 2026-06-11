# Module Plan M5-data-backfill — 存量组平台回填迁移

> 反作弊地板（注入）：迁移断言 SQL 与演练报告同样是「证据」——不得为让断言=0 而放宽断言条件；发现新违规形态只能修迁移逻辑。

Reqs: R008（决策细则=requirements.md D1）。依赖：可与 M1 并行起草/实现。**上线载体（Step 07 P-1 修订）：本迁移随 PR-A（M1+M5）合并，由 deploy 管线自动执行（deploy-api.yml 迁移 gate deploy）——合并 PR-A 即 NC1 窗口确认；M2–M4 校验在 PR-B，前置门=PR-A 生产断言=0 已存档。**（先洗数据再立规矩。）

## Task 5.1 — 迁移脚本

文件：`crates/db/migrations/2026-06-11-000001_social_group_platform_backfill/{up.sql,down.sql}`

up.sql 逻辑（单事务，注释风格对齐 2026-06-03 样例：why + 部署协调 + 安全性说明）：

1. **建审计临时表**（事务内）：记录每组回填前 platform_id、组内账号平台分布。
2. **同平台组回填**：组内全部账号同平台且 ≠ 组平台 → `UPDATE gm_social_groups SET platform_id = <账号平台>`。
3. **混平台组拆组**（D1；Step 07 修订闭 ENG-3/CEO-6）：
   - 多数平台 = 账号数最多的平台；平票 → 组内 `MIN(created_at)` 账号的平台（确定性）。
   - 原组 platform_id ← 多数平台（组 id 不变，存量 plan/campaign 引用保真）。
   - 每个少数平台的目标组 id **先 SELECT 解析、缺才 INSERT**（无 (user,name,platform) UNIQUE 约束，不能依赖 ON CONFLICT）：
     ```sql
     -- 伪代码：target_id := SELECT id FROM gm_social_groups
     --   WHERE user_id=原.user_id AND group_name=原名||'-'||platform_name AND platform_id=少数平台;
     -- 若 NULL → INSERT ... RETURNING id;
     -- UPDATE gm_social_accounts SET group_id = target_id WHERE group_id=原id AND platform_id=少数平台;
     ```
     撞名（目标组已存在，含重跑场景）时账号重挂到**既有组 id**——这是幂等性的关键分支，IT7 必测。
4. **空组**：不动，仅入审计清单。
5. **断言（事务内，违例即 RAISE EXCEPTION 整体回滚；FR-F4 修订——失败必须可诊断）**：

```sql
DO $$
DECLARE sample TEXT;
BEGIN
  SELECT string_agg(format('(g=%s,a=%s,gp=%s,ap=%s)', g.id, a.id, g.platform_id, a.platform_id), '; ')
    INTO sample
  FROM (SELECT * FROM gm_social_accounts a2 JOIN gm_social_groups g2 ON a2.group_id=g2.id
        WHERE a2.platform_id <> g2.platform_id LIMIT 20) AS v(...);  -- 形态示意，实现时按列展开
  IF sample IS NOT NULL THEN
    RAISE EXCEPTION 'INV2 violated after backfill; sample: %', sample;
  END IF;
END $$;
```

（违规样本进 EXCEPTION 消息——RAISE 中止后 NOTICE 报告不再执行，异常消息是 CI 日志里唯一诊断载体。）

约束：**up.sql 不得含显式 BEGIN/COMMIT**（双前提：diesel 的每迁移事务包裹 + IT7 的事务内重放/回滚）。

6. **报告输出**：审计临时表内容以 `RAISE NOTICE` / 导出查询输出（演练与生产执行时重定向存档）：回填计数、拆组清单（原组→新组→重挂账号数）、空组清单、存量 plan/campaign 引用回填后平台不一致清单（`gm_aipub_plans.platform_id <> g.platform_id` / `gm_campaigns` 同型查询——只列不改）。

down.sql（Step 07 修订闭 ENG-8）：纯数据迁移不可自动逆转。down = no-op + 注释「恢复依赖执行前的 DB 备份/快照（NC1 流程要求执行前快照）」。**不承诺从审计表恢复**——审计用 TEMP 表仅存在于事务内，只用于生成报告输出（NOTICE/导出查询），不落永久表（schema.rs 零改动的前提）。

**幂等性**：重跑安全——步骤 2/3 的 WHERE 条件在已一致数据上命中 0 行；拆组目标组经 SELECT-then-INSERT 解析（见上），重跑时命中既有组、重挂 UPDATE 命中 0 行。

## Task 5.2 — 演练与测试（IT7；Step 07 修订闭 ENG-7）

**执行机制**：CI 管线在空库上跑全部迁移后才 seed（ci.yml:48-88），无「迁移前插 fixture」的 hook；且对共享 seed 组重放 up.sql 会污染其他测试。故 IT7 为**自包含 pytest**：

1. 测试自己开事务（或 scratch schema），`db_cursor` 直插脏 fixture：同平台错标组、混平台组（3 fb + 1 reddit）、平票组（1+1）、空组、**目标拆组名已存在的混平台组**（ENG-3 分支）、跨用户同名组。
2. 跑断言 SQL → 违规行 >0（**RED 证据**）。
3. 在事务内重放幂等 up.sql（迁移的幂等设计使重放合法——这正是 CEO-5 复跑场景的预演）。
4. **GREEN 断言**：违规=0；错标组回正；混平台组 id 不变且 platform=facebook；新组 `<name>-reddit` 存在且 reddit 账号已重挂；撞名组的账号挂到既有组 id（无重复 INSERT）；平票组取最早账号平台；空组原值未动。
5. 事务回滚——不污染共享 seed。

**RED 证据机制（TG-P2-3）**：IT7 的 RED 不是测试内前置，而是「实现前失败」记录——提交 IT7 时 up.sql 为空壳（仅注释），执行 IT7，预期失败于步骤 4 `assert violations == 0`（报 N>0，N=fixture 违规行数），失败输出片段存档后才填充迁移逻辑。测试文件名：`crates/api/tests/test_backfill_migration.py`（`-k backfill` 锚点）。

**演练（production-dependencies.md；Dep-F4 机制钉死）**：
1. dump 产出：`pg_dump -Fc` 自生产库（47.236.115.179），凭据持有人=用户/运维（执行人记名）；可 scoped 到 `gm_social_groups`/`gm_social_accounts`/plan/campaign 引用列以缩小 PII 面。
2. 还原到本地 `postgres:15-alpine` 容器（与 ci.yml 同主版本）→ 跑迁移 → 导出报告。
3. **同一份 dump 兼作 NC1 执行前快照**（FR-F5：快照 owner=PR-A 合并人，合并前完成，归档路径写入 PR-A 描述）。
4. 执行前复核既有记录「68/69=reddit」的当前真实分布（演练报告第一节）。
5. 报告权威口径（Dep-F6）：演练与生产的三份清单均来自**执行后导出查询**；NOTICE/管线日志只是尽力补充（ci.yml 迁移输出被重定向，diesel 透传 NOTICE 未验证）。

## 补救脚本（Step 06 SM-F1/FR-F3 新增——M6 上线后 INV2 复检非 0 时的执行物）

- **不是重跑 up.sql**：校验上线后组 platform_id 已是权威，重跑多数规则会翻转干净组平台。
- 语义：组平台冻结；对每个 `platform != 组平台` 的账号执行与 up.sql 拆组相同的 SELECT-then-INSERT 重挂（目标 `<组名>-<platform_name>`），即「少数=一切不匹配者，与数量无关」。
- 载体：独立 SQL 脚本（`scripts/social_group_inv2_remediation.sql`，随 M5 一并交付），**手动 `psql --single-transaction -f` 执行**（SM-F2：手动 psql 默认自动提交，必须显式单事务，否则部分应用可持久化）+ 同款断言与导出报告。
- 运行簿（FR-F4）：migrations job 失败 → 部署被门挡、生产未变；从 job 日志的 EXCEPTION 样本诊断 → 修迁移逻辑 → 推新 commit 重触发（幂等设计保证重试安全）；**禁止削弱断言**。
- IT7 增补：对补救脚本跑一次同型 fixture 验证（混入「干净组+错绑账号」场景，断言组平台未被翻转、错绑账号被拆出）。

## 模块独立验证

```bash
diesel migration run && diesel migration redo   # 本地（redo 验证 down 不破坏）
pytest crates/api/tests/ -k backfill            # IT7
# db-migration-guard：schema.rs 无结构变更则确认无 diff；演练报告人工审阅
```

## 边界与禁区

- 不改 schema 结构（纯 DML）；schema.rs 零改动（若 guard 报 diff 即说明做错了）。
- 不自动修改 plan/campaign 的引用（只报告）；不删除任何组/账号。
- 生产执行 = 合并 PR-A 触发（NC1 修订口径）：合并前演练报告必须附 PR 描述；执行前快照（备份）由 NC1 流程要求；部署后断言与三清单存档不在本模块代码完成口径内，但属 PR-B 的前置门。
