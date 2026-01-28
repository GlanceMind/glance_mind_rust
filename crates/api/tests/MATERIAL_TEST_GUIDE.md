# 素材管理功能测试指南

## 测试账号
- 用户名: `jacksoom`
- 密码: `Lifeng94101`

## 测试环境
- API服务器: `http://localhost:8000`
- 前端服务器: `http://localhost:3000` (Vite dev server)

## 测试流程

### 1. 登录测试
1. 访问 `http://localhost:3000/login`
2. 使用测试账号登录
3. 验证登录成功，跳转到 dashboard

### 2. 素材管理页面测试
1. 点击左侧导航栏的"素材管理"
2. 验证页面加载成功，显示素材列表（可能为空）

### 3. 标签列表测试
1. 在素材管理页面，点击"上传素材"按钮
2. 验证标签下拉框是否加载了从 video_cases 收集的标签
3. 验证标签显示格式：`标签名 (使用次数)`

### 4. 视频上传和AI分析测试（完整流程）
1. 点击"上传素材"按钮
2. 选择一个视频文件（MP4格式，建议小于100MB）
3. 点击"上传到 OSS"按钮
4. 验证上传进度条显示
5. 上传成功后，验证显示"正在分析视频..."提示
6. 等待AI分析完成（可能需要几秒到几十秒）
7. 验证素材创建成功提示
8. 验证素材出现在列表中
9. 验证素材的 prompt 字段已自动填充（AI生成）

### 5. 素材列表测试
1. 验证素材卡片显示：
   - 视频缩略图（如果有）
   - 标题
   - 标签
   - 创建时间
   - AI生成的 prompt（如果有）
2. 测试搜索功能：在搜索框输入关键词
3. 测试标签筛选：选择不同标签进行筛选
4. 测试分页：如果有多个素材，验证分页功能

### 6. 素材详情测试
1. 点击素材卡片上的"查看"按钮
2. 验证视频在新标签页打开

### 7. 素材删除测试
1. 点击素材卡片上的删除按钮
2. 确认删除操作
3. 验证素材从列表中移除

### 8. 从视频库收藏测试
1. 导航到"AI Apps" → 选择应用 → 查看视频库
2. 点击任意视频案例的详情
3. 在详情对话框中，点击"收藏到素材库"按钮
4. 验证收藏成功提示
5. 导航回"素材管理"页面
6. 验证收藏的素材出现在列表中
7. 验证素材数据正确映射：
   - video_url 来自 video_case
   - prompt 来自 video_case 的 ai_prompt 或 script
   - tag 来自 video_case 的 category_name_cn 或 category_name_en
   - thumbnail_url 来自 video_case 的 ai_image_url 或 refer_image_url
   - title 来自 video_case 的 product_name 或 brand_name

## 预期结果

### 成功场景
- ✅ 视频上传到 OSS 成功
- ✅ AI 自动分析视频并生成 prompt
- ✅ 素材创建成功并出现在列表中
- ✅ 从 video_cases 收藏功能正常
- ✅ 标签从 video_cases 正确收集
- ✅ 搜索和筛选功能正常
- ✅ 分页功能正常

### 错误处理
- ✅ 不支持的视频格式显示错误提示
- ✅ 文件过大（>100MB）显示错误提示
- ✅ AI 分析失败时，prompt 字段为空但不影响素材创建
- ✅ 网络错误时显示友好提示

## API 端点测试清单

### 1. POST /api/v1/oss/upload-video
- [ ] 上传视频文件
- [ ] 验证返回 video_url
- [ ] 验证文件大小限制（100MB）
- [ ] 验证文件类型限制

### 2. POST /api/v1/materials
- [ ] 创建素材（自动触发 AI 分析）
- [ ] 验证 prompt 自动生成
- [ ] 验证素材数据保存正确

### 3. GET /api/v1/materials
- [ ] 获取素材列表
- [ ] 测试分页参数
- [ ] 测试标签筛选
- [ ] 测试搜索功能

### 4. GET /api/v1/materials/:id
- [ ] 获取素材详情
- [ ] 验证数据完整性

### 5. PUT /api/v1/materials/:id
- [ ] 更新素材信息
- [ ] 验证更新成功

### 6. DELETE /api/v1/materials/:id
- [ ] 删除素材（软删除）
- [ ] 验证素材不再出现在列表中

### 7. GET /api/v1/material-tags
- [ ] 获取标签列表
- [ ] 验证标签从 video_cases 收集
- [ ] 验证使用次数统计

### 8. POST /api/v1/video-cases/:task_no/favorite
- [ ] 收藏 video_case
- [ ] 验证数据映射正确
- [ ] 验证素材创建成功

## 注意事项

1. **AI 分析时间**：AI 视频分析可能需要 10-60 秒，请耐心等待
2. **OSS 配置**：确保 OSS 环境变量已正确配置
3. **LaoZhang API**：确保 LAOZHANG_API_KEY 已配置
4. **数据库迁移**：确保已运行数据库迁移，创建了 `gm_user_materials` 表
5. **测试数据**：建议使用真实的视频文件进行测试，而不是空文件

## 问题排查

### 上传失败
- 检查 OSS 配置（OSS_ACCESS_KEY_ID, OSS_ACCESS_KEY_SECRET, OSS_ENDPOINT, OSS_BUCKET）
- 检查网络连接
- 检查文件大小和格式

### AI 分析失败
- 检查 LAOZHANG_API_KEY 配置
- 检查视频 URL 是否可访问
- 查看后端日志了解详细错误

### 收藏功能失败
- 检查 video_case 是否存在
- 检查 video_case 是否有 video_url
- 查看后端日志了解详细错误
