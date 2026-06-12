# Anti-Gaming Test Quality Ledger — social-group-platform-fix

地板规则全文见 `constraints/testing-constraints.md`，注入每个 root/module plan。

## RED-for-the-right-reason 计划

每个测试先于实现提交运行，要求失败信息匹配预期形态，证据（失败输出片段）记入模块 plan 的执行记录：

| 测试 | 预期 RED 失败形态（实现前） |
|------|------------------------------|
| IT1（非法 platform 建组） | 当前返回 200+code==1000（api_ok! 恒 200）或 FK 违例 500 → 断言 400 失败 |
| IT3（platform_id 过滤） | 当前返回全部组 → 断言「仅 facebook 组」失败 |
| IT4（绑定平台不一致） | 当前 200（无校验）→ 断言 400 失败 |
| IT5/IT6（plan/campaign 不一致组） | 当前 200+code==1000 创建成功（无校验）→ 断言 400/404 失败 |
| UT2（expand 过滤） | 当前展开全部账号 → 断言「仅匹配账号」失败 |
| IT7（迁移演练） | 迁移前断言 SQL 违规行 >0（fixture 构造）→ 迁移后 =0 |

## 断言不可变

- 本 plan family 涉及的既有测试（test_campaign_api.py、aipub 既有测试）的断言**只读**；实现模块不得修改。
- 若发现既有断言编码了错误期望（如恰好断言了无校验行为），按规则 (b)：停、说明、`ASSERTION-CHANGE-JUSTIFIED: <原因>`、重走 RED→GREEN、断言改动独立 commit。

## 变异测试门

- 核心模块：平台一致性校验函数（M2 产出，M3/M4 复用）。
- 命令（rust）：`cargo mutants --package glance-mind-api -f <校验模块文件>`（cargo-mutants，CI/nightly 口径，非内循环）。
- 阈值：校验模块变异分数 ≥ 80%；低于阈值视为测试不充分，补测试不改阈值。

## 属性测试

- UT1 使用 `proptest`（仓库若未引入，dev-dependency 引入仅限测试目标）：
  - 性质 1：`group.user_id != user_id` ⇒ 必返回归属错误（与平台无关）。
  - 性质 2：`group.platform_id != expected_platform` 且归属正确 ⇒ 必返回平台不匹配错误。
  - 性质 3：归属与平台都正确 ⇒ 必 Ok。
- 三性质合并穷尽输出空间，无法硬编码绕过。

## 职责分离（subdriven 执行时）

- 测试任务（写 IT1–IT7/UT1–UT3 的 RED 版本）与实现任务分属不同子 agent；实现子 agent 的提示词中不含「可修改测试文件」权限。
- 测试改动单独 commit；生产代码与断言同 commit 改动在 PR review 必须书面解释。

## Step 06 增补

### IT5h / IT5g / IT6j / IT7 RED 行补充

| 测试 | 预期 RED 失败形态 |
|------|--------------------|
| IT5h | `frozen_cost == 2*P` 失败，实际 `3*P`（P=seed 单价回读，禁止 `>0` 弱断言） |
| IT5g | billing settled 断言失败（当前产生错平台任务且无失败/退款路径） |
| IT5i | create 400 断言失败（当前成功进入 pending） |
| IT6j | 「comment 不在装配结果」失败（当前走 keep-comment-on-empty 回退） |
| IT7 | 空壳 up.sql 下步骤 4 `assert violations == 0` 报 N>0；失败输出存档后才许填充迁移逻辑 |

### 绿基线锁豁免清单（TG-P3-1）

IT2 / IT3b / **IT4d / IT4g** / IT4e / IT5d / IT5j / IT6c / **IT6d / IT6g** / IT6f / FT4 为「锁定现状」断言（成功路径在实现前本就通过，无法先红——Step 08 F2 补录四条），豁免先红要求；条件：**每条在首个实现 commit 前跑一次并存绿输出**（把「可能已过」变成记录在案的事实）。

### UT1 职责分离细则（PI-P3-6）

UT1 与实现同文件（`#[cfg(test)]`）：分离降级到 commit 粒度——测试块先单独 commit，实现 commit 不得触碰测试块（PR review 检查项）。

### 变异门执行口径修订（Dep-F2/F3）

本地模块完成门（非 CI/nightly——仓库无 mutants CI 配置且无 owning task）：`cargo install cargo-mutants --locked` + `cargo mutants ... -- -- --lib`；输出存档为模块完成证据。阈值 ≥80% 不变。
