-- Rollback: Revert comments status field type from SMALLINT back to VARCHAR(50)
-- Status values: 0->PENDING, 2->COMPLETED (no intermediate state)
-- This rollback is idempotent - safe to run on databases that haven't been migrated

-- ============================================================================
-- Facebook Comments
-- ============================================================================
DO $$
BEGIN
    -- Only rollback if status column exists and IS smallint
    IF EXISTS (
        SELECT 1 FROM information_schema.columns 
        WHERE table_name = 'gm_agent_facebook_comments' 
        AND column_name = 'status' 
        AND data_type = 'smallint'
    ) THEN
        ALTER TABLE gm_agent_facebook_comments ADD COLUMN status_old VARCHAR(50) DEFAULT 'PENDING';
        
        UPDATE gm_agent_facebook_comments SET status_old = 
            CASE status
                WHEN 0 THEN 'PENDING'
                WHEN 1 THEN 'PROCESSING'
                WHEN 2 THEN 'COMPLETED'
                ELSE 'PENDING'
            END;
        
        ALTER TABLE gm_agent_facebook_comments DROP COLUMN status;
        ALTER TABLE gm_agent_facebook_comments RENAME COLUMN status_old TO status;
        
        RAISE NOTICE 'Facebook comments status column reverted to VARCHAR';
    ELSE
        RAISE NOTICE 'Facebook comments status column is not SMALLINT, skipping rollback';
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
        AND data_type = 'smallint'
    ) THEN
        ALTER TABLE gm_agent_instagram_comments ADD COLUMN status_old VARCHAR(50) DEFAULT 'PENDING';
        
        UPDATE gm_agent_instagram_comments SET status_old = 
            CASE status
                WHEN 0 THEN 'PENDING'
                WHEN 1 THEN 'PROCESSING'
                WHEN 2 THEN 'COMPLETED'
                ELSE 'PENDING'
            END;
        
        ALTER TABLE gm_agent_instagram_comments DROP COLUMN status;
        ALTER TABLE gm_agent_instagram_comments RENAME COLUMN status_old TO status;
        
        RAISE NOTICE 'Instagram comments status column reverted to VARCHAR';
    ELSE
        RAISE NOTICE 'Instagram comments status column is not SMALLINT, skipping rollback';
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
        AND data_type = 'smallint'
    ) THEN
        ALTER TABLE gm_agent_reddit_comments ADD COLUMN status_old VARCHAR(50) DEFAULT 'PENDING';
        
        UPDATE gm_agent_reddit_comments SET status_old = 
            CASE status
                WHEN 0 THEN 'PENDING'
                WHEN 1 THEN 'PROCESSING'
                WHEN 2 THEN 'COMPLETED'
                ELSE 'PENDING'
            END;
        
        ALTER TABLE gm_agent_reddit_comments DROP COLUMN status;
        ALTER TABLE gm_agent_reddit_comments RENAME COLUMN status_old TO status;
        
        RAISE NOTICE 'Reddit comments status column reverted to VARCHAR';
    ELSE
        RAISE NOTICE 'Reddit comments status column is not SMALLINT, skipping rollback';
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
        AND data_type = 'smallint'
    ) THEN
        ALTER TABLE gm_agent_twitter_comments ADD COLUMN status_old VARCHAR(50) DEFAULT 'PENDING';
        
        UPDATE gm_agent_twitter_comments SET status_old = 
            CASE status
                WHEN 0 THEN 'PENDING'
                WHEN 1 THEN 'PROCESSING'
                WHEN 2 THEN 'COMPLETED'
                ELSE 'PENDING'
            END;
        
        ALTER TABLE gm_agent_twitter_comments DROP COLUMN status;
        ALTER TABLE gm_agent_twitter_comments RENAME COLUMN status_old TO status;
        
        RAISE NOTICE 'Twitter comments status column reverted to VARCHAR';
    ELSE
        RAISE NOTICE 'Twitter comments status column is not SMALLINT, skipping rollback';
    END IF;
END $$;
