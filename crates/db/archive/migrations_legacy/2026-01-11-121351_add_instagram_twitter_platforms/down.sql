-- Remove Instagram and Twitter platforms and their regions
DELETE FROM gm_regions WHERE platform_id IN (
    SELECT id FROM gm_platforms WHERE name IN ('INSTAGRAM', 'TWITTER')
);

DELETE FROM gm_platforms WHERE name IN ('INSTAGRAM', 'TWITTER');
