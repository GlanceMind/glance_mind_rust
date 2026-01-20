use axum::{
    extract::Path,
    http::{header, StatusCode},
    response::IntoResponse,
    Extension,
};
use rust_xlsxwriter::*;

use glance_mind_db::entity::user::User;
use crate::error::{api_error::ApiError, business_error::BusinessError};
use crate::service::agent_service::AgentService;
use crate::service::campaign_service::CampaignService;

/// Export campaign data to Excel
///
/// GET /api/campaigns/:id/export
pub async fn export_campaign_data(
    Extension(user): Extension<User>,
    Extension(campaign_service): Extension<CampaignService>,
    Extension(agent_service): Extension<AgentService>,
    Path(campaign_id): Path<i32>,
) -> Result<impl IntoResponse, ApiError> {
    // Verify campaign ownership
    let _campaign = campaign_service.get_campaign(campaign_id, user.id).await?;

    // Get videos and comments
    let videos = agent_service.get_campaign_videos(campaign_id).await?;
    let comments = agent_service.get_campaign_comments(campaign_id).await?;

    // Create a new workbook
    let mut workbook = Workbook::new();

    // Add videos sheet
    add_videos_sheet(&mut workbook, &videos)
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

fn add_videos_sheet(
    workbook: &mut Workbook,
    videos: &[glance_mind_db::entity::agent::AgentVideo],
) -> Result<(), XlsxError> {
    let worksheet = workbook.add_worksheet();
    worksheet.set_name("Videos")?;

    // Header format
    let header_format = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0x4472C4))
        .set_font_color(Color::White);

    // Add headers
    worksheet.write_with_format(0, 0, "ID", &header_format)?;
    worksheet.write_with_format(0, 1, "Video ID", &header_format)?;
    worksheet.write_with_format(0, 2, "Author", &header_format)?;
    worksheet.write_with_format(0, 3, "Description", &header_format)?;
    worksheet.write_with_format(0, 4, "Task ID", &header_format)?;
    worksheet.write_with_format(0, 5, "Created At", &header_format)?;

    // Add data
    for (idx, video) in videos.iter().enumerate() {
        let row = (idx + 1) as u32;
        worksheet.write(row, 0, video.id)?;
        worksheet.write(row, 1, video.video_id.as_deref().unwrap_or(""))?;
        worksheet.write(row, 2, video.author.as_deref().unwrap_or(""))?;
        worksheet.write(row, 3, video.description.as_deref().unwrap_or(""))?;
        worksheet.write(row, 4, video.task_id)?;
        worksheet.write(row, 5, video.created_at.to_rfc3339())?;
    }

    // Set column widths
    worksheet.set_column_width(0, 10)?;
    worksheet.set_column_width(1, 25)?;
    worksheet.set_column_width(2, 20)?;
    worksheet.set_column_width(3, 50)?;
    worksheet.set_column_width(4, 10)?;
    worksheet.set_column_width(5, 25)?;

    Ok(())
}

fn add_comments_sheet(
    workbook: &mut Workbook,
    comments: &[glance_mind_db::entity::agent::AgentComment],
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
    worksheet.write_with_format(0, 1, "Video DB ID", &header_format)?;
    worksheet.write_with_format(0, 2, "Comment ID", &header_format)?;
    worksheet.write_with_format(0, 3, "User Nickname", &header_format)?;
    worksheet.write_with_format(0, 4, "User Unique ID", &header_format)?;
    worksheet.write_with_format(0, 5, "Content", &header_format)?;
    worksheet.write_with_format(0, 6, "Reason", &header_format)?;
    worksheet.write_with_format(0, 7, "Suggested Reply", &header_format)?;
    worksheet.write_with_format(0, 8, "Suggested DM", &header_format)?;
    worksheet.write_with_format(0, 9, "Status", &header_format)?;
    worksheet.write_with_format(0, 10, "Create Time", &header_format)?;
    worksheet.write_with_format(0, 11, "Created At", &header_format)?;

    // Status mapping
    let status_text = |status: i16| -> &'static str {
        match status {
            0 => "Pending",
            1 => "Approved",
            2 => "Rejected",
            3 => "Replied",
            _ => "Unknown",
        }
    };

    // Add data
    for (idx, comment) in comments.iter().enumerate() {
        let row = (idx + 1) as u32;
        worksheet.write(row, 0, comment.id)?;
        worksheet.write(row, 1, comment.video_db_id)?;
        worksheet.write(row, 2, &comment.comment_id)?;
        worksheet.write(row, 3, comment.user_nickname.as_deref().unwrap_or(""))?;
        worksheet.write(row, 4, comment.user_unique_id.as_deref().unwrap_or(""))?;
        worksheet.write(row, 5, comment.content.as_deref().unwrap_or(""))?;
        worksheet.write(row, 6, comment.reason.as_deref().unwrap_or(""))?;
        worksheet.write(row, 7, comment.suggested_reply.as_deref().unwrap_or(""))?;
        worksheet.write(row, 8, comment.suggested_dm.as_deref().unwrap_or(""))?;
        worksheet.write(row, 9, status_text(comment.status))?;
        worksheet.write(
            row,
            10,
            comment
                .create_time
                .map(|t| t.to_string())
                .unwrap_or_default()
                .as_str(),
        )?;
        worksheet.write(row, 11, comment.created_at.to_rfc3339())?;
    }

    // Set column widths
    worksheet.set_column_width(0, 10)?;
    worksheet.set_column_width(1, 12)?;
    worksheet.set_column_width(2, 25)?;
    worksheet.set_column_width(3, 20)?;
    worksheet.set_column_width(4, 20)?;
    worksheet.set_column_width(5, 50)?;
    worksheet.set_column_width(6, 40)?;
    worksheet.set_column_width(7, 50)?;
    worksheet.set_column_width(8, 50)?;
    worksheet.set_column_width(9, 12)?;
    worksheet.set_column_width(9, 20)?;
    worksheet.set_column_width(10, 25)?;

    Ok(())
}
