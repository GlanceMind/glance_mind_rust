-- Add Instagram and Twitter platforms
INSERT INTO gm_platforms (name, display_name, is_active, base_url) 
VALUES 
    ('INSTAGRAM', 'Instagram', true, ''),
    ('TWITTER', 'Twitter / X', true, '')
ON CONFLICT (name) DO NOTHING;

-- Add regions for Instagram (reuse existing regions)
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
SELECT p.id, 'CN', 'China', true
FROM gm_platforms p
WHERE p.name = 'INSTAGRAM'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'CN'
);

-- Add regions for Twitter (reuse existing regions)
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
SELECT p.id, 'CN', 'China', true
FROM gm_platforms p
WHERE p.name = 'TWITTER'
AND NOT EXISTS (
    SELECT 1 FROM gm_regions r 
    WHERE r.platform_id = p.id AND r.code = 'CN'
);
