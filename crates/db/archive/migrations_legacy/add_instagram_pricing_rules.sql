-- Add pricing rules for Instagram (platform_id=4)
-- This ensures the scheduler can calculate costs for Instagram campaigns

-- First, check if Instagram platform exists
DO $$
DECLARE
    instagram_platform_id INTEGER;
BEGIN
    -- Get Instagram platform ID
    SELECT id INTO instagram_platform_id
    FROM gm_platforms
    WHERE name = 'INSTAGRAM';

    IF instagram_platform_id IS NULL THEN
        RAISE EXCEPTION 'Instagram platform not found. Please run platform initialization migrations first.';
    END IF;

    -- Add pricing rules for Instagram
    -- SCAN_POST: Cost for scanning/fetching posts
    INSERT INTO gm_pricing_rules (action_type, platform_id, cost_points, description)
    VALUES (
        'SCAN_POST',
        instagram_platform_id,
        0.5,
        'Cost for scanning Instagram posts/reels'
    )
    ON CONFLICT (action_type, platform_id) DO UPDATE
        SET cost_points = EXCLUDED.cost_points,
            description = EXCLUDED.description;

    -- AI_ANALYZE: Cost for AI analysis of comments
    INSERT INTO gm_pricing_rules (action_type, platform_id, cost_points, description)
    VALUES (
        'AI_ANALYZE',
        instagram_platform_id,
        1.0,
        'Cost for AI analysis of Instagram comments'
    )
    ON CONFLICT (action_type, platform_id) DO UPDATE
        SET cost_points = EXCLUDED.cost_points,
            description = EXCLUDED.description;

    RAISE NOTICE 'Successfully added/updated pricing rules for Instagram (platform_id=%)', instagram_platform_id;
END $$;

-- Verify the pricing rules
SELECT 
    p.name AS platform_name,
    pr.action_type,
    pr.cost_points,
    pr.description
FROM gm_pricing_rules pr
JOIN gm_platforms p ON p.id = pr.platform_id
WHERE p.name = 'INSTAGRAM'
ORDER BY pr.action_type;
