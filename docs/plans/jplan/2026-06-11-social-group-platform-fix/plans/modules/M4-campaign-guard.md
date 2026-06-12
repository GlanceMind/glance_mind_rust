# Module Plan M4-campaign-guard — 社媒任务（campaign）组校验

> 反作弊地板（注入）：不得改弱/删除断言、不得 skip/xfail/ignore、不得伪造通过；失败修生产代码；错误期望须 `ASSERTION-CHANGE-JUSTIFIED` 重走 RED→GREEN。`test_campaign_api.py` 既有断言只读。

Reqs: R006, **R011**（Task 4.3 的 requirement 锚，Step 06 补）。依赖：M2 合入。**前置（D3）**：实现前确认本分支未提交的 campaign_{dto,repository,service} 改动已合入或明确隔离——`git status` 干净或这些文件的 diff 与本模块无交叠，证据记入执行记录。

## Task 4.1 — create_campaign 组校验

现状：`campaign_service.rs:115` `social_group_id: dto.social_group_id` 直传；campaign 自身 `platform_id: i32` 必填（entity/campaign.rs:15,30）。

**RED 测试（pytest，新用例并入 `crates/api/tests/test_campaign_api.py`，遵循其 fixture/seed 惯例）**

seed：用户 A（facebook 组 gF、reddit 组 gR）、用户 B（组 gB）。

- IT6a：create campaign `platform_id=3, social_group_id=gR` → 400 GroupPlatformMismatch。预期 RED：当前成功创建。
- IT6b：`social_group_id=gB` → 404 GroupNotFound。预期 RED：当前成功。
- IT6c：`social_group_id=null/缺省` → 成功（组可选，回归固化）。
- IT6d：`social_group_id=gF` → 成功且查库 `social_group_id==gF`（db-field-validation-guard）。

**实现**：create 路径在构造 entity 前：

```rust
if let Some(gid) = dto.social_group_id {
    load_and_check_group(&self.group_repo, gid, user_id, dto.platform_id).await?;
}
```

## Task 4.2 — update_campaign 组校验（Step 07 重写，闭 ENG-1/ENG-9）

现状：`campaign_service.rs:306` `platform_id: dto.platform_id.unwrap_or(existing.platform_id)`；`:320` `social_group_id: dto.social_group_id.or(existing.social_group_id)`——更新可改平台、可换组，零校验。**旁路风险：只改 platform_id 不带 social_group_id 的 PUT 会让旧组在新平台下生效。**

**RED 测试**

- IT6e：update 把 facebook campaign 的组换成 gR → 400。预期 RED：当前成功。
- IT6f：update 不带 social_group_id 且不改平台 → 保留原组（`.or(existing)` 语义回归固化）。
- IT6g：update 换成 gF'（另一 facebook 组）→ 成功落库。
- IT6h（ENG-1 旁路用例）：facebook campaign 已绑 gF；update **只带 `platform_id=1(reddit)`、不带 social_group_id** → 400 GroupPlatformMismatch。预期 RED：当前成功且产生 reddit campaign 挂 facebook 组。

**实现**：校验基于**生效值**而非请求字段：

```rust
let eff_pid = dto.platform_id.unwrap_or(existing.platform_id);
let eff_gid = dto.social_group_id.or(existing.social_group_id);
if let Some(gid) = eff_gid {
    if dto.social_group_id.is_some() || dto.platform_id.map_or(false, |p| p != existing.platform_id) {
        load_and_check_group(&self.group_repo, gid, user_id, eff_pid).await?;
    }
}
```

（组或平台任一发生变化且生效组存在即校验；两者都没动则不重复校验存量数据——存量一致性由 M5 负责。）

## Task 4.3 — campaign 运行时组账号平台过滤（Step 07 新增，闭 CEO-2；R007 对称物）

现状：`agent_service.rs:129-187` 经 `campaign_to_group`（campaign_id→social_group_id）从组装配回复账号，**无平台过滤**——存量错配 campaign 的错平台账号会被持续派去回复。

**RED 测试（rust 单测/集成，按 agent_service 既有测试形态）**

- IT6i：直插 DB 构造脏数据（facebook campaign → 组内 1 facebook + 1 reddit 账号），跑账号装配 → 断言仅 facebook 账号入选 + warn 日志（campaign_id, group_id, skipped_account_id）。测试以 `redis=None` 跑（enforce_daily_limits_inner 直通）。预期 RED：当前两个账号都入选。
- IT6j（Step 06 TG-P2-2 修订，钉死契约）：组内有账号但 0 个平台匹配 → **该 campaign 的 comment 不进装配结果**（排除）+ warn。这**有意偏离**现行 keep-comment-on-empty 回退（agent_service.rs:~186 空账号集时保留 comment）；「真空组（total==0）」分支保持 legacy 直通行为不变并加绿基线回归锁。
  - 预期 RED：当前 0 匹配走 keep-comment 回退分支 → 断言「comment 不在结果集」失败。

**实现**：组账号查询/装配处加 `account.platform_id == campaign.platform_id` 过滤（SQL 级优先），`skipped > 0` 时 `warn!`。与 M3 Task 3.2 的展开过滤同语义。

## 模块独立验证

```bash
pytest crates/api/tests/test_campaign_api.py        # 全量（含既有用例回归）
pytest crates/api/tests/ -k campaign   # 本地跑+输出存档（CI 无 pytest，Dep-F1）
cargo test -p glance-mind-api                       # 含 Task 4.3 的 agent_service 测试
```

RED 用例中「用户 B」的组数据经 `db_cursor` 直插构造（conftest 单用户 seed，无需第二登录态；ENG-12）。

## 边界与禁区

- 不改 campaign 既有业务流（状态机、调度）；只在 create/update 入口加校验。
- 既有 test_campaign_api.py 断言只读；若其 seed 数据违反新不变量导致转红，修 seed（数据非断言）；断言冲突走 ASSERTION-CHANGE-JUSTIFIED。
- 本分支未提交 campaign 改动不混编（独立 commit 边界）。
