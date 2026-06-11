# Module Plan M3-aipub-guard — 发布计划组校验 + 展开平台过滤

> 反作弊地板（注入）：不得改弱/删除断言、不得 skip/xfail/ignore、不得伪造通过；失败修生产代码；错误期望须 `ASSERTION-CHANGE-JUSTIFIED` 重走 RED→GREEN。测试与实现分上下文。aipub 既有测试断言只读。

Reqs: R005, R007。依赖：M2 合入（复用 `load_and_check_group` + `GroupPlatformMismatch`）。

## Task 3.1 — create_plan 组归属+平台校验（R005）

现状：`aipub_service.rs::create_plan`（:60-235）已校验 plan 的 `platform_id` 合法性与 plan_type↔group_id/social_account_id 互斥，但对 `group_id` 只字未查（不存在/他人/错平台组均可建 plan）——还隐含 IDOR 面（invariants 安全边界节）。

**RED 测试（pytest IT5，aipub 测试文件沿用既有命名惯例）**

seed：用户 A（facebook 组 gF、reddit 组 gR）、用户 B（facebook 组 gB）。

- IT5a：batch_text plan `platform_id=3(facebook), group_id=gR` → 400 GroupPlatformMismatch。
  - 预期 RED：当前 200+code==1000 创建成功（api_ok! 恒 200）→ 断言 400 失败。
- IT5b：`group_id=gB`（他人组）→ **404** GroupNotFound（钉死，ENG-12；ledger 同步）。用户 B 的组经 `db_cursor` 直插构造（conftest 单用户 seed）。预期 RED：当前 200+code==1000 创建成功。
- IT5c：`group_id=不存在` → 404。预期 RED：当前 200+code==1000 创建成功（或后续展开期才炸）。
- IT5d：`group_id=gF` → 成功 = **HTTP 200 + body.code==1000**（api_ok! 恒 200，无 201，Protocol-F4）。
- IT5e（reddit plan_type 路径）：`reddit_text` + `platform_id=1` + `group_id=gF` → 400（reddit 计划配 facebook 组同样被拦——校验对所有带 group_id 的 plan_type 生效，不只 batch_text）。预期 RED：当前 200+code==1000 创建成功 → 断言 400 失败。

**实现**

在 plan_type 互斥校验块之后、`NewAipubPlan` 构造之前加统一段：

```rust
if let Some(gid) = dto.group_id {
    let _group = load_and_check_group(&self.group_repo, gid, user_id, dto.platform_id).await?;
}
```

对所有 plan_type 一视同仁（batch_text/account_grooming/reddit_* 均要求 group_id，single_video/direct_publish 此处为 None 自动跳过）。aipub service 注入 `SocialGroupRepository`（或经 aipub_repository 暴露等价查询——以 M2 装配函数的依赖形态为准，避免双实现）。

**GREEN**：`pytest crates/api/tests/ -k aipub`；aipub 既有用例全量绿（若既有用例 seed 的组平台与 plan 不一致而转红：这是「测试数据违反新不变量」，修 seed 数据使其一致——seed 非断言，不属断言改动；若有断言显式断定「错平台组可建 plan」，触发 ASSERTION-CHANGE-JUSTIFIED 流程）。

## Task 3.2 — expand 平台过滤（R007，防御纵深兜存量脏数据）

现状：`expand`（:1459-1490）`get_group_account_ids(gid)` 返回组内全部账号，混平台账号原样展开。

**RED 测试（rust 单测 UT2 + pytest 集成）**

- UT2（rust 单测）：展开过滤逻辑——混平台账号集只产出匹配账号；warn 日志字段（plan_id/group_id/skipped account_id）断言在 rust 层。
- IT5f（pytest）：组 gMix 含 facebook 账号×2 + reddit 账号×1（直插 DB 构造存量脏数据，绕过 M2 校验），facebook plan 展开 → 断言仅产生 2 个任务且全部对应 facebook 账号。
  - 预期 RED：当前产生 3 个任务 → `assert 2` 失败。
- IT5g（expand 0-match，存量兜底路径；Step 06 FR-F1/F2 修订）：组内 0 个匹配账号（直插全 reddit 账号组 + 组 platform=facebook 的脏数据 + facebook plan，经 AI 回调路径到达 expand）→ 断言三件事：
  (a) plan status == failed；(b) **billing_status == "settled" 且冻结预算已退（余额恢复）**——实现必须走 `finalize_plan`（原子置状态+退款，同 fail_ai_task :1276-1280 模式），裸 update status 会永久冻结用户预算（check_and_update_plan_completion 只在 total>0 时结算，0 任务 plan 永不触发）；(c) 0 个 publish task。
  - **失败原因载体决策（FR-F2）**：`gm_aipub_plans` 无 error_message 列，本计划不加列——原因只入结构化 `error!` 日志（"no accounts matching plan platform"，rust 层断言），不做用户可见字段。若未来需要用户可见原因，须显式列迁移任务。
  - 预期 RED：当前产生 1 个错平台任务（且无失败路径）。
- IT5i（create 时 0 匹配前置拒绝，FR-F6）：直插脏组（组平台==plan 平台但组内账号全部其他平台），`create_plan` → **400** "no accounts matching plan platform"，plan 已回滚不存在（镜像现有冻结失败回滚 :513-517 模式）。expand 时过滤由此退化为纯存量兜底。
  - 预期 RED：当前 create 成功、plan 进入 pending。
- IT5j（回归固化）：真空组（0 账号）行为与现状一致——先跑现状记录行为再写断言（绿基线锁，登记于 anti-gaming ledger 豁免清单）。

**实现**

`aipub_repository.rs` 新增 `get_group_account_ids_for_platform(gid, platform_id) -> Vec<i32>`（SQL 级过滤），同时返回/另查组内总数用于差值日志。

调用点处置（Step 07 修订闭 ENG-6——评审核实四处调用职责不同）：

- **`:1483`（`expand_plan_to_tasks` 内）= 唯一展开点**：替换为平台过滤版；`skipped = total - matched`，`skipped > 0` 时 `warn!(plan_id, group_id, skipped, plan_platform)`；`matched == 0 && total > 0` 时按 IT5g 失败语义处理。
- **`:288 / :320 / :461`（create_plan 内计费冻结计数）**：**同步替换**为平台过滤版（决策采纳 ENG-6 方案 a）——冻结额度应等于实际可展开任务数，否则脏组场景多冻结用户额度。
  - IT5h（观测定义，Step 06 TG-P1-1 修订）：系统无「冻结计数」字段，可观测=金额。fixture：直插混平台脏组（2 facebook + 1 reddit，组平台=facebook，Task 3.1 不拦——它只比组平台与 plan 平台）+ batch_text plan **不带 image_generations**（排除 V2 图片成本干扰）。设 P = seed 的 chat 模型单价（测试内从 `gm_ai_models` 回读，**不许写死**）。断言 `response.frozen_cost == 2*P`（或 `gm_user_wallets.frozen_points` 增量 == 2*P）。
  - 预期 RED：当前 `frozen_cost == 3*P` → 断言失败（失败形态 `2P != 3P`）。禁止以 `frozen_cost > 0` 之类弱断言替代（已镜像入 anti-gaming RED 表）。
  - 两个调用态语义（SM-F3）：create 同步路径（直接内容，:534-546）0 匹配 → IT5i 的 400+回滚；AI 回调路径（:1219，预算已冻结）0 匹配 → IT5g 的 finalize_plan 退款。实现必须区分两态。

**GREEN**：`cargo test -p glance-mind-api` + `pytest -k aipub` 全量。

## 模块独立验证

```bash
cargo test -p glance-mind-api
pytest crates/api/tests/ -k aipub   # 本地跑+输出存档（CI 无 pytest，Dep-F1）
```

## 边界与禁区

- 不触碰 ai_task_types 推断、provider/LLM 调用路径（providers ledger 的 INAPPLICABLE 前提）。
- 不改 plan_type 互斥规则与既有错误文案。
- get_group_account_ids 旧方法若仍有其他调用方则保留，不删。
