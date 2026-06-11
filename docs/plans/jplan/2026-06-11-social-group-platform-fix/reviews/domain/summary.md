# Step 06 Domain Review Summary（7 评审员全部完成，2026-06-11）

P0=0。**P1=7**（去重后），路由 Step 07 Patch Batch 2：

| # | 来源 | 一句话 | 修哪 |
|---|------|--------|------|
| B2-P1-1 | FP-F1 + PI-P2-4 | M4 4.3 / M6 6.1b/6.4/FT4b / IT5h 缺 requirement 锚（建 R011/R012、扩 R007） | requirements/assumptions/invariants/root/M4/M6/test-suite |
| B2-P1-2 | Protocol-F1 (+FR-F8) | M6 6.4 错误对象契约错（isApiError/message_cn）+双 toast 决策 | M6 |
| B2-P1-3 | SM-F1 + FR-F3 + FP-F5 | M6 后补救语义+机制重定义（组平台冻结、仅处理不匹配账号；psql --single-transaction 补救脚本+运行簿） | root/NC1/M5/split-map |
| B2-P1-4 | FR-F1 | expand 0-match 走 finalize_plan 退款；IT5g 加 billing 断言 | M3 |
| B2-P1-5 | FR-F2 | 失败原因载体决策=结构化日志（IT5g 断言改写） | M3 |
| B2-P1-6 | TG-P1-1 | IT5h 观测=frozen_cost==2*P（seed 价回读） | M3/anti-gaming |
| B2-P1-7 | Dep-F1 | pytest 不在 CI——门改「本地跑+存档+环境前提」或建 CI job（决策） | production-deps/root 门3 |

P2（同批并入）：Protocol-F2/F3/F4、SM-F2/F3/F6、TG-P2-1..4、Dep-F2/F3/F4、FR-F4/F5/F6、PI-P2-1/2/3、FP-F3/F4。
P3（文本批）：FP-F6/F7、Protocol-F5、SM-F4/F5、TG-P3-1/2、Dep-F5/F6、FR-F7/F9、PI-P3-1..6。
