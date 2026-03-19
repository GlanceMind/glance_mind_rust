use crate::config::database::DBPool;
use diesel::prelude::*;
use diesel::result::Error as DieselError;
use glance_mind_db::entity::notification::{
    NewUserNotificationRead, Notification, UserNotificationRead,
};
use glance_mind_db::schema::{gm_notifications, gm_user_notification_reads};

#[derive(Clone)]
pub struct NotificationRepository {
    pool: DBPool,
}

impl NotificationRepository {
    pub fn new(pool: DBPool) -> Self {
        Self { pool }
    }

    /// Get all active notifications (published and not expired)
    pub async fn find_all_active(&self) -> Result<Vec<Notification>, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        let now = chrono::Utc::now();

        gm_notifications::table
            .filter(gm_notifications::published_at.le(now))
            .filter(
                gm_notifications::expires_at
                    .is_null()
                    .or(gm_notifications::expires_at.gt(now)),
            )
            .order(gm_notifications::published_at.desc())
            .limit(50)
            .load(&mut conn)
    }

    /// Get read notification IDs for a user
    pub async fn find_read_ids_by_user(&self, user_id: i32) -> Result<Vec<i32>, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");

        gm_user_notification_reads::table
            .filter(gm_user_notification_reads::user_id.eq(user_id))
            .select(gm_user_notification_reads::notification_id)
            .load(&mut conn)
    }

    /// Get unread count for a user
    pub async fn count_unread(&self, user_id: i32) -> Result<i64, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        let now = chrono::Utc::now();

        // Count active notifications that are not in the user's read list
        let total_active: i64 = gm_notifications::table
            .filter(gm_notifications::published_at.le(now))
            .filter(
                gm_notifications::expires_at
                    .is_null()
                    .or(gm_notifications::expires_at.gt(now)),
            )
            .count()
            .get_result(&mut conn)?;

        let read_count: i64 = gm_user_notification_reads::table
            .inner_join(gm_notifications::table)
            .filter(gm_user_notification_reads::user_id.eq(user_id))
            .filter(gm_notifications::published_at.le(now))
            .filter(
                gm_notifications::expires_at
                    .is_null()
                    .or(gm_notifications::expires_at.gt(now)),
            )
            .count()
            .get_result(&mut conn)?;

        Ok(total_active - read_count)
    }

    /// Mark a single notification as read
    pub async fn mark_as_read(
        &self,
        user_id: i32,
        notification_id: i32,
    ) -> Result<UserNotificationRead, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");

        let new_read = NewUserNotificationRead {
            user_id,
            notification_id,
        };

        diesel::insert_into(gm_user_notification_reads::table)
            .values(&new_read)
            .on_conflict((
                gm_user_notification_reads::user_id,
                gm_user_notification_reads::notification_id,
            ))
            .do_nothing()
            .get_result(&mut conn)
            .or_else(|_| {
                // If insert returned nothing (conflict), fetch the existing record
                gm_user_notification_reads::table
                    .filter(gm_user_notification_reads::user_id.eq(user_id))
                    .filter(gm_user_notification_reads::notification_id.eq(notification_id))
                    .first(&mut conn)
            })
    }

    /// Mark all active notifications as read for a user
    pub async fn mark_all_as_read(&self, user_id: i32) -> Result<usize, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        let now = chrono::Utc::now();

        // Get all active notification IDs
        let active_ids: Vec<i32> = gm_notifications::table
            .filter(gm_notifications::published_at.le(now))
            .filter(
                gm_notifications::expires_at
                    .is_null()
                    .or(gm_notifications::expires_at.gt(now)),
            )
            .select(gm_notifications::id)
            .load(&mut conn)?;

        // Get already read IDs
        let read_ids: Vec<i32> = gm_user_notification_reads::table
            .filter(gm_user_notification_reads::user_id.eq(user_id))
            .select(gm_user_notification_reads::notification_id)
            .load(&mut conn)?;

        // Insert new read records for unread notifications
        let mut inserted = 0;
        for notification_id in active_ids {
            if !read_ids.contains(&notification_id) {
                let new_read = NewUserNotificationRead {
                    user_id,
                    notification_id,
                };
                let result = diesel::insert_into(gm_user_notification_reads::table)
                    .values(&new_read)
                    .on_conflict((
                        gm_user_notification_reads::user_id,
                        gm_user_notification_reads::notification_id,
                    ))
                    .do_nothing()
                    .execute(&mut conn);
                if result.is_ok() {
                    inserted += 1;
                }
            }
        }

        Ok(inserted)
    }
}
