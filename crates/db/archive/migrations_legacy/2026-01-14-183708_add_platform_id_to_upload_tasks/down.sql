DROP INDEX IF EXISTS idx_upload_tasks_platform_id;
ALTER TABLE gm_upload_tasks DROP COLUMN platform_id;
