use crate::config::database::Database;
use glance_mind_db::entity::platform::Platform;
use glance_mind_db::entity::region::Region;
use crate::repository::platform_repository::PlatformRepository;
use diesel::result::Error as DieselError;
use std::sync::Arc;

#[derive(Clone)]
pub struct PlatformService {
    repo: PlatformRepository,
}

impl PlatformService {
    pub fn new(db: &Arc<Database>) -> Self {
        Self {
            repo: PlatformRepository::new(db.pool.clone()),
        }
    }

    pub async fn get_all_platforms(&self) -> Result<Vec<Platform>, DieselError> {
        self.repo.find_all().await
    }

    pub async fn get_regions_by_platform(
        &self,
        platform_id: i32,
    ) -> Result<Vec<Region>, DieselError> {
        self.repo.find_regions_by_platform(platform_id).await
    }
}
