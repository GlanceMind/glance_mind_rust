# Handoff: Step 04 (all drafts) → Step 05 (autoplan review)

## Drafts Complete

- `plans/root.md`（共享契约 C1–C4、模块序、链式门、上线序、风险回退）
- `plans/modules/M1-group-crud.md`（R001–R003）
- `plans/modules/M2-binding-invariant.md`（R004 + C1/C2）
- `plans/modules/M3-aipub-guard.md`（R005,R007）
- `plans/modules/M4-campaign-guard.md`（R006）
- `plans/modules/M5-data-backfill.md`（R008）
- `plans/modules/M6-frontend.md`（R009,R010；glance_mind_front 仓库）

每任务含：RED 测试、预期 RED 失败形态、实现要点（精确文件:行）、GREEN 命令、验收门、反作弊注入、边界禁区。

## Next Step

`05-autoplan-review`：对 root+modules 跑 CEO/Eng/Design 等价评审（按 step-05 reference 的流程）。

## Required Reads (only these)

1. `00-manifest.md`
2. `plans/root.md`
3. `plans/modules/*.md`（7 份全读——评审对象）
4. `03-split-map.md`
5. `ledgers/requirements.md` + `ledgers/invariants-failures.md`（评审基准）
6. `~/.claude/skills/jplan/references/step-05-autoplan-review.md`

## Avoid Reading

- 源代码（评审以 plan 与 ledger 为对象；plan 中文件:行引用已自带证据）。
- 其余 ledgers（评审员按需取用）。

## Validation Evidence (Step 04)

- 7 份 plan 无占位符；R001–R010 每条都有 owning task + RED/GREEN 双命令。
- 确定性门不依赖线上凭据；唯一 live 面（生产回填）在 NC1 控制下（窗口/回滚/存档）。
- 每模块有独立验证命令；root 定模块序与最终链式门。
- 状态键单写者表在 root C4。
