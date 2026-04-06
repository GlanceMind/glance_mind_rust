-- ============================================================================
-- Revert pricing schedule refresh
-- ============================================================================

WITH legacy_platform_rules(platform_id, action_type, cost_points) AS (
    VALUES
        (1, 'SCAN_POST',      0.50::numeric),
        (1, 'AI_ANALYZE',     1.00::numeric),
        (1, 'REPLY_COMMENT',  2.00::numeric),
        (2, 'SCAN_POST',      0.50::numeric),
        (2, 'AI_ANALYZE',     1.00::numeric),
        (2, 'REPLY_COMMENT',  2.00::numeric),
        (3, 'SCAN_POST',      0.50::numeric),
        (3, 'AI_ANALYZE',     1.00::numeric),
        (3, 'REPLY_COMMENT',  2.00::numeric),
        (4, 'SCAN_POST',      0.50::numeric),
        (4, 'AI_ANALYZE',     1.00::numeric),
        (4, 'REPLY_COMMENT',  2.00::numeric),
        (5, 'SCAN_POST',      0.50::numeric),
        (5, 'AI_ANALYZE',     1.00::numeric),
        (5, 'REPLY_COMMENT',  2.00::numeric)
)
INSERT INTO gm_pricing_rules (platform_id, action_type, cost_points)
SELECT platform_id, action_type, cost_points
FROM legacy_platform_rules
ON CONFLICT (action_type, platform_id) DO UPDATE
SET cost_points = EXCLUDED.cost_points;

DELETE FROM gm_pricing_rules
WHERE action_type = 'POST_REPLY'
  AND (
      platform_id IS NULL
      OR platform_id IN (1, 2, 3, 4, 5)
  );

DELETE FROM gm_pricing_rules
WHERE action_type = 'REPLY_COMMENT'
  AND platform_id IS NULL;

INSERT INTO gm_pricing_rules (platform_id, action_type, cost_points)
VALUES
    (NULL, 'AI_ANALYZE',      1.00),
    (NULL, 'IMAGE',           5.00),
    (NULL, 'VIDEO_GENERATE', 200.00)
ON CONFLICT (action_type) WHERE platform_id IS NULL DO UPDATE
SET cost_points = EXCLUDED.cost_points;

WITH legacy_video_models(model_key, cost_multiplier) AS (
    VALUES
        ('sora2',                    1.00::numeric),
        ('veo-2',                    5.00::numeric),
        ('jimeng-video-3.0-720p',    2.00::numeric),
        ('jimeng-video-3.0-1080p',   3.00::numeric),
        ('jimeng-video-3.0-pro',     5.00::numeric),
        ('vidu-fast',                1.00::numeric),
        ('vidu-t2v',                 1.50::numeric),
        ('vidu-i2v',                 2.00::numeric),
        ('vidu-startend',            2.50::numeric),
        ('vidu-ref2v',               3.00::numeric),
        ('vidu-template',            3.00::numeric),
        ('vidu-multiframe',          4.00::numeric),
        ('vidu-general-film',        5.00::numeric),
        ('vidu-ad-film',             6.00::numeric)
)
UPDATE gm_ai_models AS models
SET cost_multiplier = legacy.cost_multiplier
FROM legacy_video_models AS legacy
WHERE models.model_key = legacy.model_key;
