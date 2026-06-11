# Design Review — M6（全文存档，2026-06-11，独立评审 agent，已对照前端代码与 i18n 核实）

| ID | Sev | 证据 | 后果 | 补丁 |
|----|-----|------|------|------|
| DES-1 | P1 | E2E1 断言「平台标识=Facebook」但组表（Accounts.tsx:1170-1294）只有名/计数/预览/操作列，无任务实现平台列；DTO 已带 platform_id | E2E1 不可实现或被弱化；迁移拆组后用户无 UI 面理解平台模型 | M6 加 6.1b：组表加平台列（PlatformIcon 已 import :27，:989 有用例；名字经已加载 platforms :246 解析）+FT 断言+列头 i18n 键 |
| DES-2 | P2 | FT5 未说明弹窗自动打开；Accounts.tsx:272-283 只读 tab 参数 | 深链预选不可见；FT5 断言主体不明；非法 platform_id 未定义 | FT5 细化：tab=groups&platform_id=N → showAddGroupModal=true+预填；N 不在 platforms 则忽略留空；消费参数防刷新重开。查询参数机制与现有惯例一致（tab/refVideo/copyFrom） |
| DES-3 | P2 | 账号增/编/批量弹窗组下拉用未过滤 groups（:1483-1497,:1731,:1924）；:464-466 吞错误只弹通用 toast | R004 上线后选错平台组得到无解释的「保存失败」——本计划制造的新死胡同 | 扩 M6 Task 6.4：账号弹窗组选项按所选平台过滤（镜像 AIPubPlanCreate:468）；或至少 Combobox option description 标平台名；再不然 ledger 记 justified gap |
| DES-4 | P3 | 文内惯例是 Combobox 非裸 Select（:1380-1391）；支持 icon | 无阻塞 | 措辞改「复用 add-account Combobox 模式，可带 PlatformIcon」 |
| DES-5 | P3 | t.accounts.selectPlatform* 键中英已存在（en.ts:3518-3521/zh.ts:3585） | 重复造键风险 | 改「复用既有键；仅新列头/说明新增」 |
| DES-6 | P3 | 空态文案已含平台原因（en:3821/zh:3888）+建组 CTA | 足够 | 可选：建组弹窗 tips 加一行「分组隶属单一平台」 |
| DES-7 | P3 | 编辑组弹窗只有名字（:2037-2074） | 平台永久但不可见 | 可选：只读平台徽标随 DES-1 顺带 |

总评：M6 引用全部属实；修 DES-1（P1）与 DES-2 文本、DES-3 范围决策后放行。
