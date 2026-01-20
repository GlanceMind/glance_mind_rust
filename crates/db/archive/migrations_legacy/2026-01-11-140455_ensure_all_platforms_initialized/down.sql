-- This migration ensures platform initialization
-- Rolling back would remove all platforms and their regions
-- WARNING: This will cascade delete all related campaigns, social accounts, etc.

-- Remove all regions for the platforms (cascade will handle dependent records)
DELETE FROM gm_regions WHERE platform_id IN (
    SELECT id FROM gm_platforms 
    WHERE name IN ('REDDIT', 'TIKTOK', 'INSTAGRAM', 'TWITTER')
);

-- Remove the platforms themselves
DELETE FROM gm_platforms 
WHERE name IN ('REDDIT', 'TIKTOK', 'INSTAGRAM', 'TWITTER');
