-- Drop indexes
DROP INDEX IF EXISTS idx_notifications_type;
DROP INDEX IF EXISTS idx_notifications_published;
DROP INDEX IF EXISTS idx_notification_reads_user;

-- Drop tables
DROP TABLE IF EXISTS gm_user_notification_reads;
DROP TABLE IF EXISTS gm_notifications;
