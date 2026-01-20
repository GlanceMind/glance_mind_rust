-- 回滚: 删除 gm_agent_videos 表的统计数据字段
ALTER TABLE gm_agent_videos DROP COLUMN IF EXISTS like_count;
ALTER TABLE gm_agent_videos DROP COLUMN IF EXISTS comment_count;
ALTER TABLE gm_agent_videos DROP COLUMN IF EXISTS share_count;
ALTER TABLE gm_agent_videos DROP COLUMN IF EXISTS play_count;
ALTER TABLE gm_agent_videos DROP COLUMN IF EXISTS publish_time;
ALTER TABLE gm_agent_videos DROP COLUMN IF EXISTS author_unique_id;
ALTER TABLE gm_agent_videos DROP COLUMN IF EXISTS url;
