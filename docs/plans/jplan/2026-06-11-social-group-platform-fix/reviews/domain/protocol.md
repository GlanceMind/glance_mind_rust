# Protocol Reviewer（2026-06-11，对照 unified_response.rs / client.ts 核实）
- F1 P1: M6 6.4 `error.response.data.message` 字段不存在——拦截器 reject 自定义对象 {isApiError, message=msg, message_cn=msg_cn}；error.response.data 在错误时被 skip。改用 isApiError(error) ? (zh? message_cn : message)
- F2 P2: 拦截器已全局 toast（reportError），组件再 toast 会双弹——任选 _silent:true 组件自管 或 删组件 toast，FT 断言恰一个
- F3 P2: edge 行「0/负数→400」错：axum Query 对 0/-1 反序列化成功→200 空列表；非数字/超界→400 纯文本 body（非统一信封）。拆三行钉死并在 M1 注明信封偏差为框架行为
- F4 P2: 成功码断言 201 不可能——api_ok! 恒 200+code==1000。R001/IT2/IT5d/M1/M3 全改 200+code==1000
- F5 P3: GroupPlatformMismatch 无独立错误码（共享 2001）——断言锚=code==2001+msg 子串；禁止新增 ErrorCode 数字（From<i32> 白名单缺配则 http_status 落 500）
通过：C1 三 arm 模式吻合；404 链路成立（msg 携带 Display 文案可达线上）；C3 兼容性成立；ENG-5 九处清单精确；i18n/秘密无问题。
