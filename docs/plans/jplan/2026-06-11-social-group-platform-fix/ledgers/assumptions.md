# Assumption Ledger — social-group-platform-fix

| ID | Item | Sub-problem | Why-ladder (≥3 或到 bedrock) | Class | Disposition | Evidence / derivation |
|----|------|-------------|------------------------------|-------|-------------|------------------------|
| A001 | 账户组按平台区分 | 全局 | why?→任务按平台过滤组；why?→发布展开要求组=单平台账号集；why?→用户明示「适配账户组平台这个条件」 | BEDROCK (B1) | keep | 用户请求原文 2026-06-11；schema platform_id NOT NULL (B2) |
| A002 | 建组必须显式选平台 | S1 | why?→组平台必须真实；why?→默认值=1 已污染 68/69 生产数据；why?→B1/B2 | DERIVED←B1,B2 | keep | Accounts.tsx:107 硬编码默认是根因 |
| A003 | 组列表服务端按 platform_id 过滤 | S2 | why?→前端已传参但被丢弃；why?→多客户端需一致契约；why?→组平台是 bedrock 维度 | DERIVED←B1,B4 | keep | common.rs:4-11 缺字段；AIPubPlanCreate.tsx:765-779 已在传参 |
| A004 | 账号入组校验平台一致+组归属 | S3 | why?→混平台组使展开错发；why?→绑定是不变量的唯一其余写入口；why?→B3 | DERIVED←B2,B3,B4 | keep | social_account_service.rs:127-136 现状无校验 |
| A005 | aipub/campaign 创建校验组归属+组平台==任务平台 | S4 | why?→消费端是最后防线；why?→group_id 是客户端输入；why?→B4 | DERIVED←B3,B4 | keep | aipub_service.rs:84-141 仅查 group_id 存在性；campaign_service.rs:115 直传 |
| A006 | 存量数据回填迁移 | S5 | why?→现存组 platform_id 不可信；why?→校验上线后旧组将不可用/误用；why?→B1 语义对齐 | DERIVED←B1,B2 | keep | 生产 68/69=reddit（既有排查记录，执行前复核） |
| A007 | 混平台组处理=按账号平台**拆组** | S5 | why?→回填需唯一平台；why?→取多数会踢出少数账号（丢用户数据）；why?→拆组保全部账号且满足不变量 | DERIVED←B2,B3 | keep | D1 最终形态：原组保留多数平台（id 不变），少数平台账号迁入 `<原组名>-<platform_name>`（SELECT-then-INSERT 解析目标 id） |
| A013 | 两段发布 PR-A/PR-B（D2） | 上线 | why?→回填须先于校验生效；why?→迁移随 deploy 自动执行（B5 管线事实）；why?→单 PR 会让校验与未洗数据同窗 | DERIVED←B5,B1 | keep | deploy-api.yml:43-57 自动迁移并 gate deploy（合并前须复核未变更） |
| A014 | 计费冻结计数=平台匹配账号数 | S4 | why?→冻结额度应等于可展开任务数；why?→脏组场景 freeze=total 会多冻用户额度；why?→B3 展开契约定义可展开集合 | DERIVED←B3 | keep | ENG-6 方案 a；被拒分支：freeze=total（多冻，且与展开结果不守恒） |
| A015 | campaign 运行时装配平台过滤（R011） | S4 | why?→存量错配引用永久错发违反目标句；why?→消费端兜底与 R007 同力（B3/B4）；why?→迁移报告只列不改 | DERIVED←B3,B4 | keep | CEO-2；agent_service.rs:129-187 核实无平台过滤 |
| A016 | 前端平台可见+防呆（R012） | S6 | why?→E2E1 需平台标识载体；why?→R004 上线后错选组需可解释错误；why?→空组选中是延迟失败死胡同 | DERIVED←B1,B4 | keep | DES-1/DES-3/CEO-3；组列表 DTO 已带 platform_id/account_count，零数据成本 |
| A008 | update_group 不允许改平台 | S3 | why?→改平台使组内账号瞬间违反不变量；why?→无用户需求；why?→B2/B3 | DERIVED←B2,B3 | keep | UpdateSocialGroupDto 现状本就只有 group_name——保持现状即可 |
| A009 | 前端保留客户端二次 filter | S6 | why?→防后端回归；why?→纵深防御非正确性依据 | DERIVED←B4 | keep | AIPubPlanCreate.tsx:468 已存在，零成本保留 |
| A010 | 用 DB 触发器/复合 FK 强制账号-组平台一致 | S3 | why?→“DB 强一致更安全”；why?→惯例直觉；（仓库无触发器惯例，应用层+迁移断言+CI 已覆盖该不变量→无 bedrock force 强制 DB 层） | ASSUMED | **drop** | 复杂度与惯例成本＞收益；若回填后再现违规数据，升级为 DB 约束（记入 root plan 风险节） |
| A011 | 只修前端默认值（最小补丁） | S1 | why?→改动最小；why?→“前端是根因”；（但服务端无过滤无校验→任何客户端仍可写垃圾→违反 B4） | ASSUMED | **drop** | B4 要求服务端强制；最小补丁不满足 goal 句 |
| A012 | 新建独立的「按平台查组」端点 | S2 | why?→“不动旧契约更安全”；（PageRequest 加 Optional 字段向后兼容，旧调用不受影响→无新端点必要） | ASSUMED | **drop** | YAGNI：`platform_id: Option<i32>` 缺省=不过滤，契约兼容（api-contract-guard 验证） |

无遗留 ASSUMED 项。A010/A011/A012 为机制假设的 drop，不裁剪用户需求。
