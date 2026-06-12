# Module Plan M1-group-crud — 组 CRUD 平台化

> 反作弊地板（注入）：不得改弱/删除断言、不得 skip/xfail/ignore、不得伪造通过；失败修生产代码；错误期望须 `ASSERTION-CHANGE-JUSTIFIED` 重走 RED→GREEN。测试任务与实现任务分属不同上下文（subdriven）。

Reqs: R001, R002, R003。依赖：无（首发模块）。消费方：M2（错误响应惯例）、M6（R002 参数）。

## Task 1.1 — 服务端组列表平台过滤（R002/R003）

**RED 测试（先写，pytest，并入既有 `crates/api/tests/test_social_api.py::TestSocialGroups`；ENG-10）**

- IT3a：seed 同一用户 reddit 组×2 + facebook 组×1 → `GET /api/v1/social-groups?platform_id=3` 断言 `list` 长度=1、`list[0].platform_id==3`、`total==1`。
  - 预期 RED 失败：当前后端忽略参数，返回 3 个组 → `assert len == 1` 失败（`3 != 1`）。
- IT3b（回归契约）：不带 `platform_id` → 返回 3 组、total=3、响应字段含 `platform_id/group_name/account_count`（shape 锁定）。
  - 预期 RED：现状即应通过（绿基线）；若红说明 seed/断言错，先修测试侧（此时尚无实现改动，不构成断言博弈）。

**实现**

1. `crates/api/src/dto/common.rs` — `PageRequest` 增加 `pub platform_id: Option<i32>`（serde 缺省 None）。
2. `crates/api/src/repository/social_group_repository.rs::find_by_user` — 签名加 `platform_id: Option<i32>`；count 与 items 两查询同条件 `.filter(social_groups::platform_id.eq(p))`（Some 时）。
3. `crates/api/src/service/social_group_service.rs::list_groups` — 透传 `req.platform_id`。
4. handler 无改动（Query 自动反序列化新字段）。
5. **（ENG-5）9 处 `PageRequest { .. }` 字面量构造补 `platform_id: None`**：`agent_handler.rs:44`、`template_service.rs:49`、`tool_registry.rs:1145, 1609, 1623, 1679, 1700, 2360, 2381`（编译器会逐一指出；这些文件计入本模块输出清单）。

注意（ENG-10 修正）：`PageRequest` 是共享 DTO——`group_id` 字段的真实消费者是 **template 列表**（template_service.rs:52），账号列表用的是独立的 `AccountListRequest`。其余 `Query<PageRequest>` 端点（crawler/campaign/wallet/template handler）今天就静默忽略未知参数，新增 Optional 字段行为等同，无契约破坏。`platform_id` 不顺手加给其他端点语义（out-of-scope）。

**GREEN 命令**：`pytest crates/api/tests/ -k social_group` 全绿；`cargo test -p glance-mind-api` 全绿。
**验收门**：api-contract-guard（响应 shape 不变 + 新参数文档化）；IT3a/IT3b RED→GREEN 证据入执行记录。

## Task 1.2 — 建组 platform_id 合法性校验（R001）

**RED 测试**

- IT1：`POST /api/v1/social-groups {platform_id: 999, group_name: "x"}` → 断言 400 + 错误信息含 `Invalid platform_id`；断言库中无该组。
  - 预期 RED 失败：当前直接落库返回 **200+code==1000**（api_ok! 恒 200，Protocol-F4；或 FK 违例 500）→ 断言 400 失败（形态 `200 != 400`）。
- IT2：`{platform_id: 3, group_name: "fb-group"}` → **HTTP 200 + body.code==1000**（api_ok! 恒 200，无 201——Protocol-F4），查库 `platform_id==3`（db-field-validation-guard：逐字段断言）。
  - 绿基线锁（豁免清单见 anti-gaming ledger）：实现前跑一次留绿输出。
- IT3c（edge，Protocol-F3 钉死）：`?platform_id=0` / `=-1` → **200 + 空 list + total==0**（axum Query 反序列化成功，落入无匹配过滤）；`?platform_id=abc` / 超 i32 字符串 → **400 + axum 纯文本拒绝体**（只断状态码；该 400 不走统一信封，是框架行为，接受为既定偏差——前端落通用 HTTP 错误分支）。
  - 预期 RED：`0/-1` 当前参数被丢弃返回全部组 → `total==0` 失败（形态 `3 != 0`）；`abc` 当前字段不存在被忽略 → 返回 200 → 断言 400 失败（字段落地后 axum 才拒绝）。

**实现**

`social_group_service.rs::create_group`：插入前查 `gm_platforms` 存在性（repo 加 `platform_exists(id) -> bool` 或复用现有 platform repo——起草时已知 aipub_service.rs:60-66 有「Invalid platform_id」先例，对齐其取数方式与文案）；不存在 → `BusinessError::InvalidInput(format!("Invalid platform_id: {id}"))`。

**GREEN 命令**：同上 pytest -k social_group。
**验收门**：IT1/IT2 证据；`rust-verify-change`。

## 模块独立验证

```bash
cargo test -p glance-mind-api
pytest crates/api/tests/ -k social_group   # 本地跑+输出存档（CI 无 pytest，Dep-F1）
# api-contract-guard 按仓库流程
```

## 边界与禁区

- 不改 `UpdateSocialGroupDto`（平台不可改，A008）。
- 不动账号列表对 PageRequest 的既有语义。
- 不在本模块引入校验函数/错误枚举（M2 职责）。
