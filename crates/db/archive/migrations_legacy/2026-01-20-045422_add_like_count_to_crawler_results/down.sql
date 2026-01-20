-- This file should undo anything in `up.sql`
ALTER TABLE gm_crawler_results 
DROP COLUMN IF EXISTS share_count,
DROP COLUMN IF EXISTS play_count;
