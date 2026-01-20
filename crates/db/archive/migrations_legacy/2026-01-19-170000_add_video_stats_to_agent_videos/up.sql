-- 为 gm_agent_videos 表添加视频统计数据字段

-- 添加点赞数
ALTER TABLE gm_agent_videos ADD COLUMN IF NOT EXISTS like_count INTEGER DEFAULT 0;

-- 添加评论数
ALTER TABLE gm_agent_videos ADD COLUMN IF NOT EXISTS comment_count INTEGER DEFAULT 0;

-- 添加分享数
ALTER TABLE gm_agent_videos ADD COLUMN IF NOT EXISTS share_count INTEGER DEFAULT 0;

-- 添加播放数
ALTER TABLE gm_agent_videos ADD COLUMN IF NOT EXISTS play_count INTEGER DEFAULT 0;

-- 添加发布时间（Unix timestamp）
ALTER TABLE gm_agent_videos ADD COLUMN IF NOT EXISTS publish_time BIGINT DEFAULT 0;

-- 添加作者唯一ID
ALTER TABLE gm_agent_videos ADD COLUMN IF NOT EXISTS author_unique_id VARCHAR(255);

-- 添加视频URL
ALTER TABLE gm_agent_videos ADD COLUMN IF NOT EXISTS url TEXT;

-- 添加注释
COMMENT ON COLUMN gm_agent_videos.like_count IS '视频点赞数';
COMMENT ON COLUMN gm_agent_videos.comment_count IS '视频评论数';
COMMENT ON COLUMN gm_agent_videos.share_count IS '视频分享数';
COMMENT ON COLUMN gm_agent_videos.play_count IS '视频播放数';
COMMENT ON COLUMN gm_agent_videos.publish_time IS '视频发布时间 (Unix timestamp)';
COMMENT ON COLUMN gm_agent_videos.author_unique_id IS '作者唯一ID (TikTok unique_id)';
COMMENT ON COLUMN gm_agent_videos.url IS '视频分享链接';
