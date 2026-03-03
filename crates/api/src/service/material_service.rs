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
use tokio::sync::Semaphore;
use tracing::{error, info, warn};

const AI_ANALYSIS_TIMEOUT_SECS: u64 = 120;
const AI_ANALYSIS_MAX_RETRIES: u32 = 3;
const AI_ANALYSIS_BASE_DELAY_SECS: u64 = 5;
const AI_ANALYSIS_MAX_CONCURRENT: usize = 2;

#[derive(Clone)]
pub struct MaterialService {
    material_repo: MaterialRepository,
    video_case_service: VideoCaseService,
    laozhang_client: LaoZhangClient,
    analysis_semaphore: Arc<Semaphore>,
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
            analysis_semaphore: Arc::new(Semaphore::new(AI_ANALYSIS_MAX_CONCURRENT)),
        }
    }

    fn is_retryable_error(err: &ApiError) -> bool {
        let msg = format!("{:?}", err);
        msg.contains("429") || msg.contains("Too Many Requests") || msg.contains("rate")
            || msg.contains("负载已饱和") || msg.contains("503") || msg.contains("Service Unavailable")
    }

    /// Spawn background AI analysis task with retry and concurrency limiting
    fn spawn_background_analysis(&self, material_id: i32, video_url: String) {
        let laozhang_client = self.laozhang_client.clone();
        let material_repo = self.material_repo.clone();
        let semaphore = self.analysis_semaphore.clone();

        tokio::spawn(async move {
            let _permit = match semaphore.acquire().await {
                Ok(p) => p,
                Err(_) => {
                    error!("Analysis semaphore closed: material_id={}", material_id);
                    return;
                }
            };

            let model = "gemini-2.5-flash";
            let analysis_prompt = "请详细描述这个视频的内容、风格、主题和关键元素，用于后续AI视频生成。要求描述具体、详细，包含视觉元素、动作、情感和整体氛围。";

            info!(
                "Background AI analysis started: material_id={}, video_url='{}'",
                material_id,
                &video_url[..video_url.len().min(80)]
            );

            for attempt in 0..=AI_ANALYSIS_MAX_RETRIES {
                if attempt > 0 {
                    let delay = AI_ANALYSIS_BASE_DELAY_SECS * 2u64.pow(attempt - 1);
                    info!(
                        "Retrying AI analysis: material_id={}, attempt={}/{}, delay={}s",
                        material_id, attempt, AI_ANALYSIS_MAX_RETRIES, delay
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                }

                let analysis_result = tokio::time::timeout(
                    std::time::Duration::from_secs(AI_ANALYSIS_TIMEOUT_SECS),
                    laozhang_client.analyze_video(model, &video_url, analysis_prompt, Some(2000)),
                )
                .await;

                match analysis_result {
                    Ok(Ok(prompt)) => {
                        info!(
                            "Background AI analysis completed: material_id={}, prompt_len={}, attempts={}",
                            material_id, prompt.len(), attempt + 1
                        );
                        if let Err(e) = material_repo.update_prompt(material_id, prompt).await {
                            error!(
                                "Failed to update material prompt: material_id={}, error={}",
                                material_id, e
                            );
                        }
                        return;
                    }
                    Ok(Err(ref e)) if Self::is_retryable_error(e) && attempt < AI_ANALYSIS_MAX_RETRIES => {
                        warn!(
                            "Retryable AI analysis error: material_id={}, attempt={}/{}, error={}",
                            material_id, attempt + 1, AI_ANALYSIS_MAX_RETRIES, e
                        );
                        continue;
                    }
                    Ok(Err(e)) => {
                        warn!(
                            "Background AI analysis failed (non-retryable): material_id={}, error={}",
                            material_id, e
                        );
                        return;
                    }
                    Err(_) if attempt < AI_ANALYSIS_MAX_RETRIES => {
                        warn!(
                            "AI analysis timed out, will retry: material_id={}, attempt={}/{}",
                            material_id, attempt + 1, AI_ANALYSIS_MAX_RETRIES
                        );
                        continue;
                    }
                    Err(_) => {
                        warn!(
                            "Background AI analysis timed out after all retries: material_id={}, timeout={}s",
                            material_id, AI_ANALYSIS_TIMEOUT_SECS
                        );
                        return;
                    }
                }
            }
        });
    }

    /// Manually trigger re-analysis for a material with missing prompt
    pub async fn re_analyze_material(
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

        info!(
            "Manual re-analysis triggered: material_id={}, user_id={}, has_prompt={}",
            id, user_id, material.prompt.is_some()
        );

        self.spawn_background_analysis(id, material.video_url.clone());

        Ok(MaterialDetail::from(material))
    }

    /// Create a new material
    /// Material is created immediately; AI video analysis runs in the background.
    pub async fn create_material(
        &self,
        user_id: i32,
        request: CreateMaterialRequest,
    ) -> Result<MaterialDetail, ApiError> {
        let video_url = request.video_url.clone();

        let new_material = NewUserMaterial {
            user_id,
            video_url: request.video_url,
            prompt: None,
            thumbnail_url: None,
            tag: Some(request.tag),
            title: Some(request.title),
            description: request.description,
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
            "Material created successfully: id={}, user_id={}",
            material.id, user_id
        );

        self.spawn_background_analysis(material.id, video_url);

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
    /// If video URL changed, background AI re-analysis is triggered.
    pub async fn update_material(
        &self,
        id: i32,
        user_id: i32,
        request: UpdateMaterialRequest,
    ) -> Result<MaterialDetail, ApiError> {
        let video_url_changed = request.video_url.is_some();
        let new_video_url = request.video_url.clone();

        let material = self
            .material_repo
            .update(
                id,
                user_id,
                request.video_url,
                None, // prompt will be updated by background task if video changed
                request.tag,
                request.title,
                request.description,
            )
            .await
            .map_err(|e| match e {
                DieselError::NotFound => ApiError::BusinessError(
                    crate::error::business_error::BusinessError::ResourceNotFound(
                        "Material".to_string(),
                    ),
                ),
                _ => ApiError::from(DbError::SomethingWentWrong(e.to_string())),
            })?;

        if video_url_changed {
            if let Some(url) = new_video_url {
                self.spawn_background_analysis(id, url);
            }
        }

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
