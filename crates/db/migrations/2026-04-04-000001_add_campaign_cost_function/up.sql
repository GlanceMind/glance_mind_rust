-- Calculate minimum campaign execution cost based on pricing rules
-- Used by both API (preflight validation) and Scheduler (budget check)
CREATE OR REPLACE FUNCTION calculate_min_campaign_cost(
    p_platform_id INTEGER,
    p_scan_count INTEGER,
    p_ai_model_id INTEGER DEFAULT NULL
) RETURNS NUMERIC AS $$
DECLARE
    v_scan_cost NUMERIC;
    v_ai_cost NUMERIC;
    v_multiplier NUMERIC;
    v_min_cost NUMERIC;
BEGIN
    -- Get SCAN_POST base cost for this platform
    SELECT COALESCE(cost_points, 5)
    INTO v_scan_cost
    FROM gm_pricing_rules
    WHERE action_type = 'SCAN_POST'
      AND (platform_id = p_platform_id OR platform_id IS NULL)
    ORDER BY platform_id DESC NULLS LAST
    LIMIT 1;

    IF v_scan_cost IS NULL THEN
        v_scan_cost := 5;
    END IF;

    -- Get AI_ANALYZE base cost for this platform
    SELECT COALESCE(cost_points, 2)
    INTO v_ai_cost
    FROM gm_pricing_rules
    WHERE action_type = 'AI_ANALYZE'
      AND (platform_id = p_platform_id OR platform_id IS NULL)
    ORDER BY platform_id DESC NULLS LAST
    LIMIT 1;

    IF v_ai_cost IS NULL THEN
        v_ai_cost := 2;
    END IF;

    -- Get AI model cost multiplier
    IF p_ai_model_id IS NOT NULL THEN
        SELECT COALESCE(cost_multiplier, 1)
        INTO v_multiplier
        FROM gm_ai_models
        WHERE id = p_ai_model_id AND is_active = true;

        IF v_multiplier IS NULL THEN
            v_multiplier := 1;
        END IF;
    ELSE
        v_multiplier := 1;
    END IF;

    -- min_cost = (scan_cost + ai_cost × multiplier) × scan_count
    -- scan_cost: per scan invocation (crawl target video comments)
    -- ai_cost: per comment AI analysis (at least 1 comment per scan)
    -- account_count is NOT a factor — scanning is per-video, not per-account
    v_min_cost := (v_scan_cost + v_ai_cost * v_multiplier)
                  * GREATEST(p_scan_count, 1);

    RETURN v_min_cost;
END;
$$ LANGUAGE plpgsql STABLE;

COMMENT ON FUNCTION calculate_min_campaign_cost IS 
'Calculate minimum campaign execution cost. Used by API preflight and Scheduler budget check.
Formula: (SCAN_POST_cost + AI_ANALYZE_cost × model_multiplier) × scan_count
Note: account_count is NOT a factor — scanning is per-video, not per-account.';
