use crate::config::database::Database;
use crate::dto::material_dto::{
    CreateMaterialRequest, MaterialDetail, MaterialListItem, MaterialListQuery,
    MaterialListResponse, MaterialTag, MaterialTagsResponse, UpdateMaterialRequest,
};
use crate::error::{api_error::ApiError, db_error::DbError};
use crate::repository::material_repository::MaterialRepository;
use crate::service::laozhang_client::LaoZhangClient;
use crate::service::video_case_service::VideoCaseService;
use chrono::Utc;
use diesel::result::Error as DieselError;
use glance_mind_db::entity::material::NewUserMaterial;
use std::sync::Arc;
use tracing::{info, warn};

#[derive(Clone)]
pub struct MaterialService {
    material_repo: MaterialRepository,
    video_case_service: VideoCaseService,
    laozhang_client: LaoZhangClient,
}

impl MaterialService {
    pub fn new(
        db_conn: &Arc<Database>,
        laozhang_client: LaoZhangClient,
        video_case_service: VideoCaseService,
    ) -> Self {
        Self {
            material_repo: MaterialRepository::new(db_conn.pool.clone()),
            video_case_service,
            laozhang_client,
        }
    }

    /// Analyze video using AI to generate prompt
    async fn analyze_video_for_prompt(&self, video_url: &str) -> Option<String> {
        // Use gemini-2.5-flash for faster analysis (can be changed to gemini-2.5-pro for better quality)
        let model = "gemini-2.5-flash";
        let analysis_prompt = "请详细描述这个视频的内容、风格、主题和关键元素，用于后续AI视频生成。要求描述具体、详细，包含视觉元素、动作、情感和整体氛围。";

        info!("Starting AI video analysis for prompt generation: video_url='{}'", video_url);

        match self
            .laozhang_client
            .analyze_video(model, video_url, analysis_prompt, Some(2000))
            .await
        {
            Ok(prompt) => {
                info!("AI video analysis completed successfully, prompt length: {}", prompt.len());
                Some(prompt)
            }
            Err(e) => {
                warn!("AI video analysis failed: {}, continuing without prompt", e);
                None // Return None if analysis fails, material can still be created
            }
        }
    }

    /// Create a new material
    pub async fn create_material(
        &self,
        user_id: i32,
        request: CreateMaterialRequest,
    ) -> Result<MaterialDetail, ApiError> {
        // Analyze video to generate prompt
        let prompt = self.analyze_video_for_prompt(&request.video_url).await;

        let new_material = NewUserMaterial {
            user_id,
            video_url: request.video_url,
            prompt,
            thumbnail_url: None, // Can be extracted later if needed
            tag: Some(request.tag), // Required tag
            title: Some(request.title), // Required title
            description: request.description,
            duration: None, // Can be extracted from video later if needed
            file_size: None, // Can be extracted from video later if needed
            is_active: Some(true),
            created_at: Utc::now(),
            updated_at: None,
        };

        let material = self
            .material_repo
            .create(new_material)
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        info!("Material created successfully: id={}, user_id={}", material.id, user_id);

        Ok(MaterialDetail::from(material))
    }

    /// List materials for a user
    pub async fn list_materials(
        &self,
        user_id: i32,
        query: MaterialListQuery,
    ) -> Result<MaterialListResponse, ApiError> {
        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(20);

        let (materials, total) = self
            .material_repo
            .list_by_user(user_id, page as i64, page_size as i64, query.tag.clone(), query.search.clone())
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        let list: Vec<MaterialListItem> = materials.into_iter().map(MaterialListItem::from).collect();

        Ok(MaterialListResponse {
            list,
            total,
            page,
            page_size,
        })
    }

    /// Get material by ID
    pub async fn get_material(
        &self,
        id: i32,
        user_id: i32,
    ) -> Result<MaterialDetail, ApiError> {
        let material = self
            .material_repo
            .find_by_id_and_user(id, user_id)
            .await
            .map_err(|e| match e {
                DieselError::NotFound => ApiError::BusinessError(
                    crate::error::business_error::BusinessError::ResourceNotFound("Material".to_string()),
                ),
                _ => ApiError::from(DbError::SomethingWentWrong(e.to_string())),
            })?;

        Ok(MaterialDetail::from(material))
    }

    /// Update material
    pub async fn update_material(
        &self,
        id: i32,
        user_id: i32,
        request: UpdateMaterialRequest,
    ) -> Result<MaterialDetail, ApiError> {
        // If video URL changed, re-analyze to generate new prompt
        let prompt = if let Some(ref video_url) = request.video_url {
            Some(self.analyze_video_for_prompt(video_url).await)
        } else {
            None
        };

        let material = self
            .material_repo
            .update(
                id,
                user_id,
                request.video_url,
                prompt.flatten(),
                request.tag,
                request.title,
                request.description,
            )
            .await
            .map_err(|e| match e {
                DieselError::NotFound => ApiError::BusinessError(
                    crate::error::business_error::BusinessError::ResourceNotFound("Material".to_string()),
                ),
                _ => ApiError::from(DbError::SomethingWentWrong(e.to_string())),
            })?;

        Ok(MaterialDetail::from(material))
    }

    /// Delete material (soft delete)
    pub async fn delete_material(&self, id: i32, user_id: i32) -> Result<(), ApiError> {
        self.material_repo
            .delete(id, user_id)
            .await
            .map_err(|e| match e {
                DieselError::NotFound => ApiError::BusinessError(
                    crate::error::business_error::BusinessError::ResourceNotFound("Material".to_string()),
                ),
                _ => ApiError::from(DbError::SomethingWentWrong(e.to_string())),
            })?;

        info!("Material deleted: id={}, user_id={}", id, user_id);
        Ok(())
    }

    /// Collect tags from video_cases
    pub async fn collect_tags(&self) -> Result<MaterialTagsResponse, ApiError> {
        let tags_data = self
            .material_repo
            .collect_tags_from_video_cases()
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        let tags: Vec<MaterialTag> = tags_data
            .into_iter()
            .map(|(name, name_cn, usage_count)| MaterialTag {
                name: name.clone(),
                name_cn: Some(name_cn),
                name_en: Some(name),
                usage_count,
            })
            .collect();

        Ok(MaterialTagsResponse { tags })
    }

    /// Favorite material from video_case
    pub async fn favorite_from_video_case(
        &self,
        user_id: i32,
        task_no: &str,
    ) -> Result<MaterialDetail, ApiError> {
        // Get video_case by task_no using video_case_service
        let video_case_detail = self
            .video_case_service
            .get_by_task_no(task_no)
            .await?;

        // Extract data from video_case_detail
        // Find the first video with video_url
        let video_url = video_case_detail
            .videos
            .iter()
            .find_map(|v| v.video_url.clone())
            .or(video_case_detail.refer_video_url.clone())
            .ok_or_else(|| {
                ApiError::BusinessError(crate::error::business_error::BusinessError::InvalidInput(
                    "Video case has no video_url".to_string(),
                ))
            })?;

        // Extract prompt: prefer ai_prompt from first video, fallback to script
        let prompt = video_case_detail
            .videos
            .first()
            .and_then(|v| v.ai_prompt.clone())
            .or(video_case_detail.script.clone());

        // Extract tag: use category_name_cn (VideoCaseDetail only has category_name_cn)
        let tag = video_case_detail.category_name_cn.clone();

        // Extract thumbnail: prefer ai_image_url from first video, fallback to first image_urls
        let thumbnail_url = video_case_detail
            .videos
            .first()
            .and_then(|v| v.ai_image_url.clone())
            .or_else(|| video_case_detail.image_urls.first().cloned());

        // Extract title: prefer product_name, fallback to brand_name, then task_no
        let title = video_case_detail
            .product_name
            .clone()
            .or(video_case_detail.brand_name.clone())
            .or(Some(task_no.to_string()));

        // Extract description
        let description = video_case_detail.selling_point.clone();

        let new_material = NewUserMaterial {
            user_id,
            video_url,
            prompt,
            thumbnail_url,
            tag,
            title,
            description,
            duration: None,
            file_size: None,
            is_active: Some(true),
            created_at: Utc::now(),
            updated_at: None,
        };

        let material = self
            .material_repo
            .create(new_material)
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        info!(
            "Material favorited from video_case: id={}, task_no={}, user_id={}",
            material.id, task_no, user_id
        );

        Ok(MaterialDetail::from(material))
    }
}
