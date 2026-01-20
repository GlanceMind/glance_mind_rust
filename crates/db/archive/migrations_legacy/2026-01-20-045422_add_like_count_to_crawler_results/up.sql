-- Add share_count and play_count columns to gm_crawler_results table
ALTER TABLE gm_crawler_results 
ADD COLUMN IF NOT EXISTS share_count INTEGER,
ADD COLUMN IF NOT EXISTS play_count INTEGER;

-- Add comments
COMMENT ON COLUMN gm_crawler_results.share_count IS '视频分享数';
COMMENT ON COLUMN gm_crawler_results.play_count IS '视频播放数';
