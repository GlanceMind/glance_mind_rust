use crate::config::database::DBPool;
use diesel::prelude::*;
use diesel::result::Error as DieselError;
use glance_mind_db::entity::material_folder::{MaterialFolder, NewMaterialFolder};
use glance_mind_db::schema::gm_material_folders as folders;
use glance_mind_db::schema::gm_user_materials as materials;

#[derive(Clone)]
pub struct MaterialFolderRepository {
    pool: DBPool,
}

impl MaterialFolderRepository {
    pub fn new(pool: DBPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        new_folder: NewMaterialFolder,
    ) -> Result<MaterialFolder, DieselError> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().expect("Connection error");
            diesel::insert_into(folders::table)
                .values(&new_folder)
                .returning(MaterialFolder::as_returning())
                .get_result(&mut conn)
        })
        .await
        .unwrap_or_else(|e| Err(DieselError::QueryBuilderError(Box::new(e))))
    }

    pub async fn find_by_id_and_user(
        &self,
        id: i32,
        user_id: i32,
    ) -> Result<MaterialFolder, DieselError> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().expect("Connection error");
            folders::table
                .filter(folders::id.eq(id))
                .filter(folders::user_id.eq(user_id))
                .select(MaterialFolder::as_select())
                .first(&mut conn)
        })
        .await
        .unwrap_or_else(|e| Err(DieselError::QueryBuilderError(Box::new(e))))
    }

    pub async fn list_by_parent(
        &self,
        user_id: i32,
        parent_id: Option<i32>,
    ) -> Result<Vec<MaterialFolder>, DieselError> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().expect("Connection error");
            let mut query = folders::table
                .filter(folders::user_id.eq(user_id))
                .into_boxed();

            match parent_id {
                Some(pid) => {
                    query = query.filter(folders::parent_id.eq(pid));
                }
                None => {
                    query = query.filter(folders::parent_id.is_null());
                }
            }

            query
                .select(MaterialFolder::as_select())
                .order(folders::sort_order.asc())
                .then_order_by(folders::name.asc())
                .load(&mut conn)
        })
        .await
        .unwrap_or_else(|e| Err(DieselError::QueryBuilderError(Box::new(e))))
    }

    pub async fn list_all_by_user(&self, user_id: i32) -> Result<Vec<MaterialFolder>, DieselError> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().expect("Connection error");
            folders::table
                .filter(folders::user_id.eq(user_id))
                .select(MaterialFolder::as_select())
                .order(folders::depth.asc())
                .then_order_by(folders::sort_order.asc())
                .then_order_by(folders::name.asc())
                .load(&mut conn)
        })
        .await
        .unwrap_or_else(|e| Err(DieselError::QueryBuilderError(Box::new(e))))
    }

    pub async fn update_name(
        &self,
        id: i32,
        user_id: i32,
        name: String,
    ) -> Result<MaterialFolder, DieselError> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().expect("Connection error");
            diesel::update(
                folders::table
                    .filter(folders::id.eq(id))
                    .filter(folders::user_id.eq(user_id)),
            )
            .set((
                folders::name.eq(name),
                folders::updated_at.eq(Some(chrono::Utc::now())),
            ))
            .returning(MaterialFolder::as_returning())
            .get_result(&mut conn)
        })
        .await
        .unwrap_or_else(|e| Err(DieselError::QueryBuilderError(Box::new(e))))
    }

    pub async fn delete(&self, id: i32, user_id: i32) -> Result<usize, DieselError> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().expect("Connection error");
            diesel::delete(
                folders::table
                    .filter(folders::id.eq(id))
                    .filter(folders::user_id.eq(user_id)),
            )
            .execute(&mut conn)
        })
        .await
        .unwrap_or_else(|e| Err(DieselError::QueryBuilderError(Box::new(e))))
    }

    pub async fn has_children(&self, id: i32) -> Result<bool, DieselError> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().expect("Connection error");
            let count: i64 = folders::table
                .filter(folders::parent_id.eq(id))
                .count()
                .get_result(&mut conn)?;
            Ok(count > 0)
        })
        .await
        .unwrap_or_else(|e| Err(DieselError::QueryBuilderError(Box::new(e))))
    }

    pub async fn has_materials(&self, folder_id: i32) -> Result<bool, DieselError> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().expect("Connection error");
            let count: i64 = materials::table
                .filter(materials::folder_id.eq(folder_id))
                .filter(materials::is_active.eq(Some(true)))
                .count()
                .get_result(&mut conn)?;
            Ok(count > 0)
        })
        .await
        .unwrap_or_else(|e| Err(DieselError::QueryBuilderError(Box::new(e))))
    }

    pub async fn count_materials_in_folder(&self, folder_id: i32) -> Result<i64, DieselError> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().expect("Connection error");
            materials::table
                .filter(materials::folder_id.eq(folder_id))
                .filter(materials::is_active.eq(Some(true)))
                .count()
                .get_result(&mut conn)
        })
        .await
        .unwrap_or_else(|e| Err(DieselError::QueryBuilderError(Box::new(e))))
    }
}
