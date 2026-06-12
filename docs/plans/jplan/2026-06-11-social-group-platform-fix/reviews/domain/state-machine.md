# State-Machine Reviewer（2026-06-11，全写路径 grep 审计）
- F1 P1: M6 后「幂等重跑 up.sql」语义错误——校验上线后组 platform_id 已是权威，重跑多数规则会翻转干净组的平台、打断有效 plan/campaign。补救规则须改为：组平台冻结，仅拆/解绑 platform 不匹配账号
- F2 P2: 重跑无执行机制且手动 psql 默认自动提交→部分应用可持久化。钉死 psql --single-transaction；IT7/演练补手动模式原子性验证
- F3 P2: expand 0-match 失败转移未指定冻结预算去向；两个调用态（create 同步/回调）语义需分开；IT5g 加 billing 断言
- F4 P3: C4 漏 clear_group（NULL writer，INV2-safe）+ FK ON DELETE CASCADE 注记
- F5 P3: ai_chat 工具是全部 writer 的未声明入口（经同 service，已被守卫）；create_social_account 工具 schema 的 group_id 是死参数
- F6 P2: PR-A→PR-B 窗口再脏面比已接受项宽（update_account/批量/ai_chat 绑定零校验）；PR-B 门应= 合并前**新跑**断言；PR-B 部署后即刻复检；窗口 writer 入失败矩阵
通过：platform_id 单写者成立（update 仅名）；account.platform_id 不可变成立；迁移事务性成立；M1 九处清单成立。
