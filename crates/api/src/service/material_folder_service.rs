use crate::config::database::Database;
use crate::dto::material_folder_dto::{
    CreateFolderRequest, FolderDetail, FolderListResponse, FolderTreeNode, FolderTreeResponse,
    UpdateFolderRequest,
};
use crate::error::{api_error::ApiError, db_error::DbError};
use crate::repository::material_folder_repository::MaterialFolderRepository;
use chrono::Utc;
use glance_mind_db::entity::material_folder::NewMaterialFolder;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct MaterialFolderService {
    folder_repo: MaterialFolderRepository,
}

impl MaterialFolderService {
    pub fn new(db_conn: &Arc<Database>) -> Self {
        Self {
            folder_repo: MaterialFolderRepository::new(db_conn.pool.clone()),
        }
    }

    pub async fn create_folder(
        &self,
        user_id: i32,
        request: CreateFolderRequest,
    ) -> Result<FolderDetail, ApiError> {
        let depth: i16 = if let Some(pid) = request.parent_id {
            let parent = self
                .folder_repo
                .find_by_id_and_user(pid, user_id)
                .await
                .map_err(|_| ApiError::NotFound("Parent folder not found".into()))?;
            if parent.depth >= 2 {
                return Err(ApiError::BadRequest(
                    "Maximum folder depth (3 levels) reached".into(),
                ));
            }
            parent.depth + 1
        } else {
            0
        };

        let new_folder = NewMaterialFolder {
            user_id,
            parent_id: request.parent_id,
            name: request.name,
            depth,
            sort_order: 0,
            created_at: Utc::now(),
        };

        let folder = self.folder_repo.create(new_folder).await.map_err(|e| {
            let msg = e.to_string();
            if msg.contains("unique") || msg.contains("duplicate") {
                ApiError::BadRequest("A folder with this name already exists here".into())
            } else {
                ApiError::from(DbError::SomethingWentWrong(msg))
            }
        })?;

        Ok(FolderDetail::from(folder))
    }

    pub async fn list_folders(
        &self,
        user_id: i32,
        parent_id: Option<i32>,
    ) -> Result<FolderListResponse, ApiError> {
        let folders = self
            .folder_repo
            .list_by_parent(user_id, parent_id)
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        let mut result: Vec<FolderDetail> = Vec::new();
        for f in folders {
            let count = self
                .folder_repo
                .count_materials_in_folder(f.id)
                .await
                .unwrap_or(0);
            let mut detail = FolderDetail::from(f);
            detail.material_count = count;
            result.push(detail);
        }

        Ok(FolderListResponse { folders: result })
    }

    pub async fn get_folder_tree(&self, user_id: i32) -> Result<FolderTreeResponse, ApiError> {
        let all_folders = self
            .folder_repo
            .list_all_by_user(user_id)
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        let mut counts: HashMap<i32, i64> = HashMap::new();
        for f in &all_folders {
            let count = self
                .folder_repo
                .count_materials_in_folder(f.id)
                .await
                .unwrap_or(0);
            counts.insert(f.id, count);
        }

        let mut nodes: HashMap<i32, FolderTreeNode> = HashMap::new();
        for f in &all_folders {
            nodes.insert(
                f.id,
                FolderTreeNode {
                    id: f.id,
                    name: f.name.clone(),
                    parent_id: f.parent_id,
                    depth: f.depth,
                    material_count: *counts.get(&f.id).unwrap_or(&0),
                    children: vec![],
                },
            );
        }

        let mut root_ids: Vec<i32> = Vec::new();
        let child_parent_pairs: Vec<(i32, i32)> = all_folders
            .iter()
            .filter_map(|f| f.parent_id.map(|pid| (f.id, pid)))
            .collect();

        for f in &all_folders {
            if f.parent_id.is_none() {
                root_ids.push(f.id);
            }
        }

        for (child_id, parent_id) in child_parent_pairs {
            if let Some(child_node) = nodes.remove(&child_id) {
                if let Some(parent_node) = nodes.get_mut(&parent_id) {
                    parent_node.children.push(child_node);
                }
            }
        }

        let folders: Vec<FolderTreeNode> = root_ids
            .into_iter()
            .filter_map(|id| nodes.remove(&id))
            .collect();

        Ok(FolderTreeResponse { folders })
    }

    pub async fn update_folder(
        &self,
        id: i32,
        user_id: i32,
        request: UpdateFolderRequest,
    ) -> Result<FolderDetail, ApiError> {
        if let Some(ref name) = request.name {
            let folder = self
                .folder_repo
                .update_name(id, user_id, name.clone())
                .await
                .map_err(|e| {
                    let msg = e.to_string();
                    if msg.contains("unique") || msg.contains("duplicate") {
                        ApiError::BadRequest("A folder with this name already exists here".into())
                    } else if msg.contains("NotFound") {
                        ApiError::NotFound("Folder not found".into())
                    } else {
                        ApiError::from(DbError::SomethingWentWrong(msg))
                    }
                })?;
            Ok(FolderDetail::from(folder))
        } else {
            let folder = self
                .folder_repo
                .find_by_id_and_user(id, user_id)
                .await
                .map_err(|_| ApiError::NotFound("Folder not found".into()))?;
            Ok(FolderDetail::from(folder))
        }
    }

    pub async fn delete_folder(&self, id: i32, user_id: i32) -> Result<(), ApiError> {
        self.folder_repo
            .find_by_id_and_user(id, user_id)
            .await
            .map_err(|_| ApiError::NotFound("Folder not found".into()))?;

        if self
            .folder_repo
            .has_children(id)
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?
        {
            return Err(ApiError::BadRequest(
                "Cannot delete folder with sub-folders. Delete them first.".into(),
            ));
        }

        if self
            .folder_repo
            .has_materials(id)
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?
        {
            return Err(ApiError::BadRequest(
                "Cannot delete folder with materials. Move them first.".into(),
            ));
        }

        self.folder_repo
            .delete(id, user_id)
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        Ok(())
    }
}
