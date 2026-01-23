-- Migration: Unify comments status field type across all platforms
-- Change status from VARCHAR(50) to SMALLINT to match TikTok's gm_agent_comments table
-- Status values: 0=pending, 1=processing, 2=completed
-- This migration is idempotent - safe to run on databases that have already been migrated

-- ============================================================================
-- Facebook Comments
-- ============================================================================
DO $$
BEGIN
    -- Only migrate if status column exists and is NOT already smallint
    IF EXISTS (
        SELECT 1 FROM information_schema.columns 
        WHERE table_name = 'gm_agent_facebook_comments' 
        AND column_name = 'status' 
        AND data_type != 'smallint'
    ) THEN
        -- Add new column
        ALTER TABLE gm_agent_facebook_comments ADD COLUMN status_new SMALLINT DEFAULT 0;
        
        -- Migrate data
        UPDATE gm_agent_facebook_comments SET status_new = 
            CASE UPPER(COALESCE(status::text, 'PENDING'))
                WHEN 'PENDING' THEN 0
                WHEN 'PROCESSING' THEN 1
                WHEN 'COMPLETED' THEN 2
                ELSE 0
            END;
        
        -- Drop old column and rename new one
        ALTER TABLE gm_agent_facebook_comments DROP COLUMN status;
        ALTER TABLE gm_agent_facebook_comments RENAME COLUMN status_new TO status;
        
        RAISE NOTICE 'Facebook comments status column migrated to SMALLINT';
    ELSE
        RAISE NOTICE 'Facebook comments status column already is SMALLINT, skipping';
    END IF;
END $$;

-- ============================================================================
-- Instagram Comments
-- ============================================================================
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns 
        WHERE table_name = 'gm_agent_instagram_comments' 
        AND column_name = 'status' 
        AND data_type != 'smallint'
    ) THEN
        ALTER TABLE gm_agent_instagram_comments ADD COLUMN status_new SMALLINT DEFAULT 0;
        
        UPDATE gm_agent_instagram_comments SET status_new = 
            CASE UPPER(COALESCE(status::text, 'PENDING'))
                WHEN 'PENDING' THEN 0
                WHEN 'PROCESSING' THEN 1
                WHEN 'COMPLETED' THEN 2
                ELSE 0
            END;
        
        ALTER TABLE gm_agent_instagram_comments DROP COLUMN status;
        ALTER TABLE gm_agent_instagram_comments RENAME COLUMN status_new TO status;
        
        RAISE NOTICE 'Instagram comments status column migrated to SMALLINT';
    ELSE
        RAISE NOTICE 'Instagram comments status column already is SMALLINT, skipping';
    END IF;
END $$;

-- ============================================================================
-- Reddit Comments
-- ============================================================================
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns 
        WHERE table_name = 'gm_agent_reddit_comments' 
        AND column_name = 'status' 
        AND data_type != 'smallint'
    ) THEN
        ALTER TABLE gm_agent_reddit_comments ADD COLUMN status_new SMALLINT DEFAULT 0;
        
        UPDATE gm_agent_reddit_comments SET status_new = 
            CASE UPPER(COALESCE(status::text, 'PENDING'))
                WHEN 'PENDING' THEN 0
                WHEN 'PROCESSING' THEN 1
                WHEN 'COMPLETED' THEN 2
                ELSE 0
            END;
        
        ALTER TABLE gm_agent_reddit_comments DROP COLUMN status;
        ALTER TABLE gm_agent_reddit_comments RENAME COLUMN status_new TO status;
        
        RAISE NOTICE 'Reddit comments status column migrated to SMALLINT';
    ELSE
        RAISE NOTICE 'Reddit comments status column already is SMALLINT, skipping';
    END IF;
END $$;

-- ============================================================================
-- Twitter Comments
-- ============================================================================
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns 
        WHERE table_name = 'gm_agent_twitter_comments' 
        AND column_name = 'status' 
        AND data_type != 'smallint'
    ) THEN
        ALTER TABLE gm_agent_twitter_comments ADD COLUMN status_new SMALLINT DEFAULT 0;
        
        UPDATE gm_agent_twitter_comments SET status_new = 
            CASE UPPER(COALESCE(status::text, 'PENDING'))
                WHEN 'PENDING' THEN 0
                WHEN 'PROCESSING' THEN 1
                WHEN 'COMPLETED' THEN 2
                ELSE 0
            END;
        
        ALTER TABLE gm_agent_twitter_comments DROP COLUMN status;
        ALTER TABLE gm_agent_twitter_comments RENAME COLUMN status_new TO status;
        
        RAISE NOTICE 'Twitter comments status column migrated to SMALLINT';
    ELSE
        RAISE NOTICE 'Twitter comments status column already is SMALLINT, skipping';
    END IF;
END $$;
