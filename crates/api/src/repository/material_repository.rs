use crate::config::database::DBPool;
use diesel::prelude::*;
use diesel::result::Error as DieselError;
use glance_mind_db::entity::material::{NewUserMaterial, UserMaterial};
use glance_mind_db::schema::gm_user_materials as user_materials;

#[derive(Clone)]
pub struct MaterialRepository {
    pool: DBPool,
}

impl MaterialRepository {
    pub fn new(pool: DBPool) -> Self {
        Self { pool }
    }

    /// Create a new material
    pub async fn create(&self, new_material: NewUserMaterial) -> Result<UserMaterial, DieselError> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().expect("Connection error");
            diesel::insert_into(user_materials::table)
                .values(&new_material)
                .returning(UserMaterial::as_returning())
                .get_result(&mut conn)
        })
        .await
        .unwrap_or_else(|e| Err(DieselError::QueryBuilderError(Box::new(e))))
    }

    /// Find material by ID and user ID
    pub async fn find_by_id_and_user(
        &self,
        id: i32,
        user_id: i32,
    ) -> Result<UserMaterial, DieselError> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().expect("Connection error");
            user_materials::table
                .filter(user_materials::id.eq(id))
                .filter(user_materials::user_id.eq(user_id))
                .select(UserMaterial::as_select())
                .first(&mut conn)
        })
        .await
        .unwrap_or_else(|e| Err(DieselError::QueryBuilderError(Box::new(e))))
    }

    /// List materials for a user with pagination and filters
    pub async fn list_by_user(
        &self,
        user_id: i32,
        page: i64,
        page_size: i64,
        tag: Option<String>,
        search: Option<String>,
    ) -> Result<(Vec<UserMaterial>, i64), DieselError> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().expect("Connection error");

            let mut query = user_materials::table
                .filter(user_materials::user_id.eq(user_id))
                .filter(user_materials::is_active.eq(Some(true)))
                .into_boxed();

            if let Some(ref tag_filter) = tag {
                query = query.filter(user_materials::tag.eq(tag_filter));
            }

            if let Some(ref search_term) = search {
                let search_pattern = format!("%{}%", search_term);
                query = query.filter(
                    user_materials::title
                        .ilike(search_pattern.clone())
                        .or(user_materials::description.ilike(search_pattern)),
                );
            }

            let mut total_query = user_materials::table
                .filter(user_materials::user_id.eq(user_id))
                .filter(user_materials::is_active.eq(Some(true)))
                .into_boxed();

            if let Some(ref tag_filter) = tag {
                total_query = total_query.filter(user_materials::tag.eq(tag_filter));
            }
            if let Some(ref search_term) = search {
                let search_pattern = format!("%{}%", search_term);
                total_query = total_query.filter(
                    user_materials::title
                        .ilike(search_pattern.clone())
                        .or(user_materials::description.ilike(search_pattern)),
                );
            }

            let total = total_query.count().get_result::<i64>(&mut conn)?;

            let items = query
                .select(UserMaterial::as_select())
                .order(user_materials::created_at.desc())
                .limit(page_size)
                .offset((page - 1) * page_size)
                .load(&mut conn)?;

            Ok((items, total))
        })
        .await
        .unwrap_or_else(|e| Err(DieselError::QueryBuilderError(Box::new(e))))
    }

    /// Update material
    #[allow(clippy::too_many_arguments)]
    pub async fn update(
        &self,
        id: i32,
        user_id: i32,
        video_url: Option<String>,
        prompt: Option<String>,
        tag: Option<String>,
        title: Option<String>,
        description: Option<String>,
    ) -> Result<UserMaterial, DieselError> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().expect("Connection error");

            let target = user_materials::table
                .filter(user_materials::id.eq(id))
                .filter(user_materials::user_id.eq(user_id))
                .select(UserMaterial::as_select());
            let mut material = target.first::<UserMaterial>(&mut conn)?;

            if let Some(v) = video_url {
                material.video_url = v;
            }
            if let Some(p) = prompt {
                material.prompt = Some(p);
            }
            if let Some(t) = tag {
                material.tag = Some(t);
            }
            if let Some(t) = title {
                material.title = Some(t);
            }
            if let Some(d) = description {
                material.description = Some(d);
            }
            material.updated_at = Some(chrono::Utc::now());

            diesel::update(
                user_materials::table
                    .filter(user_materials::id.eq(id))
                    .filter(user_materials::user_id.eq(user_id)),
            )
            .set(&material)
            .returning(UserMaterial::as_returning())
            .get_result(&mut conn)
        })
        .await
        .unwrap_or_else(|e| Err(DieselError::QueryBuilderError(Box::new(e))))
    }

    /// Update only the prompt field (used by background AI analysis)
    pub async fn update_prompt(
        &self,
        id: i32,
        prompt: String,
    ) -> Result<UserMaterial, DieselError> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().expect("Connection error");
            diesel::update(user_materials::table.filter(user_materials::id.eq(id)))
                .set((
                    user_materials::prompt.eq(Some(prompt)),
                    user_materials::updated_at.eq(Some(chrono::Utc::now())),
                ))
                .returning(UserMaterial::as_returning())
                .get_result(&mut conn)
        })
        .await
        .unwrap_or_else(|e| Err(DieselError::QueryBuilderError(Box::new(e))))
    }

    /// Delete material (soft delete by setting is_active to false)
    pub async fn delete(&self, id: i32, user_id: i32) -> Result<usize, DieselError> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().expect("Connection error");
            diesel::update(
                user_materials::table
                    .filter(user_materials::id.eq(id))
                    .filter(user_materials::user_id.eq(user_id)),
            )
            .set(user_materials::is_active.eq(Some(false)))
            .execute(&mut conn)
        })
        .await
        .unwrap_or_else(|e| Err(DieselError::QueryBuilderError(Box::new(e))))
    }

    /// Collect tags from video_cases table
    /// Returns: (tag_name, name_cn, usage_count)
    pub async fn collect_tags_from_video_cases(
        &self,
    ) -> Result<Vec<(String, String, i64)>, DieselError> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().expect("Connection error");
            use glance_mind_db::schema::gm_data_video_cases as video_cases;

            let results: Vec<(Option<String>, Option<String>)> = video_cases::table
                .select((video_cases::category_name_cn, video_cases::category_name_en))
                .filter(
                    video_cases::category_name_cn
                        .is_not_null()
                        .or(video_cases::category_name_en.is_not_null()),
                )
                .distinct()
                .load(&mut conn)?;

            let mut tag_map: std::collections::HashMap<String, (Option<String>, i64)> =
                std::collections::HashMap::new();

            for (cn, en) in results {
                let tag_name = cn.clone().unwrap_or_else(|| en.clone().unwrap_or_default());
                if !tag_name.is_empty() {
                    let entry = tag_map.entry(tag_name.clone()).or_insert((cn.clone(), 0));
                    let count: i64 = video_cases::table
                        .filter(
                            video_cases::category_name_cn
                                .eq(&tag_name)
                                .or(video_cases::category_name_en.eq(&tag_name)),
                        )
                        .count()
                        .get_result(&mut conn)
                        .unwrap_or(0);
                    entry.1 += count;
                }
            }

            for tag_name in tag_map.keys().cloned().collect::<Vec<_>>() {
                let count: i64 = user_materials::table
                    .filter(user_materials::tag.eq(&tag_name))
                    .count()
                    .get_result(&mut conn)
                    .unwrap_or(0);
                if let Some(entry) = tag_map.get_mut(&tag_name) {
                    entry.1 += count;
                }
            }

            let mut tags: Vec<(String, String, i64)> = tag_map
                .into_iter()
                .map(|(name, (cn, count))| {
                    let cn_str = cn.unwrap_or_else(|| name.clone());
                    (name, cn_str, count)
                })
                .collect();

            tags.sort_by(|a, b| b.2.cmp(&a.2));

            Ok(tags)
        })
        .await
        .unwrap_or_else(|e| Err(DieselError::QueryBuilderError(Box::new(e))))
    }
}
