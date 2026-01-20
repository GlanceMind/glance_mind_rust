-- Add search_options column to gm_campaigns table
ALTER TABLE gm_campaigns 
ADD COLUMN search_options JSONB DEFAULT '{}'::jsonb;

-- Add comment to explain the field
COMMENT ON COLUMN gm_campaigns.search_options IS 'Advanced search customization parameters in JSON format. Platform-specific search configuration options such as region, sort type, safe search, feed type, etc.';

-- Example search_options structure:
-- {
--   "tiktok": {
--     "region": "US",
--     "sort_type": "1",
--     "publish_time": "7"
--   },
--   "reddit": {
--     "safe_search": "strict",
--     "allow_nsfw": "0",
--     "comment_sort": "top"
--   },
--   "instagram": {
--     "feed_type": "reels",
--     "comment_sort": "recent"
--   },
--   "twitter": {
--     "search_type": "Latest"
--   }
-- }
