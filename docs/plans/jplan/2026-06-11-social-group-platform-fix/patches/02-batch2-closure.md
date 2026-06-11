# Patch Note — Batch 2（Step 07 第二轮，闭 Step 06 域评审发现，2026-06-11）

来源：reviews/domain/summary.md（7 评审员，P1×7 + P2×17 + P3 批）。

## P1 闭合

| Finding | 修改 | 闭合理由 |
|---------|------|----------|
| B2-P1-1（锚缺失） | requirements 增 R011/R012、R007 扩冻结+0匹配语义；assumptions 增 A013–A016 + B5 入 01-first-principles；M4/M6 header 挂锚；invariants INV2 强制点+observability；root 验收 R001–R012；test-suite IT 重挂 | 全部 Step 07 新任务有 requirement 锚，traceability 可编译 |
| B2-P1-2（前端错误契约） | M6 Task 6.4 重写：isApiError + message/message_cn + _silent 单 toast 决策 + FT7b | 对照 client.ts:221-228 实际拒绝对象 |
| B2-P1-3（补救语义+机制） | root 链式门 7 重写；M5 新增「补救脚本」节（组平台冻结、psql --single-transaction、运行簿、IT7 增补）；NC1 (5)(6) | 不再重跑 up.sql 翻转权威平台；死按钮变可执行物 |
| B2-P1-4（退款） | M3 IT5g 断言 (a)failed (b)billing settled+退款（走 finalize_plan）(c)0 任务 | 预算永久冻结路径消除 |
| B2-P1-5（原因载体） | M3 IT5g 载体决策=结构化日志，无 DB 列 | 不可实现断言消除 |
| B2-P1-6（IT5h 观测） | M3：frozen_cost==2*P（seed 价回读，无 image_generations）+ anti-gaming RED 行 | 可观测、防弱断言 |
| B2-P1-7（pytest 不在 CI） | root 门 3 + production-deps 更正（本地门+输出存档+环境前提+d655284 引用修正） | 门可满足 |

## P2/P3 并入（同爆炸半径）

Protocol-F2 单 toast（M6）/F3 IT3c 三态（M1+ledger）/F4 成功码 200+1000（R001/IT2/IT5d/M1/M3）/F5 断言锚+禁新 ErrorCode（root C1）；SM-F2 psql 单事务+IT7 手动模式（M5）/F3 两调用态（M3）/F4 clear_group+CASCADE+ai_chat 注（root C4）/F6 新鲜门+窗口写者（root 门5+invariants）；TG-P2-1 provider 再证据（providers/llm ledger）/P2-2 IT6j 契约+RED（M4）/P2-3 IT7 RED 空壳机制+禁 BEGIN/COMMIT+文件名（M5）/P2-4 FT 表+IT3c（ledger）/P3-1 绿基线豁免清单（anti-gaming）/P3-2 命名批（UT1 序、UT2/IT5f 拆、IT4f/g、-k 串、IT5j）；Dep-F2/F3 mutants 本地门+--lib+--locked（M2/anti-gaming/framework）/F4 dump 机制（M5/production-deps）/F5 proptest="1"（framework）/F6 报告权威=导出查询（M5/NC1/production-deps）；FR-F4 诊断样本进 EXCEPTION+运行簿（M5/NC1）/F5 快照 owner（NC1/M5）/F6 IT5i create 前置拒绝（M3/R007）/F9 清单交叉通知（NC1）；PI-P2-1 UT1 序/P2-2 R005/R006 400+404/P2-3 manifest 三处/P3-3 split-map 三行/P3-4 root C1 hedge/P3-5 production-deps 行刷新；FP-F7 A007 措辞+R002 路径。

UT3 决议（TG-P1-2 选 b）：删行，IT3a/b 真 DB 覆盖两分支。

## 状态

Step 06 全部 P0/P1 闭合，P2 闭合，P3 文本批闭合。下一步：Step 08 traceability compile。
