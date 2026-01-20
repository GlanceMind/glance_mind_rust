-- 回滚：删除索引和字段
DROP INDEX IF EXISTS idx_video_tasks_orientation;
DROP INDEX IF EXISTS idx_video_tasks_title;

ALTER TABLE gm_video_generation_tasks
DROP COLUMN IF EXISTS orientation,
DROP COLUMN IF EXISTS title;
