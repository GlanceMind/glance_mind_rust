use crate::config::database::DBPool;
use diesel::prelude::*;
use diesel::result::Error as DieselError;
use diesel::SelectableHelper;
use glance_mind_db::entity::platform::Platform;
use glance_mind_db::entity::region::Region;
use glance_mind_db::schema::{gm_platforms as platforms, gm_regions as regions};

#[derive(Clone)]
pub struct PlatformRepository {
    pool: DBPool,
}

impl PlatformRepository {
    pub fn new(pool: DBPool) -> Self {
        Self { pool }
    }

    pub async fn find_all(&self) -> Result<Vec<Platform>, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        platforms::table
            .filter(platforms::is_active.eq(true))
            .select(Platform::as_select())
            .load(&mut conn)
    }

    pub async fn find_regions_by_platform(
        &self,
        platform_id: i32,
    ) -> Result<Vec<Region>, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        regions::table
            .filter(regions::platform_id.eq(platform_id))
            .filter(regions::is_active.eq(true))
            .select(Region::as_select())
            .load(&mut conn)
    }

    pub async fn find_all_regions(&self) -> Result<Vec<Region>, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        regions::table
            .filter(regions::is_active.eq(true))
            .select(Region::as_select())
            .load(&mut conn)
    }
}
