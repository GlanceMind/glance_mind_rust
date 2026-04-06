use crate::config::database::{DBPool, Database};
use crate::dto::novel_dto::*;
use chrono::{DateTime, Utc};
use diesel::dsl::max;
use diesel::prelude::*;
use glance_mind_db::entity::novel::*;
use glance_mind_db::schema::{
    gm_novel_architectures, gm_novel_blueprint_chapters, gm_novel_blueprints,
    gm_novel_chapter_prompts, gm_novel_chapters, gm_novel_character_state_snapshots,
    gm_novel_consistency_checks, gm_novel_embedding_profiles,
    gm_novel_global_summary_snapshots, gm_novel_jobs, gm_novel_knowledge_chunks,
    gm_novel_knowledge_imports, gm_novel_llm_profiles, gm_novel_plot_arc_snapshots,
    gm_novel_project_config_snapshots, gm_novel_projects, gm_novel_stage_events,
    gm_novel_stage_runs,
};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct NovelService {
    pool: DBPool,
}

struct ParsedChapter {
    chapter_number: i32,
    chapter_title: String,
    chapter_role: String,
    chapter_purpose: String,
    suspense_level: String,
    foreshadowing: String,
    plot_twist_level: String,
    chapter_summary: String,
}

impl NovelService {
    pub fn new(db_conn: &Arc<Database>) -> Self {
        Self {
            pool: db_conn.pool.clone(),
        }
    }

    // =========================================================================
    // Project methods
    // =========================================================================

    pub fn create_project(
        &self,
        user_id: i32,
        req: &NovelProjectCreateRequest,
    ) -> Result<NovelProject, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;
        let project_id = Uuid::new_v4().to_string();

        conn.transaction::<_, diesel::result::Error, _>(|conn| {
            let new_project = NewNovelProject {
                project_id: project_id.clone(),
                user_id,
                title: req.title.clone(),
                topic: req.topic.clone(),
                genre: req.genre.clone(),
                description: req.description.clone(),
                num_chapters: req.num_chapters,
                target_words_per_chapter: req.target_words_per_chapter,
                default_user_guidance: req.default_user_guidance.clone(),
                status: "draft".to_string(),
                metadata: json!({}),
            };

            let project = diesel::insert_into(gm_novel_projects::table)
                .values(&new_project)
                .get_result::<NovelProject>(conn)?;

            let snap = NewNovelProjectConfigSnapshot {
                project_id: project_id.clone(),
                architecture_llm_profile_id: req.config_snapshot.architecture_llm_profile_id,
                chapter_outline_llm_profile_id: req
                    .config_snapshot
                    .chapter_outline_llm_profile_id,
                prompt_draft_llm_profile_id: req.config_snapshot.prompt_draft_llm_profile_id,
                final_chapter_llm_profile_id: req.config_snapshot.final_chapter_llm_profile_id,
                consistency_review_llm_profile_id: req
                    .config_snapshot
                    .consistency_review_llm_profile_id,
                embedding_profile_id: req.config_snapshot.embedding_profile_id,
                proxy_setting: req.config_snapshot.proxy_setting.clone(),
                webdav_config: req.config_snapshot.webdav_config.clone(),
                other_params: req.config_snapshot.other_params.clone(),
                is_current: true,
            };

            diesel::insert_into(gm_novel_project_config_snapshots::table)
                .values(&snap)
                .execute(conn)?;

            Ok(project)
        })
        .map_err(|e| e.to_string())
    }

    pub fn list_projects(
        &self,
        user_id: i32,
        status: Option<&str>,
        page: i64,
        page_size: i64,
    ) -> Result<(Vec<NovelProject>, i64), String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;
        let offset = (page - 1).max(0) * page_size;

        let total: i64 = if let Some(s) = status {
            gm_novel_projects::table
                .filter(gm_novel_projects::user_id.eq(user_id))
                .filter(gm_novel_projects::deleted_at.is_null())
                .filter(gm_novel_projects::status.eq(s))
                .count()
                .get_result(&mut conn)
        } else {
            gm_novel_projects::table
                .filter(gm_novel_projects::user_id.eq(user_id))
                .filter(gm_novel_projects::deleted_at.is_null())
                .count()
                .get_result(&mut conn)
        }
        .map_err(|e| e.to_string())?;

        let mut query = gm_novel_projects::table
            .filter(gm_novel_projects::user_id.eq(user_id))
            .filter(gm_novel_projects::deleted_at.is_null())
            .into_boxed();

        if let Some(s) = status {
            query = query.filter(gm_novel_projects::status.eq(s.to_string()));
        }

        let items = query
            .order(gm_novel_projects::created_at.desc())
            .offset(offset)
            .limit(page_size)
            .load::<NovelProject>(&mut conn)
            .map_err(|e| e.to_string())?;

        Ok((items, total))
    }

    pub fn get_project(&self, project_id: &str, user_id: i32) -> Result<NovelProject, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;
        gm_novel_projects::table
            .filter(gm_novel_projects::project_id.eq(project_id))
            .filter(gm_novel_projects::user_id.eq(user_id))
            .filter(gm_novel_projects::deleted_at.is_null())
            .first::<NovelProject>(&mut conn)
            .map_err(|e| format!("Project not found: {e}"))
    }

    pub fn update_project(
        &self,
        project_id: &str,
        user_id: i32,
        req: &NovelProjectUpdateRequest,
    ) -> Result<NovelProject, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;

        let current = gm_novel_projects::table
            .filter(gm_novel_projects::project_id.eq(project_id))
            .filter(gm_novel_projects::user_id.eq(user_id))
            .filter(gm_novel_projects::deleted_at.is_null())
            .first::<NovelProject>(&mut conn)
            .map_err(|e| format!("Project not found: {e}"))?;

        diesel::update(
            gm_novel_projects::table
                .filter(gm_novel_projects::project_id.eq(project_id)),
        )
        .set((
            gm_novel_projects::title
                .eq(req.title.as_deref().unwrap_or(&current.title)),
            gm_novel_projects::description
                .eq(req.description.as_deref().unwrap_or(&current.description)),
            gm_novel_projects::topic
                .eq(req.topic.as_deref().unwrap_or(&current.topic)),
            gm_novel_projects::genre
                .eq(req.genre.as_deref().unwrap_or(&current.genre)),
            gm_novel_projects::num_chapters
                .eq(req.num_chapters.unwrap_or(current.num_chapters)),
            gm_novel_projects::target_words_per_chapter
                .eq(req.target_words_per_chapter.unwrap_or(current.target_words_per_chapter)),
            gm_novel_projects::default_user_guidance.eq(req
                .default_user_guidance
                .as_deref()
                .or(current.default_user_guidance.as_deref())),
            gm_novel_projects::updated_at.eq(Utc::now()),
        ))
        .get_result::<NovelProject>(&mut conn)
        .map_err(|e| e.to_string())
    }

    pub fn delete_project(&self, project_id: &str, user_id: i32) -> Result<(), String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;
        let affected = diesel::update(
            gm_novel_projects::table
                .filter(gm_novel_projects::project_id.eq(project_id))
                .filter(gm_novel_projects::user_id.eq(user_id))
                .filter(gm_novel_projects::deleted_at.is_null()),
        )
        .set((
            gm_novel_projects::deleted_at.eq(Some(Utc::now())),
            gm_novel_projects::updated_at.eq(Utc::now()),
        ))
        .execute(&mut conn)
        .map_err(|e| e.to_string())?;

        if affected == 0 {
            return Err("Project not found".to_string());
        }
        Ok(())
    }

    pub fn cancel_project(&self, project_id: &str, user_id: i32) -> Result<(), String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;

        gm_novel_projects::table
            .filter(gm_novel_projects::project_id.eq(project_id))
            .filter(gm_novel_projects::user_id.eq(user_id))
            .filter(gm_novel_projects::deleted_at.is_null())
            .first::<NovelProject>(&mut conn)
            .map_err(|e| format!("Project not found: {e}"))?;

        diesel::update(
            gm_novel_jobs::table
                .filter(gm_novel_jobs::project_id.eq(project_id))
                .filter(gm_novel_jobs::status.eq_any(&["pending", "running"])),
        )
        .set((
            gm_novel_jobs::status.eq("cancel_requested"),
            gm_novel_jobs::updated_at.eq(Utc::now()),
        ))
        .execute(&mut conn)
        .map_err(|e| e.to_string())?;

        diesel::update(
            gm_novel_projects::table
                .filter(gm_novel_projects::project_id.eq(project_id))
                .filter(gm_novel_projects::deleted_at.is_null()),
        )
        .set((
            gm_novel_projects::status.eq("cancelled"),
            gm_novel_projects::pending_stage.eq(None::<String>),
            gm_novel_projects::last_error_message.eq(None::<String>),
            gm_novel_projects::updated_at.eq(Utc::now()),
        ))
        .execute(&mut conn)
        .map_err(|e| e.to_string())?;

        Ok(())
    }

    // =========================================================================
    // Config snapshot methods
    // =========================================================================

    pub fn create_config_snapshot(
        &self,
        project_id: &str,
        req: &NovelConfigSnapshotCreateRequest,
    ) -> Result<NovelProjectConfigSnapshot, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;
        conn.transaction::<_, diesel::result::Error, _>(|conn| {
            diesel::update(
                gm_novel_project_config_snapshots::table
                    .filter(gm_novel_project_config_snapshots::project_id.eq(project_id)),
            )
            .set(gm_novel_project_config_snapshots::is_current.eq(false))
            .execute(conn)?;

            let snap = NewNovelProjectConfigSnapshot {
                project_id: project_id.to_string(),
                architecture_llm_profile_id: req.architecture_llm_profile_id,
                chapter_outline_llm_profile_id: req.chapter_outline_llm_profile_id,
                prompt_draft_llm_profile_id: req.prompt_draft_llm_profile_id,
                final_chapter_llm_profile_id: req.final_chapter_llm_profile_id,
                consistency_review_llm_profile_id: req.consistency_review_llm_profile_id,
                embedding_profile_id: req.embedding_profile_id,
                proxy_setting: req.proxy_setting.clone(),
                webdav_config: req.webdav_config.clone(),
                other_params: req.other_params.clone(),
                is_current: true,
            };

            diesel::insert_into(gm_novel_project_config_snapshots::table)
                .values(&snap)
                .get_result::<NovelProjectConfigSnapshot>(conn)
        })
        .map_err(|e| e.to_string())
    }

    pub fn list_config_snapshots(
        &self,
        project_id: &str,
    ) -> Result<Vec<NovelProjectConfigSnapshot>, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;
        gm_novel_project_config_snapshots::table
            .filter(gm_novel_project_config_snapshots::project_id.eq(project_id))
            .order(gm_novel_project_config_snapshots::created_at.desc())
            .load::<NovelProjectConfigSnapshot>(&mut conn)
            .map_err(|e| e.to_string())
    }

    pub fn activate_config_snapshot(
        &self,
        project_id: &str,
        snapshot_id: i64,
    ) -> Result<(), String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;
        conn.transaction::<_, diesel::result::Error, _>(|conn| {
            diesel::update(
                gm_novel_project_config_snapshots::table
                    .filter(gm_novel_project_config_snapshots::project_id.eq(project_id)),
            )
            .set(gm_novel_project_config_snapshots::is_current.eq(false))
            .execute(conn)?;

            let affected = diesel::update(
                gm_novel_project_config_snapshots::table
                    .filter(gm_novel_project_config_snapshots::id.eq(snapshot_id))
                    .filter(gm_novel_project_config_snapshots::project_id.eq(project_id)),
            )
            .set(gm_novel_project_config_snapshots::is_current.eq(true))
            .execute(conn)?;

            if affected == 0 {
                return Err(diesel::result::Error::NotFound);
            }
            Ok(())
        })
        .map_err(|e| e.to_string())
    }

    // =========================================================================
    // LLM profile methods
    // =========================================================================

    pub fn list_llm_profiles(&self, user_id: i32) -> Result<Vec<NovelLlmProfile>, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;
        gm_novel_llm_profiles::table
            .filter(gm_novel_llm_profiles::user_id.eq(user_id))
            .filter(gm_novel_llm_profiles::deleted_at.is_null())
            .order(gm_novel_llm_profiles::created_at.desc())
            .load::<NovelLlmProfile>(&mut conn)
            .map_err(|e| e.to_string())
    }

    pub fn get_llm_profile(&self, id: i64, user_id: i32) -> Result<NovelLlmProfile, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;
        gm_novel_llm_profiles::table
            .filter(gm_novel_llm_profiles::id.eq(id))
            .filter(gm_novel_llm_profiles::user_id.eq(user_id))
            .filter(gm_novel_llm_profiles::deleted_at.is_null())
            .first::<NovelLlmProfile>(&mut conn)
            .map_err(|e| format!("LLM profile not found: {e}"))
    }

    pub fn create_llm_profile(
        &self,
        user_id: i32,
        req: &NovelLlmProfileCreateRequest,
    ) -> Result<NovelLlmProfile, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;

        let base_url = if req.base_url.trim().is_empty() {
            std::env::var("LAOZHANG_BASE_URL")
                .or_else(|_| std::env::var("OPENAI_BASE_URL"))
                .unwrap_or_default()
        } else {
            req.base_url.clone()
        };
        let api_key = if req.api_key.trim().is_empty() {
            std::env::var("LAOZHANG_API_KEY")
                .or_else(|_| std::env::var("OPENAI_API_KEY"))
                .unwrap_or_default()
        } else {
            req.api_key.clone()
        };

        let existing = gm_novel_llm_profiles::table
            .filter(gm_novel_llm_profiles::user_id.eq(user_id))
            .filter(gm_novel_llm_profiles::name.eq(&req.name))
            .first::<NovelLlmProfile>(&mut conn)
            .optional()
            .map_err(|e| e.to_string())?;

        if let Some(row) = existing {
            return diesel::update(
                gm_novel_llm_profiles::table.filter(gm_novel_llm_profiles::id.eq(row.id)),
            )
            .set((
                gm_novel_llm_profiles::interface_format.eq(&req.interface_format),
                gm_novel_llm_profiles::base_url.eq(&base_url),
                gm_novel_llm_profiles::api_key.eq(&api_key),
                gm_novel_llm_profiles::model_name.eq(&req.model_name),
                gm_novel_llm_profiles::temperature.eq(req.temperature),
                gm_novel_llm_profiles::max_tokens.eq(req.max_tokens),
                gm_novel_llm_profiles::timeout_seconds.eq(req.timeout_seconds),
                gm_novel_llm_profiles::is_active.eq(true),
                gm_novel_llm_profiles::deleted_at.eq(None::<DateTime<Utc>>),
                gm_novel_llm_profiles::updated_at.eq(Utc::now()),
            ))
            .get_result::<NovelLlmProfile>(&mut conn)
            .map_err(|e| e.to_string());
        }

        let new = NewNovelLlmProfile {
            user_id,
            name: req.name.clone(),
            interface_format: req.interface_format.clone(),
            base_url,
            api_key,
            model_name: req.model_name.clone(),
            temperature: req.temperature,
            max_tokens: req.max_tokens,
            timeout_seconds: req.timeout_seconds,
            is_default: false,
            is_active: true,
            metadata: json!({}),
        };
        diesel::insert_into(gm_novel_llm_profiles::table)
            .values(&new)
            .get_result::<NovelLlmProfile>(&mut conn)
            .map_err(|e| e.to_string())
    }

    pub fn update_llm_profile(
        &self,
        id: i64,
        user_id: i32,
        req: &NovelLlmProfileUpdateRequest,
    ) -> Result<NovelLlmProfile, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;
        let current = gm_novel_llm_profiles::table
            .filter(gm_novel_llm_profiles::id.eq(id))
            .filter(gm_novel_llm_profiles::user_id.eq(user_id))
            .filter(gm_novel_llm_profiles::deleted_at.is_null())
            .first::<NovelLlmProfile>(&mut conn)
            .map_err(|e| format!("LLM profile not found: {e}"))?;

        diesel::update(gm_novel_llm_profiles::table.filter(gm_novel_llm_profiles::id.eq(id)))
            .set((
                gm_novel_llm_profiles::name
                    .eq(req.name.as_deref().unwrap_or(&current.name)),
                gm_novel_llm_profiles::interface_format
                    .eq(req.interface_format.as_deref().unwrap_or(&current.interface_format)),
                gm_novel_llm_profiles::base_url
                    .eq(req.base_url.as_deref().unwrap_or(&current.base_url)),
                gm_novel_llm_profiles::api_key
                    .eq(req.api_key.as_deref().unwrap_or(&current.api_key)),
                gm_novel_llm_profiles::model_name
                    .eq(req.model_name.as_deref().unwrap_or(&current.model_name)),
                gm_novel_llm_profiles::temperature
                    .eq(req.temperature.unwrap_or(current.temperature)),
                gm_novel_llm_profiles::max_tokens
                    .eq(req.max_tokens.unwrap_or(current.max_tokens)),
                gm_novel_llm_profiles::timeout_seconds
                    .eq(req.timeout_seconds.unwrap_or(current.timeout_seconds)),
                gm_novel_llm_profiles::updated_at.eq(Utc::now()),
            ))
            .get_result::<NovelLlmProfile>(&mut conn)
            .map_err(|e| e.to_string())
    }

    pub fn delete_llm_profile(&self, id: i64, user_id: i32) -> Result<(), String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;
        let affected = diesel::update(
            gm_novel_llm_profiles::table
                .filter(gm_novel_llm_profiles::id.eq(id))
                .filter(gm_novel_llm_profiles::user_id.eq(user_id))
                .filter(gm_novel_llm_profiles::deleted_at.is_null()),
        )
        .set((
            gm_novel_llm_profiles::deleted_at.eq(Some(Utc::now())),
            gm_novel_llm_profiles::updated_at.eq(Utc::now()),
        ))
        .execute(&mut conn)
        .map_err(|e| e.to_string())?;

        if affected == 0 {
            return Err("LLM profile not found".to_string());
        }
        Ok(())
    }

    // =========================================================================
    // Embedding profile methods
    // =========================================================================

    pub fn list_embedding_profiles(
        &self,
        user_id: i32,
    ) -> Result<Vec<NovelEmbeddingProfile>, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;
        gm_novel_embedding_profiles::table
            .filter(gm_novel_embedding_profiles::user_id.eq(user_id))
            .filter(gm_novel_embedding_profiles::deleted_at.is_null())
            .order(gm_novel_embedding_profiles::created_at.desc())
            .load::<NovelEmbeddingProfile>(&mut conn)
            .map_err(|e| e.to_string())
    }

    pub fn get_embedding_profile(
        &self,
        id: i64,
        user_id: i32,
    ) -> Result<NovelEmbeddingProfile, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;
        gm_novel_embedding_profiles::table
            .filter(gm_novel_embedding_profiles::id.eq(id))
            .filter(gm_novel_embedding_profiles::user_id.eq(user_id))
            .filter(gm_novel_embedding_profiles::deleted_at.is_null())
            .first::<NovelEmbeddingProfile>(&mut conn)
            .map_err(|e| format!("Embedding profile not found: {e}"))
    }

    pub fn create_embedding_profile(
        &self,
        user_id: i32,
        req: &NovelEmbeddingProfileCreateRequest,
    ) -> Result<NovelEmbeddingProfile, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;

        let existing = gm_novel_embedding_profiles::table
            .filter(gm_novel_embedding_profiles::user_id.eq(user_id))
            .filter(gm_novel_embedding_profiles::name.eq(&req.name))
            .first::<NovelEmbeddingProfile>(&mut conn)
            .optional()
            .map_err(|e| e.to_string())?;

        if let Some(row) = existing {
            return diesel::update(
                gm_novel_embedding_profiles::table
                    .filter(gm_novel_embedding_profiles::id.eq(row.id)),
            )
            .set((
                gm_novel_embedding_profiles::interface_format.eq(&req.interface_format),
                gm_novel_embedding_profiles::base_url.eq(&req.base_url),
                gm_novel_embedding_profiles::api_key.eq(&req.api_key),
                gm_novel_embedding_profiles::model_name.eq(&req.model_name),
                gm_novel_embedding_profiles::retrieval_k.eq(req.retrieval_k),
                gm_novel_embedding_profiles::is_active.eq(true),
                gm_novel_embedding_profiles::deleted_at.eq(None::<DateTime<Utc>>),
                gm_novel_embedding_profiles::updated_at.eq(Utc::now()),
            ))
            .get_result::<NovelEmbeddingProfile>(&mut conn)
            .map_err(|e| e.to_string());
        }

        let new = NewNovelEmbeddingProfile {
            user_id,
            name: req.name.clone(),
            interface_format: req.interface_format.clone(),
            base_url: req.base_url.clone(),
            api_key: req.api_key.clone(),
            model_name: req.model_name.clone(),
            retrieval_k: req.retrieval_k,
            is_default: false,
            is_active: true,
            metadata: json!({}),
        };
        diesel::insert_into(gm_novel_embedding_profiles::table)
            .values(&new)
            .get_result::<NovelEmbeddingProfile>(&mut conn)
            .map_err(|e| e.to_string())
    }

    pub fn update_embedding_profile(
        &self,
        id: i64,
        user_id: i32,
        req: &NovelEmbeddingProfileUpdateRequest,
    ) -> Result<NovelEmbeddingProfile, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;
        let current = gm_novel_embedding_profiles::table
            .filter(gm_novel_embedding_profiles::id.eq(id))
            .filter(gm_novel_embedding_profiles::user_id.eq(user_id))
            .filter(gm_novel_embedding_profiles::deleted_at.is_null())
            .first::<NovelEmbeddingProfile>(&mut conn)
            .map_err(|e| format!("Embedding profile not found: {e}"))?;

        diesel::update(
            gm_novel_embedding_profiles::table
                .filter(gm_novel_embedding_profiles::id.eq(id)),
        )
        .set((
            gm_novel_embedding_profiles::name
                .eq(req.name.as_deref().unwrap_or(&current.name)),
            gm_novel_embedding_profiles::interface_format
                .eq(req.interface_format.as_deref().unwrap_or(&current.interface_format)),
            gm_novel_embedding_profiles::base_url
                .eq(req.base_url.as_deref().unwrap_or(&current.base_url)),
            gm_novel_embedding_profiles::api_key
                .eq(req.api_key.as_deref().unwrap_or(&current.api_key)),
            gm_novel_embedding_profiles::model_name
                .eq(req.model_name.as_deref().unwrap_or(&current.model_name)),
            gm_novel_embedding_profiles::retrieval_k
                .eq(req.retrieval_k.unwrap_or(current.retrieval_k)),
            gm_novel_embedding_profiles::updated_at.eq(Utc::now()),
        ))
        .get_result::<NovelEmbeddingProfile>(&mut conn)
        .map_err(|e| e.to_string())
    }

    pub fn delete_embedding_profile(&self, id: i64, user_id: i32) -> Result<(), String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;
        let affected = diesel::update(
            gm_novel_embedding_profiles::table
                .filter(gm_novel_embedding_profiles::id.eq(id))
                .filter(gm_novel_embedding_profiles::user_id.eq(user_id))
                .filter(gm_novel_embedding_profiles::deleted_at.is_null()),
        )
        .set((
            gm_novel_embedding_profiles::deleted_at.eq(Some(Utc::now())),
            gm_novel_embedding_profiles::updated_at.eq(Utc::now()),
        ))
        .execute(&mut conn)
        .map_err(|e| e.to_string())?;

        if affected == 0 {
            return Err("Embedding profile not found".to_string());
        }
        Ok(())
    }

    // =========================================================================
    // Architecture methods
    // =========================================================================

    pub fn get_architecture(
        &self,
        project_id: &str,
    ) -> Result<Option<NovelArchitecture>, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;
        gm_novel_architectures::table
            .filter(gm_novel_architectures::project_id.eq(project_id))
            .filter(gm_novel_architectures::is_current.eq(true))
            .first::<NovelArchitecture>(&mut conn)
            .optional()
            .map_err(|e| e.to_string())
    }

    pub fn update_architecture(
        &self,
        project_id: &str,
        req: &NovelArchitectureUpdateRequest,
    ) -> Result<NovelArchitecture, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;
        let current = gm_novel_architectures::table
            .filter(gm_novel_architectures::project_id.eq(project_id))
            .filter(gm_novel_architectures::is_current.eq(true))
            .first::<NovelArchitecture>(&mut conn)
            .map_err(|e| format!("Architecture not found: {e}"))?;

        diesel::update(
            gm_novel_architectures::table.filter(gm_novel_architectures::id.eq(current.id)),
        )
        .set((
            gm_novel_architectures::core_seed_text
                .eq(req.core_seed_text.as_deref().unwrap_or(&current.core_seed_text)),
            gm_novel_architectures::character_dynamics_text.eq(req
                .character_dynamics_text
                .as_deref()
                .unwrap_or(&current.character_dynamics_text)),
            gm_novel_architectures::world_building_text.eq(req
                .world_building_text
                .as_deref()
                .unwrap_or(&current.world_building_text)),
            gm_novel_architectures::plot_architecture_text.eq(req
                .plot_architecture_text
                .as_deref()
                .unwrap_or(&current.plot_architecture_text)),
            gm_novel_architectures::full_text
                .eq(req.full_text.as_deref().unwrap_or(&current.full_text)),
            gm_novel_architectures::updated_at.eq(Utc::now()),
        ))
        .get_result::<NovelArchitecture>(&mut conn)
        .map_err(|e| e.to_string())
    }

    // =========================================================================
    // State snapshot methods
    // =========================================================================

    pub fn get_character_state(
        &self,
        project_id: &str,
    ) -> Result<Option<NovelCharacterStateSnapshot>, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;
        gm_novel_character_state_snapshots::table
            .filter(gm_novel_character_state_snapshots::project_id.eq(project_id))
            .filter(gm_novel_character_state_snapshots::is_current.eq(true))
            .first::<NovelCharacterStateSnapshot>(&mut conn)
            .optional()
            .map_err(|e| e.to_string())
    }

    pub fn update_character_state(
        &self,
        project_id: &str,
        text: &str,
    ) -> Result<NovelCharacterStateSnapshot, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;
        conn.transaction::<_, diesel::result::Error, _>(|conn| {
            let max_ver: Option<i32> = gm_novel_character_state_snapshots::table
                .filter(gm_novel_character_state_snapshots::project_id.eq(project_id))
                .select(max(gm_novel_character_state_snapshots::version_no))
                .first::<Option<i32>>(conn)?;

            diesel::update(
                gm_novel_character_state_snapshots::table
                    .filter(gm_novel_character_state_snapshots::project_id.eq(project_id)),
            )
            .set(gm_novel_character_state_snapshots::is_current.eq(false))
            .execute(conn)?;

            diesel::insert_into(gm_novel_character_state_snapshots::table)
                .values((
                    gm_novel_character_state_snapshots::project_id.eq(project_id),
                    gm_novel_character_state_snapshots::state_text.eq(text),
                    gm_novel_character_state_snapshots::version_no
                        .eq(max_ver.unwrap_or(0) + 1),
                    gm_novel_character_state_snapshots::is_current.eq(true),
                ))
                .get_result::<NovelCharacterStateSnapshot>(conn)
        })
        .map_err(|e| e.to_string())
    }

    pub fn get_global_summary(
        &self,
        project_id: &str,
    ) -> Result<Option<NovelGlobalSummarySnapshot>, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;
        gm_novel_global_summary_snapshots::table
            .filter(gm_novel_global_summary_snapshots::project_id.eq(project_id))
            .filter(gm_novel_global_summary_snapshots::is_current.eq(true))
            .first::<NovelGlobalSummarySnapshot>(&mut conn)
            .optional()
            .map_err(|e| e.to_string())
    }

    pub fn update_global_summary(
        &self,
        project_id: &str,
        text: &str,
    ) -> Result<NovelGlobalSummarySnapshot, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;
        conn.transaction::<_, diesel::result::Error, _>(|conn| {
            let max_ver: Option<i32> = gm_novel_global_summary_snapshots::table
                .filter(gm_novel_global_summary_snapshots::project_id.eq(project_id))
                .select(max(gm_novel_global_summary_snapshots::version_no))
                .first::<Option<i32>>(conn)?;

            diesel::update(
                gm_novel_global_summary_snapshots::table
                    .filter(gm_novel_global_summary_snapshots::project_id.eq(project_id)),
            )
            .set(gm_novel_global_summary_snapshots::is_current.eq(false))
            .execute(conn)?;

            diesel::insert_into(gm_novel_global_summary_snapshots::table)
                .values((
                    gm_novel_global_summary_snapshots::project_id.eq(project_id),
                    gm_novel_global_summary_snapshots::summary_text.eq(text),
                    gm_novel_global_summary_snapshots::version_no
                        .eq(max_ver.unwrap_or(0) + 1),
                    gm_novel_global_summary_snapshots::is_current.eq(true),
                ))
                .get_result::<NovelGlobalSummarySnapshot>(conn)
        })
        .map_err(|e| e.to_string())
    }

    pub fn get_plot_arcs(
        &self,
        project_id: &str,
    ) -> Result<Option<NovelPlotArcSnapshot>, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;
        gm_novel_plot_arc_snapshots::table
            .filter(gm_novel_plot_arc_snapshots::project_id.eq(project_id))
            .filter(gm_novel_plot_arc_snapshots::is_current.eq(true))
            .first::<NovelPlotArcSnapshot>(&mut conn)
            .optional()
            .map_err(|e| e.to_string())
    }

    pub fn update_plot_arcs(
        &self,
        project_id: &str,
        text: &str,
    ) -> Result<NovelPlotArcSnapshot, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;
        conn.transaction::<_, diesel::result::Error, _>(|conn| {
            let max_ver: Option<i32> = gm_novel_plot_arc_snapshots::table
                .filter(gm_novel_plot_arc_snapshots::project_id.eq(project_id))
                .select(max(gm_novel_plot_arc_snapshots::version_no))
                .first::<Option<i32>>(conn)?;

            diesel::update(
                gm_novel_plot_arc_snapshots::table
                    .filter(gm_novel_plot_arc_snapshots::project_id.eq(project_id)),
            )
            .set(gm_novel_plot_arc_snapshots::is_current.eq(false))
            .execute(conn)?;

            diesel::insert_into(gm_novel_plot_arc_snapshots::table)
                .values((
                    gm_novel_plot_arc_snapshots::project_id.eq(project_id),
                    gm_novel_plot_arc_snapshots::plot_arcs_text.eq(text),
                    gm_novel_plot_arc_snapshots::version_no.eq(max_ver.unwrap_or(0) + 1),
                    gm_novel_plot_arc_snapshots::is_current.eq(true),
                ))
                .get_result::<NovelPlotArcSnapshot>(conn)
        })
        .map_err(|e| e.to_string())
    }

    // =========================================================================
    // Blueprint methods
    // =========================================================================

    pub fn get_blueprint(
        &self,
        project_id: &str,
    ) -> Result<Option<NovelBlueprint>, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;
        gm_novel_blueprints::table
            .filter(gm_novel_blueprints::project_id.eq(project_id))
            .filter(gm_novel_blueprints::is_current.eq(true))
            .first::<NovelBlueprint>(&mut conn)
            .optional()
            .map_err(|e| e.to_string())
    }

    pub fn update_blueprint(
        &self,
        project_id: &str,
        raw_text: &str,
    ) -> Result<NovelBlueprint, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;

        let bp: NovelBlueprint = diesel::update(
            gm_novel_blueprints::table
                .filter(gm_novel_blueprints::project_id.eq(project_id))
                .filter(gm_novel_blueprints::is_current.eq(true)),
        )
        .set((
            gm_novel_blueprints::raw_text.eq(raw_text),
            gm_novel_blueprints::updated_at.eq(Utc::now()),
        ))
        .get_result::<NovelBlueprint>(&mut conn)
        .map_err(|e| format!("Blueprint not found: {e}"))?;

        diesel::delete(
            gm_novel_blueprint_chapters::table
                .filter(gm_novel_blueprint_chapters::blueprint_id.eq(bp.id)),
        )
        .execute(&mut conn)
        .map_err(|e| format!("Failed to delete old chapters: {e}"))?;

        let parsed = Self::parse_blueprint_chapters(raw_text);
        for ch in &parsed {
            diesel::insert_into(gm_novel_blueprint_chapters::table)
                .values((
                    gm_novel_blueprint_chapters::blueprint_id.eq(bp.id),
                    gm_novel_blueprint_chapters::project_id.eq(project_id),
                    gm_novel_blueprint_chapters::chapter_number.eq(ch.chapter_number),
                    gm_novel_blueprint_chapters::chapter_title.eq(&ch.chapter_title),
                    gm_novel_blueprint_chapters::chapter_role.eq(&ch.chapter_role),
                    gm_novel_blueprint_chapters::chapter_purpose.eq(&ch.chapter_purpose),
                    gm_novel_blueprint_chapters::suspense_level.eq(&ch.suspense_level),
                    gm_novel_blueprint_chapters::foreshadowing.eq(&ch.foreshadowing),
                    gm_novel_blueprint_chapters::plot_twist_level.eq(&ch.plot_twist_level),
                    gm_novel_blueprint_chapters::chapter_summary.eq(&ch.chapter_summary),
                ))
                .execute(&mut conn)
                .map_err(|e| format!("Failed to insert parsed chapter: {e}"))?;
        }

        Ok(bp)
    }

    pub fn list_blueprint_chapters(
        &self,
        project_id: &str,
    ) -> Result<Vec<NovelBlueprintChapter>, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;
        gm_novel_blueprint_chapters::table
            .filter(gm_novel_blueprint_chapters::project_id.eq(project_id))
            .order(gm_novel_blueprint_chapters::chapter_number.asc())
            .load::<NovelBlueprintChapter>(&mut conn)
            .map_err(|e| e.to_string())
    }

    pub fn get_blueprint_chapter(
        &self,
        project_id: &str,
        chapter_number: i32,
    ) -> Result<Option<NovelBlueprintChapter>, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;
        gm_novel_blueprint_chapters::table
            .filter(gm_novel_blueprint_chapters::project_id.eq(project_id))
            .filter(gm_novel_blueprint_chapters::chapter_number.eq(chapter_number))
            .order(gm_novel_blueprint_chapters::id.desc())
            .first::<NovelBlueprintChapter>(&mut conn)
            .optional()
            .map_err(|e| e.to_string())
    }

    pub fn update_blueprint_chapter(
        &self,
        project_id: &str,
        chapter_number: i32,
        req: &NovelBlueprintChapterUpdateRequest,
    ) -> Result<NovelBlueprintChapter, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;
        let current = gm_novel_blueprint_chapters::table
            .filter(gm_novel_blueprint_chapters::project_id.eq(project_id))
            .filter(gm_novel_blueprint_chapters::chapter_number.eq(chapter_number))
            .order(gm_novel_blueprint_chapters::id.desc())
            .first::<NovelBlueprintChapter>(&mut conn)
            .map_err(|e| format!("Blueprint chapter not found: {e}"))?;

        diesel::update(
            gm_novel_blueprint_chapters::table
                .filter(gm_novel_blueprint_chapters::id.eq(current.id)),
        )
        .set((
            gm_novel_blueprint_chapters::chapter_title
                .eq(req.chapter_title.as_deref().unwrap_or(&current.chapter_title)),
            gm_novel_blueprint_chapters::chapter_role
                .eq(req.chapter_role.as_deref().unwrap_or(&current.chapter_role)),
            gm_novel_blueprint_chapters::chapter_purpose
                .eq(req.chapter_purpose.as_deref().unwrap_or(&current.chapter_purpose)),
            gm_novel_blueprint_chapters::suspense_level
                .eq(req.suspense_level.as_deref().unwrap_or(&current.suspense_level)),
            gm_novel_blueprint_chapters::foreshadowing
                .eq(req.foreshadowing.as_deref().unwrap_or(&current.foreshadowing)),
            gm_novel_blueprint_chapters::plot_twist_level
                .eq(req.plot_twist_level.as_deref().unwrap_or(&current.plot_twist_level)),
            gm_novel_blueprint_chapters::chapter_summary
                .eq(req.chapter_summary.as_deref().unwrap_or(&current.chapter_summary)),
            gm_novel_blueprint_chapters::updated_at.eq(Utc::now()),
        ))
        .get_result::<NovelBlueprintChapter>(&mut conn)
        .map_err(|e| e.to_string())
    }

    // =========================================================================
    // Chapter methods
    // =========================================================================

    pub fn list_chapters(
        &self,
        project_id: &str,
        status: Option<&str>,
    ) -> Result<Vec<NovelChapter>, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;
        let mut query = gm_novel_chapters::table
            .filter(gm_novel_chapters::project_id.eq(project_id))
            .into_boxed();
        if let Some(s) = status {
            query = query.filter(gm_novel_chapters::status.eq(s));
        }
        query
            .order(gm_novel_chapters::chapter_number.asc())
            .load::<NovelChapter>(&mut conn)
            .map_err(|e| e.to_string())
    }

    pub fn get_chapter(
        &self,
        project_id: &str,
        chapter_number: i32,
    ) -> Result<Option<NovelChapter>, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;
        gm_novel_chapters::table
            .filter(gm_novel_chapters::project_id.eq(project_id))
            .filter(gm_novel_chapters::chapter_number.eq(chapter_number))
            .first::<NovelChapter>(&mut conn)
            .optional()
            .map_err(|e| e.to_string())
    }

    pub fn update_chapter(
        &self,
        project_id: &str,
        chapter_number: i32,
        req: &NovelChapterUpdateRequest,
    ) -> Result<NovelChapter, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;
        let current = gm_novel_chapters::table
            .filter(gm_novel_chapters::project_id.eq(project_id))
            .filter(gm_novel_chapters::chapter_number.eq(chapter_number))
            .first::<NovelChapter>(&mut conn)
            .map_err(|e| format!("Chapter not found: {e}"))?;

        diesel::update(
            gm_novel_chapters::table.filter(gm_novel_chapters::id.eq(current.id)),
        )
        .set((
            gm_novel_chapters::draft_text
                .eq(req.draft_text.as_deref().or(current.draft_text.as_deref())),
            gm_novel_chapters::final_text
                .eq(req.final_text.as_deref().or(current.final_text.as_deref())),
            gm_novel_chapters::status
                .eq(req.status.as_deref().unwrap_or(&current.status)),
            gm_novel_chapters::updated_at.eq(Utc::now()),
        ))
        .get_result::<NovelChapter>(&mut conn)
        .map_err(|e| e.to_string())
    }

    pub fn get_chapter_prompt(
        &self,
        project_id: &str,
        chapter_number: i32,
    ) -> Result<Option<NovelChapterPrompt>, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;
        gm_novel_chapter_prompts::table
            .filter(gm_novel_chapter_prompts::project_id.eq(project_id))
            .filter(gm_novel_chapter_prompts::chapter_number.eq(chapter_number))
            .filter(gm_novel_chapter_prompts::is_current.eq(true))
            .first::<NovelChapterPrompt>(&mut conn)
            .optional()
            .map_err(|e| e.to_string())
    }

    pub fn update_chapter_prompt(
        &self,
        project_id: &str,
        chapter_number: i32,
        text: &str,
    ) -> Result<NovelChapterPrompt, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;
        diesel::update(
            gm_novel_chapter_prompts::table
                .filter(gm_novel_chapter_prompts::project_id.eq(project_id))
                .filter(gm_novel_chapter_prompts::chapter_number.eq(chapter_number))
                .filter(gm_novel_chapter_prompts::is_current.eq(true)),
        )
        .set((
            gm_novel_chapter_prompts::edited_prompt_text.eq(Some(text)),
            gm_novel_chapter_prompts::updated_at.eq(Utc::now()),
        ))
        .get_result::<NovelChapterPrompt>(&mut conn)
        .map_err(|e| format!("Chapter prompt not found: {e}"))
    }

    // =========================================================================
    // Consistency check methods
    // =========================================================================

    pub fn list_consistency_checks(
        &self,
        project_id: &str,
        chapter_number: i32,
    ) -> Result<Vec<NovelConsistencyCheck>, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;
        gm_novel_consistency_checks::table
            .filter(gm_novel_consistency_checks::project_id.eq(project_id))
            .filter(gm_novel_consistency_checks::chapter_number.eq(chapter_number))
            .order(gm_novel_consistency_checks::created_at.desc())
            .load::<NovelConsistencyCheck>(&mut conn)
            .map_err(|e| e.to_string())
    }

    pub fn get_latest_consistency_check(
        &self,
        project_id: &str,
        chapter_number: i32,
    ) -> Result<Option<NovelConsistencyCheck>, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;
        gm_novel_consistency_checks::table
            .filter(gm_novel_consistency_checks::project_id.eq(project_id))
            .filter(gm_novel_consistency_checks::chapter_number.eq(chapter_number))
            .order(gm_novel_consistency_checks::created_at.desc())
            .first::<NovelConsistencyCheck>(&mut conn)
            .optional()
            .map_err(|e| e.to_string())
    }

    // =========================================================================
    // Knowledge methods
    // =========================================================================

    pub fn list_knowledge_imports(
        &self,
        project_id: &str,
    ) -> Result<Vec<NovelKnowledgeImport>, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;
        gm_novel_knowledge_imports::table
            .filter(gm_novel_knowledge_imports::project_id.eq(project_id))
            .order(gm_novel_knowledge_imports::created_at.desc())
            .load::<NovelKnowledgeImport>(&mut conn)
            .map_err(|e| e.to_string())
    }

    pub fn get_config_snapshot(
        &self,
        project_id: &str,
        snapshot_id: i64,
    ) -> Result<NovelProjectConfigSnapshot, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;
        gm_novel_project_config_snapshots::table
            .filter(gm_novel_project_config_snapshots::project_id.eq(project_id))
            .filter(gm_novel_project_config_snapshots::id.eq(snapshot_id))
            .first::<NovelProjectConfigSnapshot>(&mut conn)
            .map_err(|e| format!("Config snapshot not found: {e}"))
    }

    pub fn create_knowledge_import(
        &self,
        project_id: &str,
        source_name: String,
        original_text: String,
    ) -> Result<NovelKnowledgeImport, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;
        let row = NewNovelKnowledgeImport {
            project_id: project_id.to_string(),
            source_name,
            source_type: "text_file".to_string(),
            original_text,
            segment_count: 0,
            status: "pending".to_string(),
            source_stage_run_id: None,
        };
        diesel::insert_into(gm_novel_knowledge_imports::table)
            .values(&row)
            .get_result::<NovelKnowledgeImport>(&mut conn)
            .map_err(|e| e.to_string())
    }

    pub fn list_knowledge_chunks(
        &self,
        project_id: &str,
        import_id: Option<i64>,
    ) -> Result<Vec<NovelKnowledgeChunk>, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;
        let mut query = gm_novel_knowledge_chunks::table
            .filter(gm_novel_knowledge_chunks::project_id.eq(project_id))
            .into_boxed();

        if let Some(id) = import_id {
            query = query.filter(gm_novel_knowledge_chunks::knowledge_import_id.eq(id));
        }

        query
            .order(gm_novel_knowledge_chunks::chunk_index.asc())
            .load::<NovelKnowledgeChunk>(&mut conn)
            .map_err(|e| e.to_string())
    }

    // =========================================================================
    // Job methods
    // =========================================================================

    pub fn create_job(
        &self,
        project_id: &str,
        user_id: i32,
        stage_code: &str,
        task_type: &str,
        chapter_number: Option<i32>,
        request_payload: serde_json::Value,
    ) -> Result<NovelJob, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;
        let new_job = NewNovelJob {
            project_id: project_id.to_string(),
            chapter_number,
            stage_code: stage_code.to_string(),
            task_type: task_type.to_string(),
            status: "pending".to_string(),
            idempotency_key: None,
            request_payload,
            created_by: Some(user_id),
        };
        diesel::insert_into(gm_novel_jobs::table)
            .values(&new_job)
            .get_result::<NovelJob>(&mut conn)
            .map_err(|e| e.to_string())
    }

    pub fn list_jobs(&self, project_id: &str) -> Result<Vec<NovelJob>, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;
        gm_novel_jobs::table
            .filter(gm_novel_jobs::project_id.eq(project_id))
            .order(gm_novel_jobs::created_at.desc())
            .load::<NovelJob>(&mut conn)
            .map_err(|e| e.to_string())
    }

    pub fn get_job(
        &self,
        project_id: &str,
        job_id: i64,
    ) -> Result<Option<NovelJob>, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;
        gm_novel_jobs::table
            .filter(gm_novel_jobs::id.eq(job_id))
            .filter(gm_novel_jobs::project_id.eq(project_id))
            .first::<NovelJob>(&mut conn)
            .optional()
            .map_err(|e| e.to_string())
    }

    pub fn list_stage_runs(&self, job_id: i64) -> Result<Vec<NovelStageRun>, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;
        gm_novel_stage_runs::table
            .filter(gm_novel_stage_runs::job_id.eq(job_id))
            .order(gm_novel_stage_runs::created_at.asc())
            .load::<NovelStageRun>(&mut conn)
            .map_err(|e| e.to_string())
    }

    // =========================================================================
    // Job status update + project progress sync
    // =========================================================================

    pub fn update_job_status(
        &self,
        job_id: i64,
        status: &str,
        result_payload: Option<serde_json::Value>,
        error_payload: Option<serde_json::Value>,
    ) -> Result<NovelJob, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;
        let now = Utc::now();

        let current = gm_novel_jobs::table
            .filter(gm_novel_jobs::id.eq(job_id))
            .first::<NovelJob>(&mut conn)
            .map_err(|e| format!("job not found: {e}"))?;

        let started_at = if status == "running" && current.started_at.is_none() {
            Some(now)
        } else {
            current.started_at
        };
        let completed_at =
            if matches!(status, "completed" | "failed" | "partial_failed" | "cancelled") {
                Some(now)
            } else {
                current.completed_at
            };

        diesel::update(gm_novel_jobs::table.filter(gm_novel_jobs::id.eq(job_id)))
            .set((
                gm_novel_jobs::status.eq(status),
                gm_novel_jobs::result_payload
                    .eq(result_payload.unwrap_or(current.result_payload)),
                gm_novel_jobs::error_payload
                    .eq(error_payload.unwrap_or(current.error_payload)),
                gm_novel_jobs::started_at.eq(started_at),
                gm_novel_jobs::completed_at.eq(completed_at),
                gm_novel_jobs::updated_at.eq(now),
            ))
            .get_result::<NovelJob>(&mut conn)
            .map_err(|e| format!("update job status: {e}"))
    }

    pub fn sync_project_from_job(&self, job: &NovelJob) -> Result<(), String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;

        let project_status = match job.status.as_str() {
            "running" => "in_progress",
            "completed" | "partial_failed" => "in_progress",
            "failed" => "in_progress",
            _ => return Ok(()),
        };

        let all_jobs: Vec<NovelJob> = gm_novel_jobs::table
            .filter(gm_novel_jobs::project_id.eq(&job.project_id))
            .filter(gm_novel_jobs::status.ne("cancel_requested"))
            .order(gm_novel_jobs::created_at.desc())
            .load::<NovelJob>(&mut conn)
            .map_err(|e| e.to_string())?;

        let total = all_jobs.len() as f32;
        let completed_count = all_jobs
            .iter()
            .filter(|j| matches!(j.status.as_str(), "completed" | "partial_failed"))
            .count() as f32;
        let has_running = all_jobs.iter().any(|j| j.status == "running" || j.status == "pending");
        let all_failed = !all_jobs.is_empty()
            && all_jobs.iter().all(|j| j.status == "failed" || j.status == "cancelled");

        let progress = if total > 0.0 {
            (completed_count / total * 100.0).min(100.0)
        } else {
            0.0
        };

        let final_status = if all_failed {
            "failed"
        } else if has_running {
            "in_progress"
        } else {
            project_status
        };

        let error_msg = if job.status == "failed" {
            job.error_payload
                .get("error")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or_else(|| Some(format!("Job {} failed", job.id)))
        } else {
            None
        };

        diesel::update(
            gm_novel_projects::table
                .filter(gm_novel_projects::project_id.eq(&job.project_id)),
        )
        .set((
            gm_novel_projects::status.eq(final_status),
            gm_novel_projects::current_stage.eq(Some(&job.stage_code)),
            gm_novel_projects::current_chapter_number.eq(job.chapter_number),
            gm_novel_projects::progress_percent.eq(progress),
            gm_novel_projects::last_error_message.eq(error_msg),
            gm_novel_projects::updated_at.eq(Utc::now()),
        ))
        .execute(&mut conn)
        .map_err(|e| format!("sync project from job: {e}"))?;

        Ok(())
    }

    pub fn mark_project_running(
        &self,
        project_id: &str,
        stage_code: &str,
        chapter_number: Option<i32>,
    ) -> Result<(), String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;
        diesel::update(
            gm_novel_projects::table
                .filter(gm_novel_projects::project_id.eq(project_id)),
        )
        .set((
            gm_novel_projects::status.eq("in_progress"),
            gm_novel_projects::current_stage.eq(Some(stage_code)),
            gm_novel_projects::current_chapter_number.eq(chapter_number),
            gm_novel_projects::last_error_message.eq(None::<String>),
            gm_novel_projects::updated_at.eq(Utc::now()),
        ))
        .execute(&mut conn)
        .map_err(|e| format!("mark project running: {e}"))?;
        Ok(())
    }

    // =========================================================================
    // Stage run + event recording
    // =========================================================================

    pub fn upsert_stage_run(
        &self,
        project_id: &str,
        job_id: i64,
        chapter_number: i32,
        stage_code: &str,
        status: &str,
        input_hash: &str,
        input_payload: serde_json::Value,
        output_payload: Option<serde_json::Value>,
        error_message: Option<&str>,
        attempt_no: i32,
    ) -> Result<NovelStageRun, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;
        let now = Utc::now();

        let started_at = if status == "running" { Some(now) } else { None };
        let completed_at = if matches!(status, "completed" | "failed") {
            Some(now)
        } else {
            None
        };

        let existing: Option<NovelStageRun> = gm_novel_stage_runs::table
            .filter(gm_novel_stage_runs::job_id.eq(job_id))
            .filter(gm_novel_stage_runs::stage_code.eq(stage_code))
            .filter(gm_novel_stage_runs::attempt_no.eq(attempt_no))
            .first::<NovelStageRun>(&mut conn)
            .optional()
            .map_err(|e| e.to_string())?;

        if let Some(row) = existing {
            return diesel::update(
                gm_novel_stage_runs::table.filter(gm_novel_stage_runs::id.eq(row.id)),
            )
            .set((
                gm_novel_stage_runs::status.eq(status),
                gm_novel_stage_runs::output_payload.eq(
                    output_payload.unwrap_or_else(|| row.output_payload.clone()),
                ),
                gm_novel_stage_runs::error_message.eq(error_message),
                gm_novel_stage_runs::started_at.eq(started_at.or(row.started_at)),
                gm_novel_stage_runs::completed_at.eq(completed_at.or(row.completed_at)),
                gm_novel_stage_runs::updated_at.eq(now),
            ))
            .get_result::<NovelStageRun>(&mut conn)
            .map_err(|e| format!("update stage run: {e}"));
        }

        diesel::insert_into(gm_novel_stage_runs::table)
            .values((
                gm_novel_stage_runs::project_id.eq(project_id),
                gm_novel_stage_runs::job_id.eq(job_id),
                gm_novel_stage_runs::chapter_number.eq(chapter_number),
                gm_novel_stage_runs::stage_code.eq(stage_code),
                gm_novel_stage_runs::status.eq(status),
                gm_novel_stage_runs::input_hash.eq(input_hash),
                gm_novel_stage_runs::input_payload.eq(input_payload),
                gm_novel_stage_runs::output_payload.eq(
                    output_payload.unwrap_or_else(|| json!({})),
                ),
                gm_novel_stage_runs::error_message.eq(error_message),
                gm_novel_stage_runs::attempt_no.eq(attempt_no),
                gm_novel_stage_runs::started_at.eq(started_at),
                gm_novel_stage_runs::completed_at.eq(completed_at),
            ))
            .get_result::<NovelStageRun>(&mut conn)
            .map_err(|e| format!("insert stage run: {e}"))
    }

    pub fn record_stage_event(
        &self,
        project_id: &str,
        job_id: Option<i64>,
        stage_run_id: Option<i64>,
        chapter_number: Option<i32>,
        event_type: &str,
        stage_code: Option<&str>,
        payload: serde_json::Value,
    ) -> Result<NovelStageEvent, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;
        let now = Utc::now();

        let next_seq: i64 = gm_novel_stage_events::table
            .filter(gm_novel_stage_events::project_id.eq(project_id))
            .select(max(gm_novel_stage_events::sequence))
            .first::<Option<i64>>(&mut conn)
            .map_err(|e| e.to_string())?
            .unwrap_or(0)
            + 1;

        diesel::insert_into(gm_novel_stage_events::table)
            .values((
                gm_novel_stage_events::project_id.eq(project_id),
                gm_novel_stage_events::job_id.eq(job_id),
                gm_novel_stage_events::stage_run_id.eq(stage_run_id),
                gm_novel_stage_events::chapter_number.eq(chapter_number),
                gm_novel_stage_events::sequence.eq(next_seq),
                gm_novel_stage_events::event_type.eq(event_type),
                gm_novel_stage_events::stage_code.eq(stage_code),
                gm_novel_stage_events::payload.eq(payload),
                gm_novel_stage_events::occurred_at.eq(now),
            ))
            .get_result::<NovelStageEvent>(&mut conn)
            .map_err(|e| format!("record stage event: {e}"))
    }

    // =========================================================================
    // Event methods
    // =========================================================================

    pub fn list_events(
        &self,
        project_id: &str,
        after_sequence: Option<i64>,
        limit: i64,
    ) -> Result<Vec<NovelStageEvent>, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;
        let mut query = gm_novel_stage_events::table
            .filter(gm_novel_stage_events::project_id.eq(project_id))
            .into_boxed();

        if let Some(seq) = after_sequence {
            query = query.filter(gm_novel_stage_events::sequence.gt(seq));
        }

        query
            .order(gm_novel_stage_events::sequence.asc())
            .limit(limit)
            .load::<NovelStageEvent>(&mut conn)
            .map_err(|e| e.to_string())
    }

    // =========================================================================
    // Blueprint chapter parsing helpers
    // =========================================================================

    fn parse_blueprint_chapters(raw_text: &str) -> Vec<ParsedChapter> {
        let lines: Vec<&str> = raw_text.lines().collect();
        let mut chapters: Vec<ParsedChapter> = Vec::new();
        let mut chapter_starts: Vec<(usize, i32, String)> = Vec::new();

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix('第') {
                if let Some(pos) = rest.find('章') {
                    if let Ok(num) = rest[..pos].trim().parse::<i32>() {
                        let after = rest[pos + '章'.len_utf8()..].trim();
                        let title = after
                            .trim_start_matches(['-', '—', ':', '：', ' '])
                            .trim()
                            .to_string();
                        chapter_starts.push((i, num, title));
                    }
                }
            }
        }

        for (idx, &(start, num, ref title)) in chapter_starts.iter().enumerate() {
            let end = if idx + 1 < chapter_starts.len() {
                chapter_starts[idx + 1].0
            } else {
                lines.len()
            };
            let block = &lines[start..end];

            let mut ch = ParsedChapter {
                chapter_number: num,
                chapter_title: title.clone(),
                chapter_role: String::new(),
                chapter_purpose: String::new(),
                suspense_level: String::new(),
                foreshadowing: String::new(),
                plot_twist_level: String::new(),
                chapter_summary: String::new(),
            };

            for line in block {
                let t = line.trim();
                if let Some(v) = t.strip_prefix("本章定位:").or_else(|| t.strip_prefix("本章定位：")) {
                    ch.chapter_role = v.trim().to_string();
                } else if let Some(v) = t.strip_prefix("核心作用:").or_else(|| t.strip_prefix("核心作用：")) {
                    ch.chapter_purpose = v.trim().to_string();
                } else if let Some(v) = t.strip_prefix("悬念密度:").or_else(|| t.strip_prefix("悬念密度：")) {
                    ch.suspense_level = v.trim().to_string();
                } else if let Some(v) = t.strip_prefix("伏笔操作:").or_else(|| t.strip_prefix("伏笔操作：")) {
                    ch.foreshadowing = v.trim().to_string();
                } else if let Some(v) = t.strip_prefix("认知颠覆:").or_else(|| t.strip_prefix("认知颠覆：")) {
                    ch.plot_twist_level = v.trim().to_string();
                } else if let Some(v) = t.strip_prefix("本章简述:").or_else(|| t.strip_prefix("本章简述：")) {
                    ch.chapter_summary = v.trim().to_string();
                }
            }

            chapters.push(ch);
        }

        chapters
    }
}
