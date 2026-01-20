-- 删除定价规则
DELETE FROM gm_pricing_rules WHERE action_type = 'video_generate';
DELETE FROM gm_pricing_rules WHERE action_type = 'video_upload_image';

-- 删除索引（会随表自动删除，但为了完整性列出）
DROP INDEX IF EXISTS idx_video_tasks_status_updated;
DROP INDEX IF EXISTS idx_video_tasks_created_at;
DROP INDEX IF EXISTS idx_video_tasks_status;
DROP INDEX IF EXISTS idx_video_tasks_task_id;
DROP INDEX IF EXISTS idx_video_tasks_user_id;

-- 删除表
DROP TABLE IF EXISTS gm_video_generation_tasks;
