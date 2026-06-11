# Non-Code Verification Exception Ledger — social-group-platform-fix

仅一项真正非代码工作：

| ID | 工作项 | 为何非代码 | 验证替代 |
|----|--------|------------|----------|
| NC1 | 生产库（47.236.115.179/aihub_db）执行回填迁移（ENG-4：deploy-api.yml 自动跑迁移，合并即执行；Step 06 再修订） | 运维流程：合并 PR-A 即触发迁移随部署执行——「合并」就是窗口确认动作 | (1) 合并前：副本演练报告通过并附 PR-A 描述；**快照 owner=PR-A 合并人**，合并前完成（pg_dump -Fc，可 scoped；归档路径写入 PR-A 描述；与演练 dump 同源，Dep-F4/FR-F5）+ 复核 deploy-api.yml 迁移 gate 未变更（B5）；(2) 执行中：迁移事务性，失败即部署中止——**运行簿（FR-F4）**：生产未变，从 job 日志 EXCEPTION 样本诊断→修迁移→推新 commit 重试（幂等安全），禁削弱断言；(3) 执行后：**权威证据=导出查询**（断言 SQL=0 + 拆组/空组/不一致引用三清单存档；job 日志尽力补充）；三清单与活跃 campaign 交叉比对并通知 owner（FR-F9）；(4) **PR-B 合并前置门 = 合并前新鲜断言=0**（SM-F6，非沿用旧档）；PR-B 部署后即刻复检；(5) M6 上线后复跑 INV2 报告，非 0 → 执行**补救脚本**（`psql --single-transaction`，语义=组平台冻结仅拆不匹配账号，见 M5 补救脚本节；不是重跑 up.sql）；(6) committed-but-wrong 恢复首选按审计报告定向回写，全量恢复为最后手段（FR-F5） |

其余所有工作项均为代码，不进本 ledger。
