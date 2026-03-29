use crate::config::database::Database;
use crate::dto::drama_dto::{
    DramaPrivateCharacterRequest, DramaPrivateCharacterResponse, DramaPrivateCharacterUpdateRequest,
    DramaPrivateSceneAssetRequest, DramaPrivateSceneAssetResponse,
    DramaPrivateSceneAssetUpdateRequest, DramaPrivateStyleAssetRequest,
    DramaPrivateStyleAssetResponse, DramaPrivateStyleAssetUpdateRequest,
    DramaProjectResourcesRequest, DramaProjectResourcesResponse,
};
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Jsonb, Text, Timestamptz};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct DramaPrivateAssetsService {
    db: Arc<Database>,
}

#[derive(Debug)]
pub enum ProjectResourcesError {
    UnsupportedResourceFields,
    PrimaryStyleMustBeIncluded {
        primary_style_asset_id: String,
    },
    ForbiddenCharacterAssociation { invalid_character_ids: Vec<String> },
    ForbiddenStyleAssociation { invalid_style_asset_ids: Vec<String> },
    Database(String),
}

#[derive(Debug, QueryableByName)]
struct PrivateCharacterRow {
    #[diesel(sql_type = Text)]
    id: String,
    #[diesel(sql_type = Text)]
    name: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    gender: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    age: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    appearance: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    personality: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    voice_id: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    reference_image_url: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    notes: Option<String>,
    #[diesel(sql_type = Timestamptz)]
    created_at: NaiveDateTime,
    #[diesel(sql_type = Timestamptz)]
    updated_at: NaiveDateTime,
}

#[derive(Debug, QueryableByName)]
struct CharacterIdRow {
    #[diesel(sql_type = Text)]
    character_id: String,
}

#[derive(Debug, QueryableByName)]
struct StyleAssetIdRow {
    #[diesel(sql_type = Text)]
    style_asset_id: String,
}

#[derive(Debug, QueryableByName)]
struct PrivateSceneAssetRow {
    #[diesel(sql_type = Text)]
    id: String,
    #[diesel(sql_type = Text)]
    name: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    category: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    location_description: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    time_of_day: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    mood: Option<String>,
    #[diesel(sql_type = Jsonb)]
    reference_image_urls: Value,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    camera_notes: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    notes: Option<String>,
    #[diesel(sql_type = Timestamptz)]
    created_at: NaiveDateTime,
    #[diesel(sql_type = Timestamptz)]
    updated_at: NaiveDateTime,
}

#[derive(Debug, QueryableByName)]
struct PrivateStyleAssetRow {
    #[diesel(sql_type = Text)]
    id: String,
    #[diesel(sql_type = Text)]
    name: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    visual_style: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    color_tone: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    aspect_ratio: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    resolution: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    lighting_mood: Option<String>,
    #[diesel(sql_type = Jsonb)]
    reference_image_urls: Value,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    notes: Option<String>,
    #[diesel(sql_type = Timestamptz)]
    created_at: NaiveDateTime,
    #[diesel(sql_type = Timestamptz)]
    updated_at: NaiveDateTime,
}

#[derive(Debug, QueryableByName)]
struct ProjectStyleAssetLinkRow {
    #[diesel(sql_type = Text)]
    style_asset_id: String,
    #[diesel(sql_type = diesel::sql_types::Bool)]
    is_primary: bool,
}

impl DramaPrivateAssetsService {
    pub fn new(db: &Arc<Database>) -> Self {
        Self { db: db.clone() }
    }

    pub fn list_characters(
        &self,
        user_id: i64,
    ) -> Result<Vec<DramaPrivateCharacterResponse>, String> {
        let conn = &mut self.db.pool.get().map_err(|e| format!("db pool: {}", e))?;
        diesel::sql_query(
            "SELECT id, name, gender, age, appearance, personality, voice_id, reference_image_url, notes, created_at, updated_at \
             FROM drama_private_characters \
             WHERE user_id = $1 \
             ORDER BY updated_at DESC, created_at DESC, id DESC",
        )
        .bind::<BigInt, _>(user_id)
        .load::<PrivateCharacterRow>(conn)
        .map(|rows| rows.into_iter().map(map_character_row).collect())
        .map_err(|e| format!("list private characters: {}", e))
    }

    pub fn create_character(
        &self,
        user_id: i64,
        req: &DramaPrivateCharacterRequest,
    ) -> Result<DramaPrivateCharacterResponse, String> {
        let conn = &mut self.db.pool.get().map_err(|e| format!("db pool: {}", e))?;
        let id = Uuid::new_v4().to_string();
        diesel::sql_query(
            "INSERT INTO drama_private_characters \
             (id, user_id, name, gender, age, appearance, personality, voice_id, reference_image_url, notes) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) \
             RETURNING id, name, gender, age, appearance, personality, voice_id, reference_image_url, notes, created_at, updated_at",
        )
        .bind::<Text, _>(&id)
        .bind::<BigInt, _>(user_id)
        .bind::<Text, _>(&req.name)
        .bind::<diesel::sql_types::Nullable<Text>, _>(&req.gender)
        .bind::<diesel::sql_types::Nullable<Text>, _>(&req.age)
        .bind::<diesel::sql_types::Nullable<Text>, _>(&req.appearance)
        .bind::<diesel::sql_types::Nullable<Text>, _>(&req.personality)
        .bind::<diesel::sql_types::Nullable<Text>, _>(&req.voice_id)
        .bind::<diesel::sql_types::Nullable<Text>, _>(&req.reference_image_url)
        .bind::<diesel::sql_types::Nullable<Text>, _>(&req.notes)
        .get_result::<PrivateCharacterRow>(conn)
        .map(map_character_row)
        .map_err(|e| format!("create private character: {}", e))
    }

    pub fn update_character(
        &self,
        user_id: i64,
        id: &str,
        req: &DramaPrivateCharacterUpdateRequest,
    ) -> Result<Option<DramaPrivateCharacterResponse>, String> {
        let conn = &mut self.db.pool.get().map_err(|e| format!("db pool: {}", e))?;
        let existing = get_character_row(conn, user_id, id)
            .map_err(|e| format!("read private character before update: {}", e))?;
        let Some(existing) = existing else {
            return Ok(None);
        };

        diesel::sql_query(
            "UPDATE drama_private_characters \
             SET name = $3, gender = $4, age = $5, appearance = $6, personality = $7, voice_id = $8, reference_image_url = $9, notes = $10, updated_at = NOW() \
             WHERE id = $1 AND user_id = $2 \
             RETURNING id, name, gender, age, appearance, personality, voice_id, reference_image_url, notes, created_at, updated_at",
        )
        .bind::<Text, _>(id)
        .bind::<BigInt, _>(user_id)
        .bind::<Text, _>(req.name.as_deref().unwrap_or(&existing.name))
        .bind::<diesel::sql_types::Nullable<Text>, _>(req.gender.as_ref().or(existing.gender.as_ref()))
        .bind::<diesel::sql_types::Nullable<Text>, _>(req.age.as_ref().or(existing.age.as_ref()))
        .bind::<diesel::sql_types::Nullable<Text>, _>(
            req.appearance.as_ref().or(existing.appearance.as_ref()),
        )
        .bind::<diesel::sql_types::Nullable<Text>, _>(
            req.personality.as_ref().or(existing.personality.as_ref()),
        )
        .bind::<diesel::sql_types::Nullable<Text>, _>(
            req.voice_id.as_ref().or(existing.voice_id.as_ref()),
        )
        .bind::<diesel::sql_types::Nullable<Text>, _>(
            req.reference_image_url
                .as_ref()
                .or(existing.reference_image_url.as_ref()),
        )
        .bind::<diesel::sql_types::Nullable<Text>, _>(req.notes.as_ref().or(existing.notes.as_ref()))
        .get_result::<PrivateCharacterRow>(conn)
        .optional()
        .map(|row| row.map(map_character_row))
        .map_err(|e| format!("update private character: {}", e))
    }

    pub fn delete_character(&self, user_id: i64, id: &str) -> Result<bool, String> {
        let conn = &mut self.db.pool.get().map_err(|e| format!("db pool: {}", e))?;
        diesel::sql_query(
            "DELETE FROM drama_private_characters \
             WHERE id = $1 AND user_id = $2",
        )
        .bind::<Text, _>(id)
        .bind::<BigInt, _>(user_id)
        .execute(conn)
        .map(|count| count > 0)
        .map_err(|e| format!("delete private character: {}", e))
    }

    pub fn list_scene_assets(
        &self,
        user_id: i64,
    ) -> Result<Vec<DramaPrivateSceneAssetResponse>, String> {
        let conn = &mut self.db.pool.get().map_err(|e| format!("db pool: {}", e))?;
        diesel::sql_query(
            "SELECT id, name, category, location_description, time_of_day, mood, reference_image_urls, camera_notes, notes, created_at, updated_at \
             FROM drama_private_scene_assets \
             WHERE user_id = $1 \
             ORDER BY updated_at DESC, created_at DESC, id DESC",
        )
        .bind::<BigInt, _>(user_id)
        .load::<PrivateSceneAssetRow>(conn)
        .map(|rows| rows.into_iter().map(map_scene_asset_row).collect())
        .map_err(|e| format!("list private scene assets: {}", e))
    }

    pub fn create_scene_asset(
        &self,
        user_id: i64,
        req: &DramaPrivateSceneAssetRequest,
    ) -> Result<DramaPrivateSceneAssetResponse, String> {
        let conn = &mut self.db.pool.get().map_err(|e| format!("db pool: {}", e))?;
        let id = Uuid::new_v4().to_string();
        let reference_image_urls = json!(req.reference_image_urls);
        diesel::sql_query(
            "INSERT INTO drama_private_scene_assets \
             (id, user_id, name, category, location_description, time_of_day, mood, reference_image_urls, camera_notes, notes) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) \
             RETURNING id, name, category, location_description, time_of_day, mood, reference_image_urls, camera_notes, notes, created_at, updated_at",
        )
        .bind::<Text, _>(&id)
        .bind::<BigInt, _>(user_id)
        .bind::<Text, _>(&req.name)
        .bind::<diesel::sql_types::Nullable<Text>, _>(&req.category)
        .bind::<diesel::sql_types::Nullable<Text>, _>(&req.location_description)
        .bind::<diesel::sql_types::Nullable<Text>, _>(&req.time_of_day)
        .bind::<diesel::sql_types::Nullable<Text>, _>(&req.mood)
        .bind::<Jsonb, _>(&reference_image_urls)
        .bind::<diesel::sql_types::Nullable<Text>, _>(&req.camera_notes)
        .bind::<diesel::sql_types::Nullable<Text>, _>(&req.notes)
        .get_result::<PrivateSceneAssetRow>(conn)
        .map(map_scene_asset_row)
        .map_err(|e| format!("create private scene asset: {}", e))
    }

    pub fn update_scene_asset(
        &self,
        user_id: i64,
        id: &str,
        req: &DramaPrivateSceneAssetUpdateRequest,
    ) -> Result<Option<DramaPrivateSceneAssetResponse>, String> {
        let conn = &mut self.db.pool.get().map_err(|e| format!("db pool: {}", e))?;
        let existing = get_scene_asset_row(conn, user_id, id)
            .map_err(|e| format!("read private scene asset before update: {}", e))?;
        let Some(existing) = existing else {
            return Ok(None);
        };

        let reference_image_urls = req
            .reference_image_urls
            .as_ref()
            .map(|items| json!(items))
            .unwrap_or_else(|| existing.reference_image_urls.clone());

        diesel::sql_query(
            "UPDATE drama_private_scene_assets \
             SET name = $3, category = $4, location_description = $5, time_of_day = $6, mood = $7, reference_image_urls = $8, camera_notes = $9, notes = $10, updated_at = NOW() \
             WHERE id = $1 AND user_id = $2 \
             RETURNING id, name, category, location_description, time_of_day, mood, reference_image_urls, camera_notes, notes, created_at, updated_at",
        )
        .bind::<Text, _>(id)
        .bind::<BigInt, _>(user_id)
        .bind::<Text, _>(req.name.as_deref().unwrap_or(&existing.name))
        .bind::<diesel::sql_types::Nullable<Text>, _>(
            req.category.as_ref().or(existing.category.as_ref()),
        )
        .bind::<diesel::sql_types::Nullable<Text>, _>(
            req.location_description
                .as_ref()
                .or(existing.location_description.as_ref()),
        )
        .bind::<diesel::sql_types::Nullable<Text>, _>(
            req.time_of_day.as_ref().or(existing.time_of_day.as_ref()),
        )
        .bind::<diesel::sql_types::Nullable<Text>, _>(req.mood.as_ref().or(existing.mood.as_ref()))
        .bind::<Jsonb, _>(&reference_image_urls)
        .bind::<diesel::sql_types::Nullable<Text>, _>(
            req.camera_notes.as_ref().or(existing.camera_notes.as_ref()),
        )
        .bind::<diesel::sql_types::Nullable<Text>, _>(req.notes.as_ref().or(existing.notes.as_ref()))
        .get_result::<PrivateSceneAssetRow>(conn)
        .optional()
        .map(|row| row.map(map_scene_asset_row))
        .map_err(|e| format!("update private scene asset: {}", e))
    }

    pub fn delete_scene_asset(&self, user_id: i64, id: &str) -> Result<bool, String> {
        let conn = &mut self.db.pool.get().map_err(|e| format!("db pool: {}", e))?;
        diesel::sql_query(
            "DELETE FROM drama_private_scene_assets \
             WHERE id = $1 AND user_id = $2",
        )
        .bind::<Text, _>(id)
        .bind::<BigInt, _>(user_id)
        .execute(conn)
        .map(|count| count > 0)
        .map_err(|e| format!("delete private scene asset: {}", e))
    }

    pub fn list_style_assets(
        &self,
        user_id: i64,
    ) -> Result<Vec<DramaPrivateStyleAssetResponse>, String> {
        let conn = &mut self.db.pool.get().map_err(|e| format!("db pool: {}", e))?;
        diesel::sql_query(
            "SELECT id, name, visual_style, color_tone, aspect_ratio, resolution, lighting_mood, reference_image_urls, notes, created_at, updated_at \
             FROM drama_private_style_assets \
             WHERE user_id = $1 \
             ORDER BY updated_at DESC, created_at DESC, id DESC",
        )
        .bind::<BigInt, _>(user_id)
        .load::<PrivateStyleAssetRow>(conn)
        .map(|rows| rows.into_iter().map(map_style_asset_row).collect())
        .map_err(|e| format!("list private style assets: {}", e))
    }

    pub fn create_style_asset(
        &self,
        user_id: i64,
        req: &DramaPrivateStyleAssetRequest,
    ) -> Result<DramaPrivateStyleAssetResponse, String> {
        let conn = &mut self.db.pool.get().map_err(|e| format!("db pool: {}", e))?;
        let id = Uuid::new_v4().to_string();
        let reference_image_urls = json!(req.reference_image_urls);
        diesel::sql_query(
            "INSERT INTO drama_private_style_assets \
             (id, user_id, name, visual_style, color_tone, aspect_ratio, resolution, lighting_mood, reference_image_urls, notes) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) \
             RETURNING id, name, visual_style, color_tone, aspect_ratio, resolution, lighting_mood, reference_image_urls, notes, created_at, updated_at",
        )
        .bind::<Text, _>(&id)
        .bind::<BigInt, _>(user_id)
        .bind::<Text, _>(&req.name)
        .bind::<diesel::sql_types::Nullable<Text>, _>(&req.visual_style)
        .bind::<diesel::sql_types::Nullable<Text>, _>(&req.color_tone)
        .bind::<diesel::sql_types::Nullable<Text>, _>(&req.aspect_ratio)
        .bind::<diesel::sql_types::Nullable<Text>, _>(&req.resolution)
        .bind::<diesel::sql_types::Nullable<Text>, _>(&req.lighting_mood)
        .bind::<Jsonb, _>(&reference_image_urls)
        .bind::<diesel::sql_types::Nullable<Text>, _>(&req.notes)
        .get_result::<PrivateStyleAssetRow>(conn)
        .map(map_style_asset_row)
        .map_err(|e| format!("create private style asset: {}", e))
    }

    pub fn update_style_asset(
        &self,
        user_id: i64,
        id: &str,
        req: &DramaPrivateStyleAssetUpdateRequest,
    ) -> Result<Option<DramaPrivateStyleAssetResponse>, String> {
        let conn = &mut self.db.pool.get().map_err(|e| format!("db pool: {}", e))?;
        let existing = get_style_asset_row(conn, user_id, id)
            .map_err(|e| format!("read private style asset before update: {}", e))?;
        let Some(existing) = existing else {
            return Ok(None);
        };

        let reference_image_urls = req
            .reference_image_urls
            .as_ref()
            .map(|items| json!(items))
            .unwrap_or_else(|| existing.reference_image_urls.clone());

        diesel::sql_query(
            "UPDATE drama_private_style_assets \
             SET name = $3, visual_style = $4, color_tone = $5, aspect_ratio = $6, resolution = $7, lighting_mood = $8, reference_image_urls = $9, notes = $10, updated_at = NOW() \
             WHERE id = $1 AND user_id = $2 \
             RETURNING id, name, visual_style, color_tone, aspect_ratio, resolution, lighting_mood, reference_image_urls, notes, created_at, updated_at",
        )
        .bind::<Text, _>(id)
        .bind::<BigInt, _>(user_id)
        .bind::<Text, _>(req.name.as_deref().unwrap_or(&existing.name))
        .bind::<diesel::sql_types::Nullable<Text>, _>(
            req.visual_style.as_ref().or(existing.visual_style.as_ref()),
        )
        .bind::<diesel::sql_types::Nullable<Text>, _>(
            req.color_tone.as_ref().or(existing.color_tone.as_ref()),
        )
        .bind::<diesel::sql_types::Nullable<Text>, _>(
            req.aspect_ratio.as_ref().or(existing.aspect_ratio.as_ref()),
        )
        .bind::<diesel::sql_types::Nullable<Text>, _>(
            req.resolution.as_ref().or(existing.resolution.as_ref()),
        )
        .bind::<diesel::sql_types::Nullable<Text>, _>(
            req.lighting_mood.as_ref().or(existing.lighting_mood.as_ref()),
        )
        .bind::<Jsonb, _>(&reference_image_urls)
        .bind::<diesel::sql_types::Nullable<Text>, _>(req.notes.as_ref().or(existing.notes.as_ref()))
        .get_result::<PrivateStyleAssetRow>(conn)
        .optional()
        .map(|row| row.map(map_style_asset_row))
        .map_err(|e| format!("update private style asset: {}", e))
    }

    pub fn delete_style_asset(&self, user_id: i64, id: &str) -> Result<bool, String> {
        let conn = &mut self.db.pool.get().map_err(|e| format!("db pool: {}", e))?;
        diesel::sql_query(
            "DELETE FROM drama_private_style_assets \
             WHERE id = $1 AND user_id = $2",
        )
        .bind::<Text, _>(id)
        .bind::<BigInt, _>(user_id)
        .execute(conn)
        .map(|count| count > 0)
        .map_err(|e| format!("delete private style asset: {}", e))
    }

    pub fn get_project_resources(
        &self,
        project_id: &str,
        user_id: i64,
    ) -> Result<DramaProjectResourcesResponse, String> {
        let conn = &mut self.db.pool.get().map_err(|e| format!("db pool: {}", e))?;
        let character_ids = diesel::sql_query(
            "SELECT character_id \
             FROM drama_project_character_links \
             WHERE project_id = $1 AND user_id = $2 \
             ORDER BY created_at ASC, character_id ASC",
        )
        .bind::<Text, _>(project_id)
        .bind::<BigInt, _>(user_id)
        .load::<CharacterIdRow>(conn)
        .map_err(|e| format!("get project character links: {}", e))?
        .into_iter()
        .map(|row| row.character_id)
        .collect();

        let style_links = diesel::sql_query(
            "SELECT style_asset_id, is_primary \
             FROM drama_project_style_asset_links \
             WHERE project_id = $1 AND user_id = $2 \
             ORDER BY created_at ASC, style_asset_id ASC",
        )
        .bind::<Text, _>(project_id)
        .bind::<BigInt, _>(user_id)
        .load::<ProjectStyleAssetLinkRow>(conn)
        .map_err(|e| format!("get project style links: {}", e))?;
        let style_asset_ids = style_links
            .iter()
            .map(|row| row.style_asset_id.clone())
            .collect::<Vec<_>>();
        let primary_style_asset_id = style_links
            .iter()
            .find_map(|row| row.is_primary.then_some(row.style_asset_id.clone()));

        Ok(DramaProjectResourcesResponse {
            project_id: project_id.to_string(),
            character_ids,
            scene_asset_ids: Vec::new(),
            style_asset_ids,
            primary_style_asset_id,
        })
    }

    pub fn put_project_resources(
        &self,
        project_id: &str,
        user_id: i64,
        req: &DramaProjectResourcesRequest,
    ) -> Result<DramaProjectResourcesResponse, ProjectResourcesError> {
        let conn = &mut self
            .db
            .pool
            .get()
            .map_err(|e| ProjectResourcesError::Database(format!("db pool: {}", e)))?;
        let character_ids = dedupe_ids(&req.character_ids);
        let style_asset_ids = dedupe_ids(&req.style_asset_ids);
        let primary_style_asset_id = normalize_optional_id(req.primary_style_asset_id.as_deref());

        if has_unsupported_project_resources(req) {
            return Err(ProjectResourcesError::UnsupportedResourceFields);
        }

        if let Some(primary_style_asset_id) = primary_style_asset_id.as_ref() {
            if !style_asset_ids.contains(primary_style_asset_id) {
                return Err(ProjectResourcesError::PrimaryStyleMustBeIncluded {
                    primary_style_asset_id: primary_style_asset_id.clone(),
                });
            }
        }

        let invalid_character_ids = character_ids
            .iter()
            .map(|character_id| {
                character_owned_by_user(conn, user_id, character_id)
                    .map(|owned| (character_id.clone(), owned))
            })
            .collect::<QueryResult<Vec<(String, bool)>>>()
            .map_err(|e| ProjectResourcesError::Database(format!("validate character ownership: {}", e)))?
            .into_iter()
            .filter_map(|(character_id, owned)| (!owned).then_some(character_id))
            .collect::<Vec<_>>();

        if !invalid_character_ids.is_empty() {
            return Err(ProjectResourcesError::ForbiddenCharacterAssociation {
                invalid_character_ids,
            });
        }

        let invalid_style_asset_ids = style_asset_ids
            .iter()
            .map(|style_asset_id| {
                style_asset_owned_by_user(conn, user_id, style_asset_id)
                    .map(|owned| (style_asset_id.clone(), owned))
            })
            .collect::<QueryResult<Vec<(String, bool)>>>()
            .map_err(|e| ProjectResourcesError::Database(format!("validate style ownership: {}", e)))?
            .into_iter()
            .filter_map(|(style_asset_id, owned)| (!owned).then_some(style_asset_id))
            .collect::<Vec<_>>();

        if !invalid_style_asset_ids.is_empty() {
            return Err(ProjectResourcesError::ForbiddenStyleAssociation {
                invalid_style_asset_ids,
            });
        }

        conn.transaction::<DramaProjectResourcesResponse, diesel::result::Error, _>(|conn| {
            diesel::sql_query(
                "DELETE FROM drama_project_character_links \
                 WHERE project_id = $1 AND user_id = $2",
            )
            .bind::<Text, _>(project_id)
            .bind::<BigInt, _>(user_id)
            .execute(conn)?;

            for character_id in &character_ids {
                diesel::sql_query(
                    "INSERT INTO drama_project_character_links \
                     (project_id, user_id, character_id) \
                     VALUES ($1, $2, $3)",
                )
                .bind::<Text, _>(project_id)
                .bind::<BigInt, _>(user_id)
                .bind::<Text, _>(character_id)
                .execute(conn)?;
            }

            diesel::sql_query(
                "DELETE FROM drama_project_style_asset_links \
                 WHERE project_id = $1 AND user_id = $2",
            )
            .bind::<Text, _>(project_id)
            .bind::<BigInt, _>(user_id)
            .execute(conn)?;

            for style_asset_id in &style_asset_ids {
                diesel::sql_query(
                    "INSERT INTO drama_project_style_asset_links \
                     (project_id, user_id, style_asset_id, is_primary) \
                     VALUES ($1, $2, $3, $4)",
                )
                .bind::<Text, _>(project_id)
                .bind::<BigInt, _>(user_id)
                .bind::<Text, _>(style_asset_id)
                .bind::<diesel::sql_types::Bool, _>(
                    primary_style_asset_id
                        .as_deref()
                        .is_some_and(|primary| primary == style_asset_id),
                )
                .execute(conn)?;
            }

            Ok(DramaProjectResourcesResponse {
                project_id: project_id.to_string(),
                character_ids: character_ids.clone(),
                scene_asset_ids: Vec::new(),
                style_asset_ids: style_asset_ids.clone(),
                primary_style_asset_id: primary_style_asset_id.clone(),
            })
        })
        .map_err(|e| ProjectResourcesError::Database(format!("replace project resources: {}", e)))
    }
}

fn get_character_row(
    conn: &mut diesel::PgConnection,
    user_id: i64,
    id: &str,
) -> QueryResult<Option<PrivateCharacterRow>> {
    diesel::sql_query(
        "SELECT id, name, gender, age, appearance, personality, voice_id, reference_image_url, notes, created_at, updated_at \
         FROM drama_private_characters \
         WHERE id = $1 AND user_id = $2",
    )
    .bind::<Text, _>(id)
    .bind::<BigInt, _>(user_id)
    .get_result::<PrivateCharacterRow>(conn)
    .optional()
}

fn get_scene_asset_row(
    conn: &mut diesel::PgConnection,
    user_id: i64,
    id: &str,
) -> QueryResult<Option<PrivateSceneAssetRow>> {
    diesel::sql_query(
        "SELECT id, name, category, location_description, time_of_day, mood, reference_image_urls, camera_notes, notes, created_at, updated_at \
         FROM drama_private_scene_assets \
         WHERE id = $1 AND user_id = $2",
    )
    .bind::<Text, _>(id)
    .bind::<BigInt, _>(user_id)
    .get_result::<PrivateSceneAssetRow>(conn)
    .optional()
}

fn get_style_asset_row(
    conn: &mut diesel::PgConnection,
    user_id: i64,
    id: &str,
) -> QueryResult<Option<PrivateStyleAssetRow>> {
    diesel::sql_query(
        "SELECT id, name, visual_style, color_tone, aspect_ratio, resolution, lighting_mood, reference_image_urls, notes, created_at, updated_at \
         FROM drama_private_style_assets \
         WHERE id = $1 AND user_id = $2",
    )
    .bind::<Text, _>(id)
    .bind::<BigInt, _>(user_id)
    .get_result::<PrivateStyleAssetRow>(conn)
    .optional()
}

fn character_owned_by_user(
    conn: &mut diesel::PgConnection,
    user_id: i64,
    id: &str,
) -> QueryResult<bool> {
    diesel::sql_query(
        "SELECT id AS character_id \
         FROM drama_private_characters \
         WHERE id = $1 AND user_id = $2",
    )
    .bind::<Text, _>(id)
    .bind::<BigInt, _>(user_id)
    .get_result::<CharacterIdRow>(conn)
    .optional()
    .map(|row| row.is_some())
}

fn style_asset_owned_by_user(
    conn: &mut diesel::PgConnection,
    user_id: i64,
    id: &str,
) -> QueryResult<bool> {
    diesel::sql_query(
        "SELECT id AS style_asset_id \
         FROM drama_private_style_assets \
         WHERE id = $1 AND user_id = $2",
    )
    .bind::<Text, _>(id)
    .bind::<BigInt, _>(user_id)
    .get_result::<StyleAssetIdRow>(conn)
    .optional()
    .map(|row| {
        row.map(|row| !row.style_asset_id.is_empty())
            .unwrap_or(false)
    })
}

fn map_character_row(row: PrivateCharacterRow) -> DramaPrivateCharacterResponse {
    DramaPrivateCharacterResponse {
        id: row.id,
        name: row.name,
        gender: row.gender,
        age: row.age,
        appearance: row.appearance,
        personality: row.personality,
        voice_id: row.voice_id,
        reference_image_url: row.reference_image_url,
        notes: row.notes,
        created_at: row.created_at.and_utc().to_rfc3339(),
        updated_at: row.updated_at.and_utc().to_rfc3339(),
    }
}

fn map_scene_asset_row(row: PrivateSceneAssetRow) -> DramaPrivateSceneAssetResponse {
    DramaPrivateSceneAssetResponse {
        id: row.id,
        name: row.name,
        category: row.category,
        location_description: row.location_description,
        time_of_day: row.time_of_day,
        mood: row.mood,
        reference_image_urls: json_array_to_string_vec(row.reference_image_urls),
        camera_notes: row.camera_notes,
        notes: row.notes,
        created_at: row.created_at.and_utc().to_rfc3339(),
        updated_at: row.updated_at.and_utc().to_rfc3339(),
    }
}

fn map_style_asset_row(row: PrivateStyleAssetRow) -> DramaPrivateStyleAssetResponse {
    DramaPrivateStyleAssetResponse {
        id: row.id,
        name: row.name,
        visual_style: row.visual_style,
        color_tone: row.color_tone,
        aspect_ratio: row.aspect_ratio,
        resolution: row.resolution,
        lighting_mood: row.lighting_mood,
        reference_image_urls: json_array_to_string_vec(row.reference_image_urls),
        notes: row.notes,
        created_at: row.created_at.and_utc().to_rfc3339(),
        updated_at: row.updated_at.and_utc().to_rfc3339(),
    }
}

fn json_array_to_string_vec(value: Value) -> Vec<String> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(ToOwned::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

fn dedupe_ids(ids: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut unique = Vec::new();
    for id in ids {
        if seen.insert(id.clone()) {
            unique.push(id.clone());
        }
    }
    unique
}

fn normalize_optional_id(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn has_unsupported_project_resources(req: &DramaProjectResourcesRequest) -> bool {
    !req.scene_asset_ids.is_empty()
}
