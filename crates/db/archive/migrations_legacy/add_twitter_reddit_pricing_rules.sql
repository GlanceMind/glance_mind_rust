-- Add pricing rules for Twitter and Reddit platforms
-- This ensures the scheduler can calculate costs for all supported platforms

DO $$
DECLARE
    twitter_platform_id INTEGER;
    reddit_platform_id INTEGER;
BEGIN
    -- Get platform IDs
    SELECT id INTO twitter_platform_id FROM gm_platforms WHERE name = 'TWITTER';
    SELECT id INTO reddit_platform_id FROM gm_platforms WHERE name = 'REDDIT';

    IF twitter_platform_id IS NULL THEN
        RAISE WARNING 'Twitter platform not found. Skipping Twitter pricing rules.';
    END IF;

    IF reddit_platform_id IS NULL THEN
        RAISE WARNING 'Reddit platform not found. Skipping Reddit pricing rules.';
    END IF;

    -- Add pricing rules for Twitter
    IF twitter_platform_id IS NOT NULL THEN
        INSERT INTO gm_pricing_rules (action_type, platform_id, cost_points, description)
        VALUES 
            ('SCAN_POST', twitter_platform_id, 0.5, 'Cost for scanning Twitter posts'),
            ('AI_ANALYZE', twitter_platform_id, 1.0, 'Cost for AI analysis of Twitter comments')
        ON CONFLICT (action_type, platform_id) DO UPDATE
            SET cost_points = EXCLUDED.cost_points,
                description = EXCLUDED.description;

        RAISE NOTICE 'Successfully added/updated pricing rules for Twitter (platform_id=%)', twitter_platform_id;
    END IF;

    -- Add pricing rules for Reddit
    IF reddit_platform_id IS NOT NULL THEN
        INSERT INTO gm_pricing_rules (action_type, platform_id, cost_points, description)
        VALUES 
            ('SCAN_POST', reddit_platform_id, 0.5, 'Cost for scanning Reddit posts'),
            ('AI_ANALYZE', reddit_platform_id, 1.0, 'Cost for AI analysis of Reddit comments')
        ON CONFLICT (action_type, platform_id) DO UPDATE
            SET cost_points = EXCLUDED.cost_points,
                description = EXCLUDED.description;

        RAISE NOTICE 'Successfully added/updated pricing rules for Reddit (platform_id=%)', reddit_platform_id;
    END IF;
END $$;

-- Verify all pricing rules
SELECT 
    p.name AS platform_name,
    pr.action_type,
    pr.cost_points,
    pr.description
FROM gm_pricing_rules pr
JOIN gm_platforms p ON p.id = pr.platform_id
WHERE p.name IN ('TWITTER', 'REDDIT', 'INSTAGRAM', 'TIKTOK')
ORDER BY p.name, pr.action_type;
