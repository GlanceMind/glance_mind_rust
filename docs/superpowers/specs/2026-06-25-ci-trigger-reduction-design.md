# CI 触发频率优化设计

- 日期：2026-06-25
- 范围：`.github/workflows/` 三个 workflow 的触发条件
- 目标：在不引入分支保护的前提下，显著减少冗余 CI 运行

## 背景与问题

仓库有三个 workflow：

| Workflow | 改前 push | 改前 pull_request | 干的活 |
|---|---|---|---|
| Rust CI (`ci.yml`) | `main` | → `main` | fmt + clippy + `cargo test --workspace`（带 Postgres + 迁移）|
| Harness API DB (`harness.yml`) | `main` | **任意目标分支** | `cargo test --workspace wallet_billing_harness`（无 DB）|
| Deploy API (`deploy-api.yml`) | `release/**`、`main` + tag（已路径过滤）| — | 迁移 + 构建镜像 + 部署 |

最近 100 次运行分布：Harness-PR 28、Rust CI-PR 23、Rust CI-push 15、Harness-push 15、Deploy-push 15。

四个过度触发根因：

1. **三个 workflow 都没有 `concurrency`**：同一 PR 分支连续 push 会叠加跑满，旧运行不取消（实测 `claude/page-manage-aipub` 单 PR 跑了 4× 以上，运行列表还有成批手动 `cancelled`）。受限 self-hosted runner 上这是最大浪费。
2. **Harness 是 Rust CI 的真子集**（`wallet_billing_harness` ⊂ `--workspace`），却在每个指向 main 的 PR 上和 Rust CI 并排跑；两者并行启动，连"先快后全"的快速失败价值都没有。
3. **合并到 main 后全量重测**：Rust CI-push + Harness-push 重测的是刚从绿色 PR 合并的代码。
4. **Rust CI / Harness 无路径过滤**：改 README、`docs/`、根目录 `*.sql` / `*.py` 也触发完整 Rust 构建+测试。

关键约束：`main` 未开启分支保护、无 required status checks。因此（a）加路径过滤安全（不会出现 required check 因被跳过而永久 pending 卡住 PR）；（b）现有 CI 本就是参考而非强制门禁。

## 决策：Option B —— 去重 + 砍合并后重测

在零风险去重（concurrency + 路径过滤 + Harness 去重）基础上，额外去掉 push-to-main 的全量重测，只靠 PR 阶段验证。

## 具体改动

### 1. Rust CI (`ci.yml`) —— 主门禁，仅 PR 阶段

- **去掉** `push: [main]`（不再合并后全量重测）。
- `pull_request` 加 `paths-ignore`：`**.md`、`docs/**`、`.githooks/**`、`*.sql`、`*.py`。
- 加 `workflow_dispatch`（手动兜底，可随时对 main 跑全量）。
- 加 `concurrency: {group: rust-ci-${{ github.ref }}, cancel-in-progress: true}`。

**路径过滤正确性约束**：必须用顶层 `*.sql` / `*.py`（只匹配根目录的 `verify_instagram_stats.sql`、`test_all_image_models.py`），**绝不能用 `**.sql`** —— 否则 `crates/db/migrations/**/up.sql` 迁移会被跳过，那是绝不能漏测的。混合改动（同时改 `.md` 和 `.rs`）仍会触发，因为 `paths-ignore` 只在"全部改动都命中忽略列表"时才跳过。

### 2. Harness API DB (`harness.yml`) —— 只补 Rust CI 不覆盖的场景

- **去掉** `push: [main]` 和对 main 的 PR 触发。
- `pull_request` 改为 `branches-ignore: [main]` + 同款 `paths-ignore`：只在非 main 目标的 PR（feature 栈、release 分支）上跑这个便宜子集。
- 保留 `workflow_dispatch`。
- 加 `concurrency: {group: harness-${{ github.ref }}, cancel-in-progress: true}`。

### 3. Deploy API (`deploy-api.yml`) —— 触发不动，仅串行化

- 触发条件保持原样（已路径过滤、目的明确）。
- 加 `concurrency: {group: deploy-api, cancel-in-progress: false}`：多次部署排队而非并发竞争；`false` 保证不会取消进行中的部署。

## 收益与代价

**收益**（按最近 100 次运行估算）：砍掉 push-main 重测（~30 次）+ Harness 与 Rust CI 去重（Harness-PR 28 → ~5）+ concurrency 收敛活跃分支叠加运行 + 路径过滤跳过文档类改动。约 100 次 → 约 35–45 次，做同样的事。

**代价（明确换掉的安全网）**：合并后不再自动重测 main，残余风险收窄到一个很窄的窗口 —— 两个各自绿的 PR 语义冲突、合并后"能编译但测试挂"。因为：

- 编译级破坏仍被 **Deploy 的 `cargo build --release -p glance_mind_api`（`Dockerfile.api:54`）兜住** —— main 编译不过则部署失败。
- "能编译但测试红"的情况会在**下一个 PR 的 CI**（从已坏 main 切出）暴露，而非即时暴露。
- main 本就无分支保护、无 merge queue（陈旧 PR 本可合并），此风险已部分存在；Option B 只是不再用"合并后重测"补它。

## 验证

- `python3` + PyYAML 解析三文件无异常；`yamllint`（GH-Actions 规则配置）clean。
- 触发结构与 concurrency 经解析确认与本设计一致。
- 本 PR 本身就是 `pull_request → main`，会用新版 `ci.yml`（PR 事件取合并提交的 workflow）实跑一次 Rust CI，验证新配置在真实流水线下可用。

## 回滚

`git revert` 本 PR 即可恢复原触发条件；workflow 改动无数据/状态副作用。Deploy 的 `concurrency` 如不需要可单独删除该 3 行。
