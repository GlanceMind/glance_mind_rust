-- Ensure regions exist for all platforms
-- This is a fix for foreign key constraint violations when creating campaigns

-- Re-run region initialization in case it was missed
-- This is idempotent and safe to run multiple times

-- TikTok Regions (most commonly used)
INSERT INTO gm_regions (platform_id, code, display_name, is_active, name)
SELECT p.id, 'GLOBAL', 'Global', true, 'GLOBAL'
FROM gm_platforms p
WHERE p.name = 'TIKTOK'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'GLOBAL'
);

INSERT INTO gm_regions (platform_id, code, display_name, is_active, name)
SELECT p.id, 'US', 'United States', true, 'US'
FROM gm_platforms p
WHERE p.name = 'TIKTOK'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'US'
);

INSERT INTO gm_regions (platform_id, code, display_name, is_active, name)
SELECT p.id, 'CN', 'China', true, 'CN'
FROM gm_platforms p
WHERE p.name = 'TIKTOK'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'CN'
);

-- Reddit Regions
INSERT INTO gm_regions (platform_id, code, display_name, is_active, name)
SELECT p.id, 'GLOBAL', 'Global', true, 'GLOBAL'
FROM gm_platforms p
WHERE p.name = 'REDDIT'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'GLOBAL'
);

-- Instagram Regions
INSERT INTO gm_regions (platform_id, code, display_name, is_active, name)
SELECT p.id, 'GLOBAL', 'Global', true, 'GLOBAL'
FROM gm_platforms p
WHERE p.name = 'INSTAGRAM'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'GLOBAL'
);

-- Twitter Regions
INSERT INTO gm_regions (platform_id, code, display_name, is_active, name)
SELECT p.id, 'GLOBAL', 'Global', true, 'GLOBAL'
FROM gm_platforms p
WHERE p.name = 'TWITTER'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'GLOBAL'
);
