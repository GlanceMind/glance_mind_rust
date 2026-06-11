# Module Plan M6-frontend — 建组平台选择器 + 发布表单组选择（glance_mind_front 仓库）

> 反作弊地板（注入）：同后端——前端测试断言不得改弱，e2e 不得跳过。本模块在 **glance_mind_front** 仓库实施、独立 PR。

Reqs: R009, R010, **R012**（Task 6.1b/6.4/FT4b 的 requirement 锚，Step 06 补）。依赖：M1（R002 服务端过滤）已部署后做 E2E1 验收；组件级改动可先行开发。

## Task 6.1 — 建组弹窗平台选择器（R009，根因修复）

现状：`apps/web/src/pages/Accounts.tsx:105-107` `newGroup = { name: '', platformId: 1 }` 硬编码默认 Reddit；弹窗（:2009-2026）只有组名输入。

**RED 测试（vitest/RTL，遵循前端仓库既有测试惯例与文件位置）**

- FT1：渲染建组弹窗 → 断言存在平台 Select 且初始为空/占位（**无默认选中平台**）。预期 RED：当前无该控件 → 查询失败。
- FT2：只填组名不选平台 → 提交按钮 disabled。预期 RED：当前仅 name 控制 disabled。
- FT3：选 Facebook + 填名 → 提交 payload 断言 `platform_id === 3`。预期 RED：当前恒为 1。

**实现**

1. state 初始 `platformId: ''`（或 null），**删除默认 1**；reset（:567）同步。
2. 弹窗加平台选择：**复用 add-account 弹窗的 `Combobox` 模式（Accounts.tsx:1380-1391，`options={platforms.map(...)}`，可带 `PlatformIcon`）**，非裸 Select（DES-4）。数据源复用页面已有 `platforms` 列表（:246）。
3. 提交校验：name && platformId 才可提交；payload `platform_id: Number(platformId)`。
4. i18n：**复用既有 `t.accounts.selectPlatform*` 键（en.ts:3518-3521 / zh.ts:3585 已存在）**；仅为新增列头/说明造新键（中英两份）（DES-5）。可选：弹窗 tips 列表加一行「分组隶属单一平台」（DES-6）。

## Task 6.1b — 组列表平台列（Step 07 新增，闭 DES-1）

现状：组表（Accounts.tsx:1170-1294）只有名称/账号数/预览/操作列，平台不可见——E2E1 的「平台标识=Facebook」断言无实现载体；迁移拆组后用户也无 UI 面理解平台模型。

**RED 测试**：FT6 渲染组列表（mock 含 facebook 组）→ 断言行内出现平台名/图标。预期 RED：当前无该列。

**实现**：组表加平台列：`PlatformIcon`（:27 已 import，:989 有用例）+ 平台名（经已加载 `platforms` 数组解析，同 :246 模式）；列头 i18n 新键。可选顺带（DES-7）：编辑组弹窗显示只读平台徽标。

## Task 6.2 — 发布表单组选择回归（R010）

现状已基本正确（fetchGroups 传 platform_id、客户端 filter :468、无组时引导建组）。本任务为**回归固化 + 体验衔接**：

- FT4（回归）：mock 组列表含跨平台组 → facebook 任务的组下拉只渲染 platform_id=3 的组。预期 RED 形态：若有人移除 filter 则红（先验证现状绿，作为锁定断言提交）。
- FT4b（CEO-3 空组防呆）：组选择器选项显示 `account_count`（DTO 已带）；`account_count==0` 的组**禁选并提示「该组暂无账号」**。RED：当前可选空组 → 断言失败。
- FT5（深链，Step 07 细化闭 DES-2）：从发布表单点「去建组」跳 `/accounts?tab=groups&platform_id=N`：
  1. 参数存在且 N 在已加载 `platforms` 中 → **自动 `showAddGroupModal=true`** 并预填 `newGroup.platformId=N`（仍可改）；
  2. N 非法/未知 → 忽略参数，弹窗不自动开，选择器留空（必选校验仍把门）；
  3. 参数消费后从 URL 清除（防刷新重开弹窗）。
  RED：当前只读 tab 参数（:272-283）→ 三断言全红。
  - 实现：跳转链接拼参 + Accounts.tsx searchParams 处理。查询参数机制与既有 `tab`/`refVideo`/`copyFrom` 惯例一致。

## Task 6.4 — 账号弹窗组下拉平台过滤（Step 07 新增，闭 DES-3）

现状：账号新增/编辑/批量弹窗的组下拉用未过滤 `groups`（:1483-1497, :1731, :1924）；保存错误被吞成通用 toast（:464-466）。M2 R004 上线后，选错平台组 → 无解释的「保存失败」死胡同。

**RED 测试**：FT7 选 Platform=Facebook 后，三处组下拉仅渲染 facebook 组（mock 跨平台组列表）。预期 RED：当前渲染全部组。

**实现**：三处组 options 按当前所选 `platformId` 过滤（镜像 AIPubPlanCreate.tsx:468）；未选平台时组下拉禁用（先选平台后选组）。

错误透传（Step 06 Protocol-F1/F2 修订——原 `error.response.data.message` 字段**不存在**）：
- 拦截器（client.ts:221-228）reject 的是自定义对象 `{isApiError, code, message=msg, message_cn=msg_cn, ...}`，错误响应的 `data` 字段被 serde skip。正确取法：`isApiError(error) ? (locale==='zh' ? error.message_cn : error.message) : t.accounts.saveAccountFailed`。
- **单 toast 决策**：拦截器已对非 `_silent` 请求全局 `reportError` 弹 toast——保存调用改为 `_silent: true`，由组件自管 toast（恰一个）。FT7b 断言：错误时 toast 文案 == 信封 msg/msg_cn 且只弹一次。
  - FT7b 预期 RED：当前错误被吞成通用 `saveAccountFailed` 文案、且拦截器+组件双 toast → 文案断言与「恰一次」计数断言双失败。

## Task 6.3 — E2E1（链式验收，R002 部署后）

按前端仓库 e2e 惯例（若 e2e 基建不可用——记忆中曾有 Playwright/TLS 受阻——降级为手动验收脚本并留截图/录屏证据，gap 记入 test-suite ledger 的 justified gaps 并在 Step 08 复核）：

1. 建组：选 Facebook、名 `e2e-fb-group` → 列表出现且平台标识=Facebook。
2. 建 facebook post 发布任务 → 组下拉出现 `e2e-fb-group` → 选中创建成功。
3. 反例：reddit 任务的组下拉不出现 `e2e-fb-group`。

## 模块独立验证

```bash
# glance_mind_front 仓库
npm run test      # vitest FT1–FT7（含 6.1b 的 FT6、6.4 的 FT7）
npm run build     # 生产构建绿
# E2E1 按上节，证据存档至本 plan family 的 compile/ 目录
```

## 边界与禁区

- 不动 `aipubTargetContract.ts` 的目标类型契约（video/reel→account 等为既有产品语义）。
- 不动 `platformContentTypes.ts` 的平台 ID 映射。
- 客户端 filter（AIPubPlanCreate.tsx:468）保留（A009 纵深防御）。
- 桌面端（gm_desktop）**已核实为坏**（Step 07 更正 CEO-4）：`account_provider.dart:101-104` 的 createGroup 只发 `{'name': name}`——无 platform_id 且键名错（应为 group_name），对照 `CreateSocialGroupDto` 今天就反序列化失败。**决策：记录 known-broken、开独立跟进票（桌面建组修 payload + 平台选择器），不入本计划范围**——本计划不使其变得更坏，修复属独立桌面任务。原「已有平台选择」的 justified-gap 记录作废。
