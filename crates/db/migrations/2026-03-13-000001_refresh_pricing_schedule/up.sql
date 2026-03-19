-- ============================================================================
-- Refresh pricing schedule to points-first March 2026 policy
-- ============================================================================
-- Source of truth:
--   100 points = 1 CNY
--   Social scan = 2 points
--   AI analyze = 1 point
--   Reply/post actions = 0 points
--   Image generation = 10 points
--   Video base = 400 points with per-model multipliers
-- ============================================================================

-- Platform-scoped social interaction pricing
WITH desired_platform_rules(platform_id, action_type, cost_points) AS (
    VALUES
        (1, 'SCAN_POST',      2.00::numeric),
        (1, 'AI_ANALYZE',     1.00::numeric),
        (1, 'REPLY_COMMENT',  0.00::numeric),
        (1, 'POST_REPLY',     0.00::numeric),
        (2, 'SCAN_POST',      2.00::numeric),
        (2, 'AI_ANALYZE',     1.00::numeric),
        (2, 'REPLY_COMMENT',  0.00::numeric),
        (2, 'POST_REPLY',     0.00::numeric),
        (3, 'SCAN_POST',      2.00::numeric),
        (3, 'AI_ANALYZE',     1.00::numeric),
        (3, 'REPLY_COMMENT',  0.00::numeric),
        (3, 'POST_REPLY',     0.00::numeric),
        (4, 'SCAN_POST',      2.00::numeric),
        (4, 'AI_ANALYZE',     1.00::numeric),
        (4, 'REPLY_COMMENT',  0.00::numeric),
        (4, 'POST_REPLY',     0.00::numeric),
        (5, 'SCAN_POST',      2.00::numeric),
        (5, 'AI_ANALYZE',     1.00::numeric),
        (5, 'REPLY_COMMENT',  0.00::numeric),
        (5, 'POST_REPLY',     0.00::numeric)
)
INSERT INTO gm_pricing_rules (platform_id, action_type, cost_points)
SELECT d.platform_id, d.action_type, d.cost_points
FROM desired_platform_rules d
WHERE EXISTS (SELECT 1 FROM gm_platforms p WHERE p.id = d.platform_id)
ON CONFLICT (action_type, platform_id) DO UPDATE
SET cost_points = EXCLUDED.cost_points;

-- Global pricing used by AIPub/video billing flows.
INSERT INTO gm_pricing_rules (platform_id, action_type, cost_points)
VALUES
    (NULL, 'AI_ANALYZE',    1.00),
    (NULL, 'IMAGE',        10.00),
    (NULL, 'VIDEO_GENERATE', 400.00),
    (NULL, 'REPLY_COMMENT',  0.00),
    (NULL, 'POST_REPLY',     0.00)
ON CONFLICT (action_type) WHERE platform_id IS NULL DO UPDATE
SET cost_points = EXCLUDED.cost_points;

-- Video model multipliers. Final cost = 400 points * multiplier.
WITH desired_video_models(model_key, cost_multiplier) AS (
    VALUES
        ('sora2',                    1.00::numeric),
        ('veo-2',                    1.00::numeric),
        ('jimeng-video-3.0-720p',    2.00::numeric),
        ('jimeng-video-3.0-1080p',   3.00::numeric),
        ('jimeng-video-3.0-pro',     4.00::numeric),
        ('vidu-fast',                1.00::numeric),
        ('vidu-t2v',                 1.50::numeric),
        ('vidu-i2v',                 2.00::numeric),
        ('vidu-startend',            2.00::numeric),
        ('vidu-ref2v',               2.50::numeric),
        ('vidu-template',            2.50::numeric),
        ('vidu-multiframe',          3.00::numeric),
        ('vidu-general-film',        3.00::numeric),
        ('vidu-ad-film',             3.75::numeric)
)
UPDATE gm_ai_models AS models
SET cost_multiplier = desired.cost_multiplier
FROM desired_video_models AS desired
WHERE models.model_key = desired.model_key;
