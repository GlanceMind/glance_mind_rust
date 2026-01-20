-- 添加任务名称和视频方向字段
ALTER TABLE gm_video_generation_tasks
ADD COLUMN title VARCHAR(255),
ADD COLUMN orientation VARCHAR(20) DEFAULT 'portrait';

-- 添加索引以提高查询性能
CREATE INDEX idx_video_tasks_title ON gm_video_generation_tasks(title);
CREATE INDEX idx_video_tasks_orientation ON gm_video_generation_tasks(orientation);

-- 添加注释
COMMENT ON COLUMN gm_video_generation_tasks.title IS '任务名称';
COMMENT ON COLUMN gm_video_generation_tasks.orientation IS '视频方向: portrait(竖屏9:16) 或 landscape(横屏16:9)';
