use axum::{
    extract::Path,
    http::{header, StatusCode},
    response::IntoResponse,
    Extension,
};
use rust_xlsxwriter::*;

use crate::dto::agent_dto::UnifiedCommentDto;
use crate::dto::crawler_dto::UnifiedContentDto;
use crate::error::{api_error::ApiError, business_error::BusinessError};
use crate::service::campaign_service::CampaignService;
use crate::state::user_state::UserState;
use glance_mind_db::entity::user::User;

/// Export campaign data to Excel
///
/// GET /api/campaigns/:id/export
pub async fn export_campaign_data(
    Extension(user): Extension<User>,
    Extension(campaign_service): Extension<CampaignService>,
    Extension(user_state): Extension<UserState>,
    Path(campaign_id): Path<i32>,
) -> Result<impl IntoResponse, ApiError> {
    // Verify campaign ownership
    let campaign = campaign_service.get_campaign(campaign_id, user.id).await?;

    let contents = user_state
        .crawler_service
        .get_all_campaign_contents_unified(campaign_id, campaign.platform_id)
        .await?;
    let comments = user_state
        .agent_service
        .get_all_campaign_comments_unified(campaign_id, campaign.platform_id)
        .await?;

    // Create a new workbook
    let mut workbook = Workbook::new();

    // Add unified content sheet
    add_contents_sheet(&mut workbook, &contents)
        .map_err(|_| ApiError::BusinessError(BusinessError::ExcelGenerationFailed))?;

    // Add comments sheet
    add_comments_sheet(&mut workbook, &comments)
        .map_err(|_| ApiError::BusinessError(BusinessError::ExcelGenerationFailed))?;

    // Write to buffer
    let buffer = workbook
        .save_to_buffer()
        .map_err(|_| ApiError::BusinessError(BusinessError::ExcelGenerationFailed))?;

    let filename = format!(
        "campaign_{}_export_{}.xlsx",
        campaign_id,
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    );
    let content_disposition = format!("attachment; filename=\"{}\"", filename);

    Ok((
        StatusCode::OK,
        [
            (
                header::CONTENT_TYPE,
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".to_string(),
            ),
            (header::CONTENT_DISPOSITION, content_disposition),
        ],
        buffer,
    ))
}

fn add_contents_sheet(
    workbook: &mut Workbook,
    contents: &[UnifiedContentDto],
) -> Result<(), XlsxError> {
    let worksheet = workbook.add_worksheet();
    worksheet.set_name("Contents")?;

    // Header format
    let header_format = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0x4472C4))
        .set_font_color(Color::White);

    // Add headers
    worksheet.write_with_format(0, 0, "ID", &header_format)?;
    worksheet.write_with_format(0, 1, "Platform", &header_format)?;
    worksheet.write_with_format(0, 2, "Content ID", &header_format)?;
    worksheet.write_with_format(0, 3, "Content Type", &header_format)?;
    worksheet.write_with_format(0, 4, "Task ID", &header_format)?;
    worksheet.write_with_format(0, 5, "Title", &header_format)?;
    worksheet.write_with_format(0, 6, "Author Name", &header_format)?;
    worksheet.write_with_format(0, 7, "Author ID", &header_format)?;
    worksheet.write_with_format(0, 8, "URL", &header_format)?;
    worksheet.write_with_format(0, 9, "Like Count", &header_format)?;
    worksheet.write_with_format(0, 10, "Comment Count", &header_format)?;
    worksheet.write_with_format(0, 11, "Share Count", &header_format)?;
    worksheet.write_with_format(0, 12, "View Count", &header_format)?;
    worksheet.write_with_format(0, 13, "Posted At", &header_format)?;
    worksheet.write_with_format(0, 14, "Created At", &header_format)?;

    // Add data
    for (idx, content) in contents.iter().enumerate() {
        let row = (idx + 1) as u32;
        worksheet.write(row, 0, content.id)?;
        worksheet.write(row, 1, content.platform.as_str())?;
        worksheet.write(row, 2, content.content_id.as_str())?;
        worksheet.write(row, 3, content.content_type.as_str())?;
        worksheet.write(row, 4, content.task_id)?;
        worksheet.write(row, 5, content.title.as_deref().unwrap_or(""))?;
        worksheet.write(row, 6, content.author_name.as_deref().unwrap_or(""))?;
        worksheet.write(row, 7, content.author_id.as_deref().unwrap_or(""))?;
        worksheet.write(row, 8, content.url.as_deref().unwrap_or(""))?;
        worksheet.write(
            row,
            9,
            content
                .like_count
                .map(|v| v.to_string())
                .unwrap_or_default()
                .as_str(),
        )?;
        worksheet.write(
            row,
            10,
            content
                .comment_count
                .map(|v| v.to_string())
                .unwrap_or_default()
                .as_str(),
        )?;
        worksheet.write(
            row,
            11,
            content
                .share_count
                .map(|v| v.to_string())
                .unwrap_or_default()
                .as_str(),
        )?;
        worksheet.write(
            row,
            12,
            content
                .view_count
                .map(|v| v.to_string())
                .unwrap_or_default()
                .as_str(),
        )?;
        worksheet.write(
            row,
            13,
            content
                .posted_at
                .map(|v| v.to_rfc3339())
                .unwrap_or_default()
                .as_str(),
        )?;
        worksheet.write(row, 14, content.created_at.to_rfc3339())?;
    }

    // Set column widths
    worksheet.set_column_width(0, 10)?;
    worksheet.set_column_width(1, 12)?;
    worksheet.set_column_width(2, 25)?;
    worksheet.set_column_width(3, 18)?;
    worksheet.set_column_width(4, 10)?;
    worksheet.set_column_width(5, 50)?;
    worksheet.set_column_width(6, 20)?;
    worksheet.set_column_width(7, 20)?;
    worksheet.set_column_width(8, 50)?;
    worksheet.set_column_width(9, 12)?;
    worksheet.set_column_width(10, 14)?;
    worksheet.set_column_width(11, 12)?;
    worksheet.set_column_width(12, 12)?;
    worksheet.set_column_width(13, 25)?;
    worksheet.set_column_width(14, 25)?;

    Ok(())
}

fn add_comments_sheet(
    workbook: &mut Workbook,
    comments: &[UnifiedCommentDto],
) -> Result<(), XlsxError> {
    let worksheet = workbook.add_worksheet();
    worksheet.set_name("Comments")?;

    // Header format
    let header_format = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0x4472C4))
        .set_font_color(Color::White);

    // Add headers
    worksheet.write_with_format(0, 0, "ID", &header_format)?;
    worksheet.write_with_format(0, 1, "Platform", &header_format)?;
    worksheet.write_with_format(0, 2, "Content DB ID", &header_format)?;
    worksheet.write_with_format(0, 3, "Comment ID", &header_format)?;
    worksheet.write_with_format(0, 4, "User Name", &header_format)?;
    worksheet.write_with_format(0, 5, "User ID", &header_format)?;
    worksheet.write_with_format(0, 6, "Content", &header_format)?;
    worksheet.write_with_format(0, 7, "Parent Comment ID", &header_format)?;
    worksheet.write_with_format(0, 8, "Like Count", &header_format)?;
    worksheet.write_with_format(0, 9, "Reply Count", &header_format)?;
    worksheet.write_with_format(0, 10, "Reason", &header_format)?;
    worksheet.write_with_format(0, 11, "Suggested Reply", &header_format)?;
    worksheet.write_with_format(0, 12, "Suggested DM", &header_format)?;
    worksheet.write_with_format(0, 13, "Suggested Reply Post", &header_format)?;
    worksheet.write_with_format(0, 14, "Status", &header_format)?;
    worksheet.write_with_format(0, 15, "Comment Created At", &header_format)?;
    worksheet.write_with_format(0, 16, "Created At", &header_format)?;

    // Add data
    for (idx, comment) in comments.iter().enumerate() {
        let row = (idx + 1) as u32;
        worksheet.write(row, 0, comment.id)?;
        worksheet.write(row, 1, comment.platform.as_str())?;
        worksheet.write(row, 2, comment.content_db_id)?;
        worksheet.write(row, 3, comment.comment_id.as_str())?;
        worksheet.write(row, 4, comment.user_name.as_deref().unwrap_or(""))?;
        worksheet.write(row, 5, comment.user_id.as_deref().unwrap_or(""))?;
        worksheet.write(row, 6, comment.content.as_deref().unwrap_or(""))?;
        worksheet.write(row, 7, comment.parent_comment_id.as_deref().unwrap_or(""))?;
        worksheet.write(
            row,
            8,
            comment
                .like_count
                .map(|v| v.to_string())
                .unwrap_or_default()
                .as_str(),
        )?;
        worksheet.write(
            row,
            9,
            comment
                .reply_count
                .map(|v| v.to_string())
                .unwrap_or_default()
                .as_str(),
        )?;
        worksheet.write(row, 10, comment.reason.as_deref().unwrap_or(""))?;
        worksheet.write(row, 11, comment.suggested_reply.as_deref().unwrap_or(""))?;
        worksheet.write(row, 12, comment.suggested_dm.as_deref().unwrap_or(""))?;
        worksheet.write(
            row,
            13,
            comment.suggested_reply_post.as_deref().unwrap_or(""),
        )?;
        worksheet.write(row, 14, comment.status.as_str())?;
        worksheet.write(
            row,
            15,
            comment
                .comment_created_at
                .map(|t| t.to_rfc3339())
                .unwrap_or_default()
                .as_str(),
        )?;
        worksheet.write(row, 16, comment.created_at.to_rfc3339())?;
    }

    // Set column widths
    worksheet.set_column_width(0, 10)?;
    worksheet.set_column_width(1, 12)?;
    worksheet.set_column_width(2, 14)?;
    worksheet.set_column_width(3, 25)?;
    worksheet.set_column_width(4, 20)?;
    worksheet.set_column_width(5, 20)?;
    worksheet.set_column_width(6, 50)?;
    worksheet.set_column_width(7, 20)?;
    worksheet.set_column_width(8, 12)?;
    worksheet.set_column_width(9, 12)?;
    worksheet.set_column_width(10, 40)?;
    worksheet.set_column_width(11, 50)?;
    worksheet.set_column_width(12, 50)?;
    worksheet.set_column_width(13, 50)?;
    worksheet.set_column_width(14, 12)?;
    worksheet.set_column_width(15, 25)?;
    worksheet.set_column_width(16, 25)?;

    Ok(())
}
