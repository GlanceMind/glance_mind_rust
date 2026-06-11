# Failure/Recovery Reviewer（2026-06-11，对照 aipub 计费/finalize 与 deploy 管线核实）
- F1 P1: expand 0-match 失败必须走 finalize_plan（原子置状态+退款）；裸 update status 永久冻结预算；安全网 check_and_update_plan_completion 只在 total>0 时结算——0 任务 plan 永不结算。IT5g 加 billing_status==settled+余额恢复断言
- F2 P1: 「失败原因」无持久化载体——gm_aipub_plans 无 error_message 列；AI task 是成功完成态。决策：原因只入结构化日志（rust 层断言），IT5g 改断 (a)status failed (b)billing settled (c)0 任务；要用户可见原因则须显式列迁移任务（本计划不做）
- F3 P1: 复跑机制死按钮（与 SM-F1/F2、FP-F5 合并）：钉手动 psql --single-transaction 运行**补救脚本**（非原 up.sql，语义见 SM-F1）+运行簿
- F4 P2: 断言 RAISE 在报告之前→CI 失败无诊断信息——违规行样本进 EXCEPTION 消息或 NOTICE 前移；NC1 加失败运行簿（部署被门挡、prod 未变、修迁移重推、禁削弱断言）
- F5 P2: 快照无 owner/时机/位置——钉：PR-A 合并人在合并前执行 scoped pg_dump（组+账号+引用列），归档路径写入 PR-A 描述；committed-but-wrong 首选按审计报告定向回写，全量恢复为最后手段
- F6 P2: create 时 0 匹配组→不冻结+延迟失败——加 IT5i：create_plan 即 400 "no accounts matching plan platform"+回滚（镜像现有冻结失败回滚 :513-517），expand 过滤退为存量兜底
- F7 P3: M1 InvalidInput 字符串错误不入册——接受（对齐 aipub 先例），不阻塞
- F8 P3: toast 不泄内部（5xx 已脱敏）；指定 msg/msg_cn 按 locale（并入 Protocol-F1）
- F9 P3: campaign 0-match 仅日志可观测——NC1 三清单交叉比对活跃 campaign 并通知 owner（一句话闭环）
通过：deploy 门控正确（迁移失败旧 API 不动）；重试安全；IT7 设计成立；IT6h 关闭成立；日志无 PII。
