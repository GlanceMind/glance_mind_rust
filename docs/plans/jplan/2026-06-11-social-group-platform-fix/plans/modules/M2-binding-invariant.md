# Module Plan M2-binding-invariant — 校验函数 + 账号↔组绑定一致性

> 反作弊地板（注入）：不得改弱/删除断言、不得 skip/xfail/ignore、不得伪造通过；失败修生产代码；错误期望须 `ASSERTION-CHANGE-JUSTIFIED` 重走 RED→GREEN。测试与实现分上下文。

Reqs: R004 + 共享契约 C1/C2（root.md）。依赖：M1 已合入。消费方：M3/M4。

## Task 2.1 — 共享错误枚举 + 校验纯函数（C1/C2）

**RED 测试（rust 单测 + proptest，与实现同 crate 但先写先红）**

文件：`crates/api/src/service/validation/group_platform.rs` 的 `#[cfg(test)]` 模块（新文件，测试先行——函数签名按 root.md C2 冻结，测试编译失败不算 RED，先放空实现 `todo!()` 让断言红）。

- UT1 属性测试（proptest，三性质，见 anti-gaming ledger）：
  - P1: `group.user_id != user_id` ⇒ `Err(GroupNotFound)`（与平台值无关）。
  - P2: 归属正确且 `group.platform_id != expected` ⇒ `Err(GroupPlatformMismatch{..})` 且错误体含两侧平台。
  - P3: 归属与平台均正确 ⇒ `Ok(())`。
  - 预期 RED：`todo!()` panic / 占位实现返回 Ok ⇒ P1/P2 失败。
- 普通单测：错误 Display 文案断言（对齐 C1 文案模板）。

**实现**

1. `crates/api/src/error/business_error.rs`：新增 `GroupPlatformMismatch { group_platform_id: i32, expected_platform_id: i32 }`（与 root C1 定稿一致，i32 id）。三个 arm（ENG-2 核实的既有模式）：`#[error("Group platform {group_platform_id} does not match required platform {expected_platform_id}")]` Display、`to_error_code() → ErrorCode::BadRequest`（→400）、`to_message_cn()` 中文文案。平台**名字**渲染不在纯函数职责内；需要名字的调用方由 `load_and_check_group` 显式多查 `gm_platforms`（本计划默认不做）。
2. `crates/api/src/service/validation/mod.rs` + `group_platform.rs`：`check_group_platform`（纯函数）与 `load_and_check_group`（取数装配：repo.find_by_id 不存在→GroupNotFound；再调纯函数）。`service/mod.rs` 挂模块。

**GREEN**：`cargo test -p glance-mind-api validation::group_platform`。
**变异门（模块完成时）**：`cargo mutants --package glance-mind-api -f '*validation/group_platform*'` 分数 ≥80%。

## Task 2.2 — update_account 绑定校验（R004）

现状：`social_account_service.rs::update_account`（约 :127-136）`gid==0→None，否则直接 Some(gid)`，零校验。

**RED 测试（pytest IT4，`test_social_account_api.py`）**

seed：用户 A（facebook 账号 a1、facebook 组 gF、reddit 组 gR）、用户 B（组 gB）。

- IT4a 绑不存在组（gid=99999）→ 404；IT4b 绑他人组 gB → 404（同响应，防枚举）；IT4c 绑平台不一致组 gR → 400 且错误码/文案=GroupPlatformMismatch；IT4d 绑 gF → 200 且查库 `group_id==gF`；IT4e 解绑 gid=0 → 200 且 `group_id IS NULL`（回归固化）。
  - 预期 RED：IT4a/b/c 当前全部 200 → 断言失败 `200 != 404/400`。

**实现**：`update_account` 的 `Some(gid) && gid != 0` 分支改为 `load_and_check_group(&group_repo, gid, user_id, account.platform_id)` 后再赋值。service 需注入 `SocialGroupRepository`（构造处同步改）。

## Task 2.3 — 批量创建账号绑定校验（R004）

现状：批量路径（约 :270-300）`group_id: dto.group_id` 直传。

**RED 测试**：IT4f 批量创建 platform=facebook 且 `group_id=gR`（reddit 组）→ 400 GroupPlatformMismatch，且断言一个账号都没落库（原子性）；IT4g `group_id=gF` → 成功且每行 `group_id==gF`。
  - 预期 RED：IT4f 当前成功落库 → 断言失败。

**实现**：批量入口在生成 candidates 前调用同一校验函数（一次校验，整批共享）。

## 模块独立验证

```bash
cargo test -p glance-mind-api
pytest crates/api/tests/ -k social_account        # 本地跑+输出存档（CI 无 pytest，Dep-F1）
cargo install cargo-mutants --locked              # 工具钉锁（Dep-F2）
cargo mutants --package glance-mind-api -f '*validation/group_platform*' -- -- --lib   # ≥80%
# `-- -- --lib` 限定只跑 lib 单测（Dep-F3：-f 只限变异对象不限测试集，
#  否则每个 mutant 都会跑 DB-backed rust 集成测试——慢且需活 DB）
```

UT1 与实现同文件的职责分离（PI-P3-6 决议）：分离在 **commit 粒度**强制——UT1 的 `#[cfg(test)]` 块先于实现单独 commit；实现 commit 的 diff 不得触碰该测试块（PR review 检查项）。

## Task 2.4 — 移除未防护死代码（Step 07 新增，闭 ENG-11）

`social_account_repository.rs:275-292` `batch_update_group_by_profile_names` 写 group_id 零校验且全仓无调用方——**删除**（防未来调用方静默绕过 INV2）。验证：`cargo build` 绿 + 全仓 grep 无引用。属死代码删除，无 RED 测试义务（无行为可断言）；删除单独 commit。

## 边界与禁区

- 单个 `create_account` 不动（group_id 恒 None，无校验面）。
- 不动 aipub/campaign（M3/M4）。
- proptest 若为新 dev-dependency：仅加 `[dev-dependencies]`，不进生产依赖（framework ledger）。
- IT4 系列「用户 B」的组经 `db_cursor` 直插构造（conftest 单用户 seed；ENG-12）。
