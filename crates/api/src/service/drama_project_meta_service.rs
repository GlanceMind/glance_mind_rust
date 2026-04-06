use crate::config::database::Database;
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Jsonb, Text, Timestamptz};
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Clone)]
pub struct DramaProjectMetaService {
    db: Arc<Database>,
}

#[derive(Debug, QueryableByName)]
pub struct DramaProjectMetaRow {
    #[diesel(sql_type = Text)]
    pub project_id: String,
    #[diesel(sql_type = BigInt)]
    pub user_id: i64,
    #[diesel(sql_type = Jsonb)]
    pub characters: Value,
    #[diesel(sql_type = Jsonb)]
    pub style_references: Value,
    #[diesel(sql_type = Jsonb)]
    pub text_materials: Value,
    #[diesel(sql_type = Jsonb)]
    pub visual_settings: Value,
    #[diesel(sql_type = Timestamptz)]
    pub created_at: chrono::NaiveDateTime,
    #[diesel(sql_type = Timestamptz)]
    pub updated_at: chrono::NaiveDateTime,
}

impl DramaProjectMetaService {
    pub fn new(db: &Arc<Database>) -> Self {
        Self { db: db.clone() }
    }

    pub fn get(
        &self,
        project_id: &str,
        user_id: i64,
    ) -> Result<Option<DramaProjectMetaRow>, String> {
        let conn = &mut self.db.pool.get().map_err(|e| format!("db pool: {}", e))?;
        diesel::sql_query(
            "SELECT project_id, user_id, characters, style_references, text_materials, visual_settings, created_at, updated_at \
             FROM drama_project_meta \
             WHERE project_id = $1 AND user_id = $2",
        )
        .bind::<Text, _>(project_id)
        .bind::<BigInt, _>(user_id)
        .get_result::<DramaProjectMetaRow>(conn)
        .optional()
        .map_err(|e| format!("get drama project meta: {}", e))
    }

    pub fn upsert(
        &self,
        project_id: &str,
        user_id: i64,
        characters: &Value,
        style_references: &Value,
        text_materials: &Value,
        visual_settings: &Value,
    ) -> Result<DramaProjectMetaRow, String> {
        let conn = &mut self.db.pool.get().map_err(|e| format!("db pool: {}", e))?;
        diesel::sql_query(
            "INSERT INTO drama_project_meta \
             (project_id, user_id, characters, style_references, text_materials, visual_settings) \
             VALUES ($1, $2, $3, $4, $5, $6) \
             ON CONFLICT (project_id) DO UPDATE SET \
               user_id = EXCLUDED.user_id, \
               characters = EXCLUDED.characters, \
               style_references = EXCLUDED.style_references, \
               text_materials = EXCLUDED.text_materials, \
               visual_settings = EXCLUDED.visual_settings, \
               updated_at = NOW() \
             RETURNING project_id, user_id, characters, style_references, text_materials, visual_settings, created_at, updated_at",
        )
        .bind::<Text, _>(project_id)
        .bind::<BigInt, _>(user_id)
        .bind::<Jsonb, _>(characters)
        .bind::<Jsonb, _>(style_references)
        .bind::<Jsonb, _>(text_materials)
        .bind::<Jsonb, _>(visual_settings)
        .get_result::<DramaProjectMetaRow>(conn)
        .map_err(|e| format!("upsert drama project meta: {}", e))
    }

    pub fn empty(project_id: &str, user_id: i64) -> DramaProjectMetaRow {
        let now = chrono::Utc::now().naive_utc();
        DramaProjectMetaRow {
            project_id: project_id.to_string(),
            user_id,
            characters: json!([]),
            style_references: json!([]),
            text_materials: json!([]),
            visual_settings: json!({}),
            created_at: now,
            updated_at: now,
        }
    }
}
