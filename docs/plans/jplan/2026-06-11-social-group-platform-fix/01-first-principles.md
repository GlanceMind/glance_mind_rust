# Step 01 — First-Principles Reduction

## 1. Irreducible Goal（一句话，不含机制）

**对任一平台的社媒任务或发布任务，用户能且只能选到自己在该平台下的账户组，且任务最终触达的每个账号都确属该平台。**

（注意：「修建组弹窗」「加 platform_id 过滤」都是机制，不进目标句。）

## 2. Decomposition（按不同作用力拆分）

| # | 子问题 | 作用力（force） | 为何独立 |
|---|---|---|---|
| S1 | 组的平台身份在**创建时**必须真实 | 数据正确性；actor=建组用户；failure mode=默认值静默污染 | 写入侧错误是其余一切错误的源头 |
| S2 | 按平台检索组必须在**服务端**可靠 | API 契约一致性（web/desktop/未来客户端）；failure mode=参数被 serde 静默丢弃 | 契约问题，与写入正确性无关；前端 filter 只是单客户端兜底 |
| S3 | 账号↔组绑定必须保持**平台一致性不变量** | 数据不变量；actor=单个/批量绑定路径；failure mode=混平台组 | 即使 S1/S2 正确，绑定路径仍可破坏不变量 |
| S4 | 任务**消费端**（aipub plan、campaign）必须校验组归属与组平台 | 安全/正确性边界——服务端不能信任何客户端；failure mode=错平台展开发布、越权用他人组 | 消费端是最后防线，失败后果（发到错平台/他人组）与 S1–S3 不同 |
| S5 | **存量数据**必须修复 | 历史数据形态（68/69 组=reddit、组内混平台）≠ 模型语义；一次性迁移 + 生产执行窗口 | 纯数据问题，有独立的风险面（生产库、回滚） |
| S6 | 前端创建/选择体验适配 | 用户操作界面；独立仓库、独立部署节奏 | 跨 repo 交付力不同 |

## 3. Why-Ladder 分类

详表见 `ledgers/assumptions.md`。结论：

- BEDROCK ×4：B1 用户明示「账户组按平台区分，任务/发布都要适配」；B2 schema 硬契约 `gm_social_groups.platform_id NOT NULL` + FK；B3 发布契约（batch_text/reddit/grooming 按组展开，组必须能代表单一平台的账号集合）；B4 服务端安全边界（不可信客户端输入）。
- DERIVED ×8（建组显式选平台、服务端平台过滤、绑定一致性校验、消费端归属+平台校验、回填迁移、混平台组拆组、update 不可改平台、前端选择器/兜底过滤保留）。
- ASSUMED ×3，全部已处置（DB 触发器→drop；「只修前端默认值」→drop；新建独立查询端点→drop，复用 PageRequest）。

## 4. Reconstruction Note

```text
Bedrock forces:
  B1 用户明示需求（2026-06-11 请求原文）
  B2 schema 契约 gm_social_groups.platform_id NOT NULL + gm_platforms FK（schema.rs:1461,1952）
  B3 发布展开契约：plan 按 group_id 展开到组内全部账号（aipub_service.rs:1459-1490）
  B4 服务端安全边界：group_id/platform_id 均为客户端输入，必须服务端校验
  B5 部署管线契约（Step 06 FP-F3 补录）：crates/db/migrations 变更随 deploy 自动事务执行并 gate deploy
     （deploy-api.yml:43-57；PR-A 合并前须复核该 gate 未被改动）——D2 两段发布由此推导

Minimal construction:
  写入侧（S1/S6）：建组必须显式携带合法 platform_id（前端加选择器，后端校验 platform 存在）
  读取侧（S2）：组列表接口支持服务端 platform_id 过滤（扩展现有 PageRequest，不新建端点）
  不变量（S3）：账号入组时校验 账号.platform_id == 组.platform_id 且组属于该用户
  消费侧（S4）：aipub create_plan 与 campaign create/update 校验 组属于用户 且 组.platform_id == 任务.platform_id
  数据（S5）：迁移回填——按组内账号真实平台回填组平台；混平台组拆组（每平台一组，账号重挂）；
            迁移后一致性断言 SQL 必须 0 违规行
  防御纵深：前端客户端 filter 保留（单客户端兜底，不作为正确性依据）

Conventional approaches NOT taken:
  1) 跨平台组（删 platform_id，执行期按账号平台筛选）——违反 B1 用户决策；且 B3 的展开契约
     要求组能代表单一平台账号集合，执行期筛选会让“发给组”语义不可预测。
  2) DB 触发器/复合外键强制账号-组平台一致——仓库无触发器惯例，复杂度高；
     应用层校验 + 回填断言 + CI 集成测试已覆盖该不变量（若日后再现违规数据可升级为 DB 约束）。
  3) 只改前端默认值 platformId:1 的最小补丁——根因仍在：任何客户端仍可写垃圾数据、
     服务端过滤仍缺失、消费端仍不校验（违反 B4）。

Gate applicability findings:
  反测试作弊地板            REQUIRED  （地板，不可挑战；constraints/testing-constraints.md）
  特性确定性 mock 测试       REQUIRED  （每个校验/过滤行为：rust 单测 + 既有 python 集成测试惯例）
  真实生产依赖测试           REQUIRED  （真实依赖=Postgres；沿用 crates/api/tests 真 DB 集成测试 + 迁移在
                                       staging/库副本上演练后才上生产）
  mock-provider 门           INAPPLICABLE （本任务不新增/不修改任何外部 provider/LLM/第三方 API 调用；
                                       改动面=DTO/校验/查询/迁移/前端表单。证据：inventory 全部缺陷点
                                       均在 DB 与请求校验层）
  real-provider 门           INAPPLICABLE （同上证据）
  LLM API boundary 覆盖      INAPPLICABLE （无 LLM 行为变更）
  属性测试（核心逻辑）        REQUIRED  （平台一致性校验函数：任意 (账号平台,组平台,任务平台) 组合，
                                       不一致必拒、一致必放）
  变异分数门槛               REQUIRED  （校验模块为核心模块，纳入 CI/nightly 变异口径）
```

## 5. 范围说明（dropped requirement 提示）

无用户需求被裁剪。被 drop 的三项均为机制假设（见 ledger），不构成 scope 变更。
S5 生产库执行（47.236.115.179）保留为外部依赖：计划内交付迁移与演练证据，实际执行窗口由用户确认。
