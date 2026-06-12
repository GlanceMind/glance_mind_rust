# CEO Review（全文存档，2026-06-11，独立评审 agent，已对照代码核实）

见 summary.md 的归并。本文件保存完整发现表。

| ID | Sev | 发现 | 证据 | 后果 | 补丁建议 |
|----|-----|------|------|------|----------|
| CEO-1 | P1 | 回填先于校验的要求与 root D2 单 PR 部署矛盾；未说明迁移是否随部署自动执行 | root.md 上线顺序 vs M5 头部依赖行；manifest A3 | 部署后到 NC1 间窗口内，错标组在新校验下从「能用但错」变成「全面 400」 | 二选一：(a) 迁移随部署管线执行，NC1 确认并入部署窗口；(b) 拆两次后端发布：R1=M1+M5（部署+回填+断言=0），R2=M2/M3/M4 校验 |
| CEO-2 | P1 | campaign 运行时消费组无平台过滤（aipub 有 R007，campaign 没有对称物） | agent_service.rs:129-187 campaign_to_group→组账号分配回复，无平台过滤 | 存量错配 campaign 的 reddit 账号永久回复 facebook 评论，违反目标句 | M4 加任务：agent_service 组账号装配处加平台过滤（R007 对称）；或 NC1 验收强制处置错配引用清单。推荐前者 |
| CEO-3 | P2 | 空组死胡同：R005 不查组内有无账号，失败延迟到 expand | M3 IT5g；social_account_dto.rs:40-41 已有 account_count | 用户新建组忘绑账号→选组→异步失败，感知「还是坏的」 | M6：组选择器显示 account_count，0 账号禁选/警告；可选后端 create_plan 加「组空」400 |
| CEO-4 | P2 | M6 桌面端假设错误：gm_desktop createGroup 只发 {'name': name}，无 platform_id 且键名错，今天就反序列化失败 | gm_desktop account_provider.dart:101-104 vs CreateSocialGroupDto | justified gap 建立在假象上，Step 08 会签发错误记录；桌面建组已坏且修完仍坏 | 更正 M6 边界文本；显式决策：小型桌面跟进任务 或 记录 known-broken+开票 |
| CEO-5 | P2 | 发布窗口内旧前端继续造 platform_id=1 组，一次性迁移不复跑 | invariants 失败模式表；M5 幂等性节 | 窗口期垃圾组被新校验锁死，用户困惑 | root 最终验收加：M6 上线后复跑一致性报告，非 0 则幂等重跑回填；前端 PR 与 NC1 同日压缩窗口 |
| CEO-6 | P3 | 拆组对用户不可见；组名含平台词可能与修正后标签矛盾 | requirements D1；M5 Task5.1 | 与混组数量成正比的困惑（演练报告量化） | 演练计数>少量则发布说明/横幅；拆组幂等需覆盖「目标名已存在」分支（重挂到既有组 id） |
| CEO-7 | P3 | 前提确认：平台维度分组正确，无陷阱；跨平台发布未来=任务多组选择，本设计不封死 | 01-first-principles §4 | 无 | root 可加一行注记防止日后复活跨平台组 |

总评：根因完整、无过度工程；修 CEO-1/2，并入 CEO-3/5 一行级补充，更正 CEO-4 记录即可放行。
