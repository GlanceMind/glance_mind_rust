use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use glance_mind_db::entity::social_group::{NewSocialGroup, SocialGroup};
use glance_mind_db::schema::gm_social_groups as social_groups;

#[derive(Clone)]
pub struct SocialGroupRepository {
    pool: Pool<ConnectionManager<PgConnection>>,
}

impl SocialGroupRepository {
    pub fn new(pool: Pool<ConnectionManager<PgConnection>>) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        new_group: NewSocialGroup,
    ) -> Result<SocialGroup, diesel::result::Error> {
        let mut conn = self.pool.get().expect("Failed to get DB connection");

        diesel::insert_into(social_groups::table)
            .values(&new_group)
            .get_result(&mut conn)
    }

    pub async fn find_by_user(
        &self,
        user_id: i32,
        page: i64,
        page_size: i64,
        platform_id: Option<i32>,
    ) -> Result<(Vec<SocialGroup>, i64), diesel::result::Error> {
        let mut conn = self.pool.get().expect("Failed to get DB connection");

        let total = {
            let mut q = social_groups::table
                .filter(social_groups::user_id.eq(user_id))
                .into_boxed();
            if let Some(p) = platform_id {
                q = q.filter(social_groups::platform_id.eq(p));
            }
            q.count().get_result(&mut conn)?
        };

        let items = {
            let mut q = social_groups::table
                .filter(social_groups::user_id.eq(user_id))
                .into_boxed();
            if let Some(p) = platform_id {
                q = q.filter(social_groups::platform_id.eq(p));
            }
            q.limit(page_size)
                .offset((page - 1) * page_size)
                .load::<SocialGroup>(&mut conn)?
        };

        Ok((items, total))
    }

    pub async fn find_by_id(
        &self,
        id: i32,
        user_id: i32,
    ) -> Result<SocialGroup, diesel::result::Error> {
        let mut conn = self.pool.get().expect("Failed to get DB connection");

        social_groups::table
            .filter(social_groups::id.eq(id))
            .filter(social_groups::user_id.eq(user_id))
            .first(&mut conn)
    }

    pub async fn update(
        &self,
        id: i32,
        user_id: i32,
        group_name: &str,
    ) -> Result<SocialGroup, diesel::result::Error> {
        let mut conn = self.pool.get().expect("Failed to get DB connection");

        diesel::update(
            social_groups::table
                .filter(social_groups::id.eq(id))
                .filter(social_groups::user_id.eq(user_id)),
        )
        .set(social_groups::group_name.eq(group_name))
        .get_result(&mut conn)
    }

    pub async fn delete(&self, id: i32, user_id: i32) -> Result<usize, diesel::result::Error> {
        let mut conn = self.pool.get().expect("Failed to get DB connection");

        diesel::delete(
            social_groups::table
                .filter(social_groups::id.eq(id))
                .filter(social_groups::user_id.eq(user_id)),
        )
        .execute(&mut conn)
    }

    /// Returns Ok(true) if the platform exists in gm_platforms, Ok(false) otherwise.
    pub fn platform_exists(&self, platform_id_param: i32) -> Result<bool, diesel::result::Error> {
        use glance_mind_db::schema::gm_platforms::dsl::*;

        let mut conn = self.pool.get().expect("Failed to get DB connection");

        let count: i64 = gm_platforms
            .filter(id.eq(platform_id_param))
            .count()
            .get_result(&mut conn)?;

        Ok(count > 0)
    }
}
