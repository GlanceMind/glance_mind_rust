# Manifest — social-group-platform-fix

## Request Summary

用户请求（2026-06-11）：基于已完成的只读分析，制定完整修复计划，使「社媒账户组」可正常使用。
方向已定：**账户组保持平台维度（platform-scoped）**；社媒任务（campaign）与发布工作流（aipub plan）创建/执行时都必须正确适配「组的平台」这一条件。

分析结论（根因链，已验证，详见 00-context-inventory.md）：

1. `gm_social_groups.platform_id` 非空必填，但 Web 端建组弹窗（glance_mind_front `Accounts.tsx`）**没有平台选择器**，全部组以硬编码默认 `platformId: 1`（Reddit）创建 → 生产库 68/69 组 platform_id=reddit（垃圾数据）。
2. 后端组列表接口 `PageRequest` 无 `platform_id` 字段，前端传的过滤参数被 serde 静默丢弃；真实过滤只靠前端客户端 filter。
3. 账号绑定组（`social_account_service.rs`）不校验账号平台与组平台一致。
4. aipub plan / campaign 创建时不校验 `group_id` 的组平台与任务 `platform_id` 一致（甚至不校验组归属）。
5. 结果：Facebook 发帖任务按 platform_id=3 过滤组 → 永远为空 → 无法选组；而 Facebook post/story 又强制走组目标 → 发布流程卡死。

## Plan Family Status

- status: **`complete`**（plan family 就绪，待实现）
- current_step: `09-handoff` (completed)
- next_step: 无——进入实现阶段（读 `handoff.md` 开工；建议 subdriven 执行，测试/实现分子 agent）

## Completed Steps

| Step | Artifact(s) | Date |
|---|---|---|
| 00-init | 00-manifest.md, 00-context-inventory.md, constraints/testing-constraints.md, handoffs/00-to-01.md | 2026-06-11 |
| 01-first-principles | 01-first-principles.md, ledgers/assumptions.md, handoffs/01-to-02.md | 2026-06-11 |
| 02-ledgers | ledgers/{requirements,invariants-failures,test-suite,anti-gaming-test-quality,production-dependencies,providers,llm-api-boundary,framework-test-research,non-code-exceptions}.md, handoffs/02-to-03.md | 2026-06-11 |
| 03-split | 03-split-map.md, handoffs/03-to-04.md | 2026-06-11 |
| 04-draft | plans/root.md, plans/modules/M{1-group-crud,2-binding-invariant,3-aipub-guard,4-campaign-guard,5-data-backfill,6-frontend}.md, handoffs/04-all-to-05.md | 2026-06-11 |
| 05-autoplan-review | reviews/autoplan/{ceo,eng,design,summary}.md, handoffs/05-to-07.md | 2026-06-11 |
| 07-patch-iterate (Batch 1) | patches/01-batch1-p1-closure.md, 受补丁 plans/ledgers ×11, handoffs/07-to-06.md | 2026-06-11 |
| 06-domain-review (×7) | reviews/domain/{first-principles,protocol,state-machine,test-gate,dependency,failure-recovery,plan-integrator,summary}.md | 2026-06-11 |
| 07-patch-iterate (Batch 2) | patches/02-batch2-closure.md, 受补丁 plans/ledgers/manifest ×15 | 2026-06-11 |
| 08-traceability | compile/traceability-compile.md（首轮 FAIL×3 → 修复 → PASS）, handoffs/08-to-09.md | 2026-06-11 |
| 09-handoff | handoff.md | 2026-06-11 |

## Artifact Map

- `00-manifest.md` — 本文件（路由真源）
- `00-context-inventory.md` — 代码上下文清单（路径 + 相关性）
- `constraints/testing-constraints.md` — 反测试作弊约束摘要（来自 ~/.claude/TESTING_CONSTRAINTS.md）
- `handoffs/00-to-01.md` — Step 01 输入说明
- `ledgers/` — (Step 02 写入)
- `plans/root.md`, `plans/modules/` — (Step 04 写入)
- `reviews/`, `patches/`, `compile/` — (Step 05–08 写入)

## Module Queue（全部已起草并两轮评审修订）

1. root（plans/root.md）— 共享契约 C1–C4 + 链式门 + 两段发布
2. M1-group-crud — R001/R002/R003
3. M2-binding-invariant — R004 + 共享校验函数/错误枚举
4. M3-aipub-guard — R005/R007（含计费冻结、finalize 退款、create 前置拒绝）
5. M4-campaign-guard — R006/R011（前置：核实分支未提交 campaign 改动，D3）
6. M5-data-backfill — R008（PR-A 载体）+ 补救脚本
7. M6-frontend — R009/R010/R012（glance_mind_front 仓库）

详见 `03-split-map.md`。

## Reviewer Queue

- [x] Step 05 CEO/Eng/Design（manual equivalent）— reviews/autoplan/
- [x] Step 06 七评审员（first-principles/protocol/state-machine/test-gate/dependency/failure-recovery/plan-integrator）— reviews/domain/
- [x] Step 08 Traceability Compiler — FAIL×3 修复后 PASS（compile/traceability-compile.md）

## Patch Queue

- [x] Patch Batch 1（P-1…P-8）：P1×7 + P2×9 + P3 文本批 — 完成，复核 PASS（patches/01-batch1-p1-closure.md）；NEW-1~6 文本陈旧项当场修复
- 延后（P3，不阻塞）：DES-6/7 可选 polish（随 M6 实现时顺带决定）

## Files Required For Next Invocation (Step 08 traceability compile)

1. `00-manifest.md`
2. `plans/root.md` + `plans/modules/*.md`
3. `ledgers/*.md`（全部 10 份）
4. `01-first-principles.md`
5. `~/.claude/skills/jplan/references/step-08-traceability.md` + `references/reviewer-prompts.md`（Traceability Compiler 节）
6. `patches/02-batch2-closure.md`

## Open Blockers / 决议状态（Step 07 Batch 2 后刷新）

- 无 open P0/P1。
- A1：已决（D1 拆组细则，requirements.md）。
- A2：仍然成立——M6 在 `glance_mind_front` 独立 PR 交付。
- A3：**已被 NC1 修订取代**（合并 PR-A 即窗口确认，无独立人工窗口）；存留的运维项=合并前快照就绪（owner=PR-A 合并人）+ pg_dump 凭据持有人确认（Dep-F4）。
