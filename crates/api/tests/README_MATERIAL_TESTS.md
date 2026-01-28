# Material Management API 测试指南

## 测试概述

素材管理功能的完整测试套件，包括：
1. 视频上传到 OSS
2. 创建素材（包含名称和类目，触发 AI 分析）
3. 列表查询（支持分页、筛选、搜索）
4. 获取素材详情
5. 更新素材
6. 删除素材
7. 从视频库收藏素材
8. 获取标签列表

## 运行测试

### 前置条件

1. **API 服务运行中**
   ```bash
   cd glance_mind_rust
   make dev-restart
   ```

2. **环境变量配置**
   - OSS 配置（`ALIYUN_OSS_*`）
   - LaoZhang API Key（`LAOZHANG_API_KEY`）
   - 数据库连接（`DATABASE_URL`）

3. **测试账号**
   - 用户名: `jacksoom`
   - 密码: `Lifeng94101`

### 运行所有测试

```bash
cd glance_mind_rust
cargo test --test material_integration_test -- --ignored --nocapture
```

### 运行单个测试

```bash
# 测试视频上传
cargo test --test material_integration_test test_upload_video_to_oss -- --ignored --nocapture

# 测试创建素材（包含名称和类目）
cargo test --test material_integration_test test_create_material_with_ai_analysis -- --ignored --nocapture

# 测试完整流程
cargo test --test material_integration_test test_complete_material_flow -- --ignored --nocapture
```

## 测试用例说明

### 1. `test_upload_video_to_oss`
- **目的**: 测试视频上传到 OSS
- **验证**: 返回有效的视频 URL
- **要求**: OSS 配置正确

### 2. `test_create_material_with_ai_analysis`
- **目的**: 测试创建素材，包含必填字段（名称、类目）
- **验证**: 
  - 素材成功创建
  - 名称和类目正确保存
  - AI 分析生成 prompt（可选）
- **要求**: LaoZhang API 配置正确

### 3. `test_list_materials`
- **目的**: 测试素材列表查询
- **验证**: 返回分页列表和总数

### 4. `test_list_material_tags`
- **目的**: 测试标签列表
- **验证**: 返回从 video_cases 收集的标签

### 5. `test_favorite_from_video_case`
- **目的**: 测试从视频库收藏素材
- **验证**: 成功创建素材并提取信息

### 6. `test_complete_material_flow`
- **目的**: 测试完整流程
- **步骤**:
  1. 上传视频
  2. 创建素材（名称、类目必填）
  3. 列表查询
  4. 获取详情
  5. 更新素材
  6. 删除素材

### 7. `test_create_material_missing_required_fields`
- **目的**: 测试缺少必填字段的情况
- **说明**: 后端验证必填字段，前端也应验证

## API 端点

### POST `/api/v1/oss/upload-video`
上传视频到 OSS
- **Content-Type**: `multipart/form-data`
- **Field**: `file` (视频文件)
- **限制**: 最大 100MB

### POST `/api/v1/materials`
创建素材
- **必填字段**:
  - `video_url`: 视频 URL
  - `title`: 素材名称
  - `tag`: 类目标签
- **可选字段**:
  - `description`: 描述
- **自动生成**:
  - `prompt`: AI 分析生成的提示词

### GET `/api/v1/materials`
列表查询
- **查询参数**:
  - `page`: 页码（默认 1）
  - `page_size`: 每页数量（默认 20）
  - `tag`: 按标签筛选
  - `search`: 搜索关键词

### GET `/api/v1/materials/:id`
获取素材详情

### PUT `/api/v1/materials/:id`
更新素材
- **可选字段**: `title`, `tag`, `description`, `video_url`

### DELETE `/api/v1/materials/:id`
删除素材

### GET `/api/v1/material-tags`
获取标签列表（从 video_cases 收集）

### POST `/api/v1/video-cases/:task_no/favorite`
从视频库收藏素材

## 前端操作流程优化

### 优化后的流程

1. **打开上传弹窗**
   - 点击"上传素材"按钮

2. **填写必填信息**（在上传前）
   - 素材名称（必填，带红色 *）
   - 类目（必填，从下拉列表选择，带红色 *）
   - 描述（可选）

3. **选择视频文件**
   - 点击文件选择器
   - 验证文件类型和大小（最大 100MB）

4. **上传并创建**
   - 点击"上传并创建素材"按钮
   - 显示上传进度
   - 上传成功后自动触发 AI 分析
   - 显示分析进度

5. **完成**
   - 素材创建成功
   - 自动刷新列表
   - 关闭弹窗

### 验证规则

- **素材名称**: 必填，不能为空
- **类目**: 必填，必须从列表中选择
- **视频文件**: 必填，支持 MP4/MOV/AVI/WebM/MKV，最大 100MB
- **描述**: 可选

## 注意事项

1. **AI 分析**: 创建素材时会自动调用 AI 分析视频生成 prompt，可能需要一些时间
2. **标签来源**: 标签从 `video_cases` 表的 `category_name_cn/en` 字段收集
3. **数据持久化**: 所有字段（名称、类目、描述）都会保存到数据库
4. **错误处理**: 前端和后端都有验证，确保数据完整性
