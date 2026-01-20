-- Ensure all supported platforms are initialized
-- This migration is idempotent and can be safely run on existing databases

-- =============================================================================
-- Step 1: Ensure all platforms exist
-- =============================================================================

-- Reddit
INSERT INTO gm_platforms (name, display_name, is_active, base_url) 
VALUES ('REDDIT', 'Reddit', true, '')
ON CONFLICT (name) DO NOTHING;

-- TikTok
INSERT INTO gm_platforms (name, display_name, is_active, base_url) 
VALUES ('TIKTOK', 'TikTok', true, '')
ON CONFLICT (name) DO NOTHING;

-- Instagram
INSERT INTO gm_platforms (name, display_name, is_active, base_url) 
VALUES ('INSTAGRAM', 'Instagram', true, '')
ON CONFLICT (name) DO NOTHING;

-- Twitter / X
INSERT INTO gm_platforms (name, display_name, is_active, base_url) 
VALUES ('TWITTER', 'Twitter / X', true, '')
ON CONFLICT (name) DO NOTHING;

-- =============================================================================
-- Step 2: Ensure all platforms have standard regions
-- =============================================================================

-- Reddit Regions
INSERT INTO gm_regions (platform_id, code, display_name, is_active)
SELECT p.id, 'GLOBAL', 'Global', true
FROM gm_platforms p
WHERE p.name = 'REDDIT'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'GLOBAL'
);

INSERT INTO gm_regions (platform_id, code, display_name, is_active)
SELECT p.id, 'US', 'United States', true
FROM gm_platforms p
WHERE p.name = 'REDDIT'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'US'
);

INSERT INTO gm_regions (platform_id, code, display_name, is_active)
SELECT p.id, 'UK', 'United Kingdom', true
FROM gm_platforms p
WHERE p.name = 'REDDIT'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'UK'
);

INSERT INTO gm_regions (platform_id, code, display_name, is_active)
SELECT p.id, 'JP', 'Japan', true
FROM gm_platforms p
WHERE p.name = 'REDDIT'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'JP'
);

INSERT INTO gm_regions (platform_id, code, display_name, is_active)
SELECT p.id, 'CN', 'China', true
FROM gm_platforms p
WHERE p.name = 'REDDIT'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'CN'
);

INSERT INTO gm_regions (platform_id, code, display_name, is_active)
SELECT p.id, 'EU', 'European Union', true
FROM gm_platforms p
WHERE p.name = 'REDDIT'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'EU'
);

INSERT INTO gm_regions (platform_id, code, display_name, is_active)
SELECT p.id, 'KR', 'South Korea', true
FROM gm_platforms p
WHERE p.name = 'REDDIT'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'KR'
);

INSERT INTO gm_regions (platform_id, code, display_name, is_active)
SELECT p.id, 'MY', 'Malaysia', true
FROM gm_platforms p
WHERE p.name = 'REDDIT'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'MY'
);

INSERT INTO gm_regions (platform_id, code, display_name, is_active)
SELECT p.id, 'SG', 'Singapore', true
FROM gm_platforms p
WHERE p.name = 'REDDIT'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'SG'
);

INSERT INTO gm_regions (platform_id, code, display_name, is_active)
SELECT p.id, 'AE', 'United Arab Emirates', true
FROM gm_platforms p
WHERE p.name = 'REDDIT'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'AE'
);

INSERT INTO gm_regions (platform_id, code, display_name, is_active)
SELECT p.id, 'DE', 'Germany', true
FROM gm_platforms p
WHERE p.name = 'REDDIT'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'DE'
);

-- TikTok Regions
INSERT INTO gm_regions (platform_id, code, display_name, is_active)
SELECT p.id, 'GLOBAL', 'Global', true
FROM gm_platforms p
WHERE p.name = 'TIKTOK'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'GLOBAL'
);

INSERT INTO gm_regions (platform_id, code, display_name, is_active)
SELECT p.id, 'US', 'United States', true
FROM gm_platforms p
WHERE p.name = 'TIKTOK'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'US'
);

INSERT INTO gm_regions (platform_id, code, display_name, is_active)
SELECT p.id, 'UK', 'United Kingdom', true
FROM gm_platforms p
WHERE p.name = 'TIKTOK'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'UK'
);

INSERT INTO gm_regions (platform_id, code, display_name, is_active)
SELECT p.id, 'JP', 'Japan', true
FROM gm_platforms p
WHERE p.name = 'TIKTOK'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'JP'
);

INSERT INTO gm_regions (platform_id, code, display_name, is_active)
SELECT p.id, 'CN', 'China', true
FROM gm_platforms p
WHERE p.name = 'TIKTOK'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'CN'
);

INSERT INTO gm_regions (platform_id, code, display_name, is_active)
SELECT p.id, 'EU', 'European Union', true
FROM gm_platforms p
WHERE p.name = 'TIKTOK'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'EU'
);

INSERT INTO gm_regions (platform_id, code, display_name, is_active)
SELECT p.id, 'KR', 'South Korea', true
FROM gm_platforms p
WHERE p.name = 'TIKTOK'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'KR'
);

INSERT INTO gm_regions (platform_id, code, display_name, is_active)
SELECT p.id, 'MY', 'Malaysia', true
FROM gm_platforms p
WHERE p.name = 'TIKTOK'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'MY'
);

INSERT INTO gm_regions (platform_id, code, display_name, is_active)
SELECT p.id, 'SG', 'Singapore', true
FROM gm_platforms p
WHERE p.name = 'TIKTOK'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'SG'
);

INSERT INTO gm_regions (platform_id, code, display_name, is_active)
SELECT p.id, 'AE', 'United Arab Emirates', true
FROM gm_platforms p
WHERE p.name = 'TIKTOK'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'AE'
);

INSERT INTO gm_regions (platform_id, code, display_name, is_active)
SELECT p.id, 'DE', 'Germany', true
FROM gm_platforms p
WHERE p.name = 'TIKTOK'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'DE'
);

-- Instagram Regions
INSERT INTO gm_regions (platform_id, code, display_name, is_active)
SELECT p.id, 'GLOBAL', 'Global', true
FROM gm_platforms p
WHERE p.name = 'INSTAGRAM'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'GLOBAL'
);

INSERT INTO gm_regions (platform_id, code, display_name, is_active)
SELECT p.id, 'US', 'United States', true
FROM gm_platforms p
WHERE p.name = 'INSTAGRAM'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'US'
);

INSERT INTO gm_regions (platform_id, code, display_name, is_active)
SELECT p.id, 'UK', 'United Kingdom', true
FROM gm_platforms p
WHERE p.name = 'INSTAGRAM'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'UK'
);

INSERT INTO gm_regions (platform_id, code, display_name, is_active)
SELECT p.id, 'JP', 'Japan', true
FROM gm_platforms p
WHERE p.name = 'INSTAGRAM'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'JP'
);

INSERT INTO gm_regions (platform_id, code, display_name, is_active)
SELECT p.id, 'CN', 'China', true
FROM gm_platforms p
WHERE p.name = 'INSTAGRAM'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'CN'
);

INSERT INTO gm_regions (platform_id, code, display_name, is_active)
SELECT p.id, 'EU', 'European Union', true
FROM gm_platforms p
WHERE p.name = 'INSTAGRAM'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'EU'
);

INSERT INTO gm_regions (platform_id, code, display_name, is_active)
SELECT p.id, 'KR', 'South Korea', true
FROM gm_platforms p
WHERE p.name = 'INSTAGRAM'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'KR'
);

INSERT INTO gm_regions (platform_id, code, display_name, is_active)
SELECT p.id, 'MY', 'Malaysia', true
FROM gm_platforms p
WHERE p.name = 'INSTAGRAM'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'MY'
);

INSERT INTO gm_regions (platform_id, code, display_name, is_active)
SELECT p.id, 'SG', 'Singapore', true
FROM gm_platforms p
WHERE p.name = 'INSTAGRAM'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'SG'
);

INSERT INTO gm_regions (platform_id, code, display_name, is_active)
SELECT p.id, 'AE', 'United Arab Emirates', true
FROM gm_platforms p
WHERE p.name = 'INSTAGRAM'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'AE'
);

INSERT INTO gm_regions (platform_id, code, display_name, is_active)
SELECT p.id, 'DE', 'Germany', true
FROM gm_platforms p
WHERE p.name = 'INSTAGRAM'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'DE'
);

-- Twitter Regions
INSERT INTO gm_regions (platform_id, code, display_name, is_active)
SELECT p.id, 'GLOBAL', 'Global', true
FROM gm_platforms p
WHERE p.name = 'TWITTER'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'GLOBAL'
);

INSERT INTO gm_regions (platform_id, code, display_name, is_active)
SELECT p.id, 'US', 'United States', true
FROM gm_platforms p
WHERE p.name = 'TWITTER'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'US'
);

INSERT INTO gm_regions (platform_id, code, display_name, is_active)
SELECT p.id, 'UK', 'United Kingdom', true
FROM gm_platforms p
WHERE p.name = 'TWITTER'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'UK'
);

INSERT INTO gm_regions (platform_id, code, display_name, is_active)
SELECT p.id, 'JP', 'Japan', true
FROM gm_platforms p
WHERE p.name = 'TWITTER'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'JP'
);

INSERT INTO gm_regions (platform_id, code, display_name, is_active)
SELECT p.id, 'CN', 'China', true
FROM gm_platforms p
WHERE p.name = 'TWITTER'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'CN'
);

INSERT INTO gm_regions (platform_id, code, display_name, is_active)
SELECT p.id, 'EU', 'European Union', true
FROM gm_platforms p
WHERE p.name = 'TWITTER'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'EU'
);

INSERT INTO gm_regions (platform_id, code, display_name, is_active)
SELECT p.id, 'KR', 'South Korea', true
FROM gm_platforms p
WHERE p.name = 'TWITTER'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'KR'
);

INSERT INTO gm_regions (platform_id, code, display_name, is_active)
SELECT p.id, 'MY', 'Malaysia', true
FROM gm_platforms p
WHERE p.name = 'TWITTER'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'MY'
);

INSERT INTO gm_regions (platform_id, code, display_name, is_active)
SELECT p.id, 'SG', 'Singapore', true
FROM gm_platforms p
WHERE p.name = 'TWITTER'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'SG'
);

INSERT INTO gm_regions (platform_id, code, display_name, is_active)
SELECT p.id, 'AE', 'United Arab Emirates', true
FROM gm_platforms p
WHERE p.name = 'TWITTER'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'AE'
);

INSERT INTO gm_regions (platform_id, code, display_name, is_active)
SELECT p.id, 'DE', 'Germany', true
FROM gm_platforms p
WHERE p.name = 'TWITTER'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'DE'
);
