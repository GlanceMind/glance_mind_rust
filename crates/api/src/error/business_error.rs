use crate::response::error_code::ErrorCode;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BusinessError {
    #[error("Filename has no extension")]
    FilenameMissingExtension,

    #[error("Unsupported image format: .{0}. Only JPG, PNG, WebP supported")]
    UnsupportedImageFormat(String),

    #[error("Image too large ({0:.2}MB), max 5MB supported. Please compress and retry.")]
    ImageTooLarge(f64),

    #[error("Must provide either prompt or image")]
    PromptOrImageRequired,

    #[error("Dual image mode requires both start frame and end frame")]
    DualImageRequiresStartAndEnd,

    #[error("Model ID {0} does not exist or is not available")]
    ModelNotFound(i32),

    #[error("Current model does not support dual image input, please select an FL model (model name contains 'FL')")]
    ModelNotSupportDualImage,

    #[error("Text-to-video requires prompt")]
    TextToVideoRequiresPrompt,

    #[error("Start frame image too large ({0:.2}MB), max 5MB supported")]
    StartFrameTooLarge(f64),

    #[error("End frame image too large ({0:.2}MB), max 5MB supported")]
    EndFrameTooLarge(f64),

    #[error("Invalid status")]
    InvalidStatus,

    #[error("Insufficient available balance to start campaign")]
    InsufficientBalanceForCampaign,

    #[error("Social account {0} not found")]
    SocialAccountNotFound(i32),

    #[error("You don't have permission to operate on this task")]
    TaskPermissionDenied,

    // Resource Not Found Errors
    #[error("Template not found")]
    TemplateNotFound,

    #[error("Campaign not found")]
    CampaignNotFound,

    #[error("Task not found")]
    TaskNotFound,

    #[error("Video task not found: {0}")]
    VideoTaskNotFound(String),

    #[error("Account not found")]
    AccountNotFound,

    #[error("Group not found")]
    GroupNotFound,

    #[error("Group platform {group_platform_id} does not match required platform {expected_platform_id}")]
    GroupPlatformMismatch {
        group_platform_id: i32,
        expected_platform_id: i32,
    },

    #[error("Item not found")]
    ItemNotFound,

    #[error("{0} not found")]
    ResourceNotFound(String),

    // Permission Errors
    #[error("You don't have permission to access this template")]
    TemplatePermissionDenied,

    #[error("You don't have permission to access this campaign")]
    CampaignPermissionDenied,

    #[error("You don't have permission to access this account")]
    AccountPermissionDenied,

    #[error("You don't have permission to access this video task")]
    VideoTaskPermissionDenied,

    // Validation & Input Errors
    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    #[error("Username already exists")]
    UsernameAlreadyExists,

    #[error("Missing required parameter: {0}")]
    MissingRequiredParameter(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Failed to parse form")]
    FormParsingFailed,

    #[error("Failed to read form field: {0}")]
    InvalidFormField(String),

    #[error("Invalid file type: {0}. Allowed types: JPG, PNG, GIF, WebP, BMP")]
    InvalidFileType(String),

    #[error("File too large. Maximum size is {0} bytes")]
    FileTooLarge(usize),

    // Data Operation Errors
    #[error("Failed to generate Excel file")]
    ExcelGenerationFailed,

    #[error("Failed to hash password")]
    PasswordHashFailed,
}

impl BusinessError {
    pub fn to_error_code(&self) -> ErrorCode {
        match self {
            BusinessError::FilenameMissingExtension => ErrorCode::BadRequest,
            BusinessError::UnsupportedImageFormat(_) => ErrorCode::BadRequest,
            BusinessError::ImageTooLarge(_) => ErrorCode::BadRequest,
            BusinessError::PromptOrImageRequired => ErrorCode::BadRequest,
            BusinessError::DualImageRequiresStartAndEnd => ErrorCode::BadRequest,
            BusinessError::ModelNotFound(_) => ErrorCode::BadRequest,
            BusinessError::ModelNotSupportDualImage => ErrorCode::BadRequest,
            BusinessError::TextToVideoRequiresPrompt => ErrorCode::BadRequest,
            BusinessError::StartFrameTooLarge(_) => ErrorCode::BadRequest,
            BusinessError::EndFrameTooLarge(_) => ErrorCode::BadRequest,
            BusinessError::InvalidStatus => ErrorCode::BadRequest,
            BusinessError::InsufficientBalanceForCampaign => ErrorCode::BadRequest,
            BusinessError::SocialAccountNotFound(_) => ErrorCode::NotFound,
            BusinessError::TaskPermissionDenied => ErrorCode::Forbidden,

            // Resource Not Found Errors
            BusinessError::TemplateNotFound => ErrorCode::TemplateNotFound,
            BusinessError::CampaignNotFound => ErrorCode::CampaignNotFound,
            BusinessError::TaskNotFound => ErrorCode::TaskNotFound,
            BusinessError::VideoTaskNotFound(_) => ErrorCode::VideoNotFound,
            BusinessError::AccountNotFound => ErrorCode::NotFound,
            BusinessError::GroupNotFound => ErrorCode::NotFound,
            BusinessError::GroupPlatformMismatch { .. } => ErrorCode::BadRequest,
            BusinessError::ItemNotFound => ErrorCode::NotFound,
            BusinessError::ResourceNotFound(_) => ErrorCode::NotFound,

            // Permission Errors
            BusinessError::TemplatePermissionDenied => ErrorCode::Forbidden,
            BusinessError::CampaignPermissionDenied => ErrorCode::Forbidden,
            BusinessError::AccountPermissionDenied => ErrorCode::Forbidden,
            BusinessError::VideoTaskPermissionDenied => ErrorCode::Forbidden,

            // Validation & Input Errors
            BusinessError::ValidationFailed(_) => ErrorCode::ValidationError,
            BusinessError::UsernameAlreadyExists => ErrorCode::UserAlreadyExists,
            BusinessError::MissingRequiredParameter(_) => ErrorCode::BadRequest,
            BusinessError::InvalidInput(_) => ErrorCode::BadRequest,
            BusinessError::FormParsingFailed => ErrorCode::BadRequest,
            BusinessError::InvalidFormField(_) => ErrorCode::BadRequest,
            BusinessError::InvalidFileType(_) => ErrorCode::BadRequest,
            BusinessError::FileTooLarge(_) => ErrorCode::BadRequest,

            // Data Operation Errors
            BusinessError::ExcelGenerationFailed => ErrorCode::InternalServerError,
            BusinessError::PasswordHashFailed => ErrorCode::InternalServerError,
        }
    }

    pub fn to_message_cn(&self) -> String {
        match self {
            BusinessError::FilenameMissingExtension => "文件名没有后缀".to_string(),
            BusinessError::UnsupportedImageFormat(ext) => {
                format!("不支持的图片格式: .{}。仅支持 JPG, PNG, WebP", ext)
            }
            BusinessError::ImageTooLarge(size) => {
                format!("图片过大 ({:.2}MB)，最大支持 5MB。请压缩后重试。", size)
            }
            BusinessError::PromptOrImageRequired => "必须提供提示词或图片".to_string(),
            BusinessError::DualImageRequiresStartAndEnd => {
                "双图模式需要同时提供起始帧和结束帧".to_string()
            }
            BusinessError::ModelNotFound(id) => format!("模型 ID {} 不存在或不可用", id),
            BusinessError::ModelNotSupportDualImage => {
                "当前模型不支持双图输入，请选择 FL 模型（模型名称包含 'FL'）".to_string()
            }
            BusinessError::TextToVideoRequiresPrompt => "文生视频需要提供提示词".to_string(),
            BusinessError::StartFrameTooLarge(size) => {
                format!("起始帧图片过大 ({:.2}MB)，最大支持 5MB", size)
            }
            BusinessError::EndFrameTooLarge(size) => {
                format!("结束帧图片过大 ({:.2}MB)，最大支持 5MB", size)
            }
            BusinessError::InvalidStatus => "状态无效".to_string(),
            BusinessError::InsufficientBalanceForCampaign => {
                "余额不足，无法启动广告活动".to_string()
            }
            BusinessError::SocialAccountNotFound(id) => format!("社交账号 {} 不存在", id),
            BusinessError::TaskPermissionDenied => "您没有权限操作此任务".to_string(),

            // Resource Not Found Errors
            BusinessError::TemplateNotFound => "模板未找到".to_string(),
            BusinessError::CampaignNotFound => "活动未找到".to_string(),
            BusinessError::TaskNotFound => "任务未找到".to_string(),
            BusinessError::VideoTaskNotFound(task_id) => format!("视频任务未找到: {}", task_id),
            BusinessError::AccountNotFound => "账号未找到".to_string(),
            BusinessError::GroupNotFound => "分组未找到".to_string(),
            BusinessError::GroupPlatformMismatch {
                group_platform_id,
                expected_platform_id,
            } => {
                format!(
                    "分组平台 {} 与所需平台 {} 不匹配",
                    group_platform_id, expected_platform_id
                )
            }
            BusinessError::ItemNotFound => "项目未找到".to_string(),
            BusinessError::ResourceNotFound(resource) => format!("{}未找到", resource),

            // Permission Errors
            BusinessError::TemplatePermissionDenied => "您没有权限访问此模板".to_string(),
            BusinessError::CampaignPermissionDenied => "您没有权限访问此活动".to_string(),
            BusinessError::AccountPermissionDenied => "您没有权限访问此账号".to_string(),
            BusinessError::VideoTaskPermissionDenied => "您没有权限访问此视频任务".to_string(),

            // Validation & Input Errors
            BusinessError::ValidationFailed(msg) => format!("验证失败: {}", msg),
            BusinessError::UsernameAlreadyExists => "用户名已存在".to_string(),
            BusinessError::MissingRequiredParameter(param) => format!("缺少必需参数: {}", param),
            BusinessError::InvalidInput(msg) => format!("无效输入: {}", msg),
            BusinessError::FormParsingFailed => "表单解析失败".to_string(),
            BusinessError::InvalidFormField(field) => format!("表单字段读取失败: {}", field),
            BusinessError::InvalidFileType(file_type) => {
                format!(
                    "无效的文件类型: {}。支持的类型: JPG, PNG, GIF, WebP, BMP",
                    file_type
                )
            }
            BusinessError::FileTooLarge(max_size) => {
                format!("文件过大。最大允许 {} 字节", max_size)
            }

            // Data Operation Errors
            BusinessError::ExcelGenerationFailed => "Excel文件生成失败".to_string(),
            BusinessError::PasswordHashFailed => "密码加密失败".to_string(),
        }
    }
}
