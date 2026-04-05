-- ============================================================================
-- fn_estimate_aipub_plan_cost: Pre-submit cost estimation for AI Publish plans
-- ============================================================================
-- Returns a single-row table with the breakdown: chat, video, image, total.
-- Used by the API to provide real-time cost preview on the create-plan page.
-- Formula per account:
--   chat_cost  = AI_ANALYZE  base × chat_model  multiplier
--   video_cost = VIDEO_GENERATE base × video_model multiplier  (0 when NULL)
--   image_cost = IMAGE base × image_model multiplier            (0 when NULL)
-- total = (chat_cost + video_cost + image_cost) × account_count
-- ============================================================================

CREATE OR REPLACE FUNCTION fn_estimate_aipub_plan_cost(
    p_chat_model_id   INT DEFAULT NULL,
    p_video_model_id  INT DEFAULT NULL,
    p_image_model_id  INT DEFAULT NULL,
    p_account_count   INT DEFAULT 1
) RETURNS TABLE (
    chat_unit_cost   NUMERIC,
    video_unit_cost  NUMERIC,
    image_unit_cost  NUMERIC,
    per_account_cost NUMERIC,
    account_count    INT,
    total_cost       NUMERIC,
    pricing_snapshot JSONB
) AS $$
DECLARE
    v_chat_base    NUMERIC;
    v_video_base   NUMERIC;
    v_image_base   NUMERIC;
    v_chat_mult    NUMERIC := 1.0;
    v_video_mult   NUMERIC := 1.0;
    v_image_mult   NUMERIC := 1.0;
    v_chat_cost    NUMERIC;
    v_video_cost   NUMERIC;
    v_image_cost   NUMERIC;
    v_per_account  NUMERIC;
    v_total        NUMERIC;
    v_acct_count   INT;
    v_chat_model_name  TEXT := NULL;
    v_video_model_name TEXT := NULL;
    v_image_model_name TEXT := NULL;
BEGIN
    v_acct_count := GREATEST(COALESCE(p_account_count, 1), 1);

    -- ① Chat base price (AI_ANALYZE, global)
    SELECT COALESCE(cost_points, 1.0) INTO v_chat_base
    FROM gm_pricing_rules
    WHERE action_type = 'AI_ANALYZE' AND platform_id IS NULL
    LIMIT 1;
    v_chat_base := COALESCE(v_chat_base, 1.0);

    -- ② Video base price (VIDEO_GENERATE or VIDEO, global)
    SELECT COALESCE(cost_points, 0) INTO v_video_base
    FROM gm_pricing_rules
    WHERE action_type = 'VIDEO_GENERATE' AND platform_id IS NULL
    LIMIT 1;
    IF v_video_base IS NULL OR v_video_base = 0 THEN
        SELECT COALESCE(cost_points, 0) INTO v_video_base
        FROM gm_pricing_rules
        WHERE action_type = 'VIDEO' AND platform_id IS NULL
        LIMIT 1;
    END IF;
    v_video_base := COALESCE(v_video_base, 0);

    -- ③ Image base price (IMAGE, global)
    SELECT COALESCE(cost_points, 0) INTO v_image_base
    FROM gm_pricing_rules
    WHERE action_type = 'IMAGE' AND platform_id IS NULL
    LIMIT 1;
    v_image_base := COALESCE(v_image_base, 0);

    -- ④ Model multipliers + names
    IF p_chat_model_id IS NOT NULL THEN
        SELECT COALESCE(m.cost_multiplier, 1.0), m.name
        INTO v_chat_mult, v_chat_model_name
        FROM gm_ai_models m WHERE m.id = p_chat_model_id AND m.is_active = true;
        v_chat_mult := COALESCE(v_chat_mult, 1.0);
    END IF;

    IF p_video_model_id IS NOT NULL THEN
        SELECT COALESCE(m.cost_multiplier, 1.0), m.name
        INTO v_video_mult, v_video_model_name
        FROM gm_ai_models m WHERE m.id = p_video_model_id AND m.is_active = true;
        v_video_mult := COALESCE(v_video_mult, 1.0);
    ELSE
        v_video_base := 0;
    END IF;

    IF p_image_model_id IS NOT NULL THEN
        SELECT COALESCE(m.cost_multiplier, 1.0), m.name
        INTO v_image_mult, v_image_model_name
        FROM gm_ai_models m WHERE m.id = p_image_model_id AND m.is_active = true;
        v_image_mult := COALESCE(v_image_mult, 1.0);
    ELSE
        v_image_base := 0;
    END IF;

    -- ⑤ Calculate
    v_chat_cost   := v_chat_base  * v_chat_mult;
    v_video_cost  := v_video_base * v_video_mult;
    v_image_cost  := v_image_base * v_image_mult;
    v_per_account := v_chat_cost + v_video_cost + v_image_cost;
    v_total       := v_per_account * v_acct_count;

    RETURN QUERY SELECT
        v_chat_cost,
        v_video_cost,
        v_image_cost,
        v_per_account,
        v_acct_count,
        v_total,
        jsonb_build_object(
            'chat_base',  v_chat_base,  'chat_mult',  v_chat_mult,  'chat_model',  v_chat_model_name,
            'video_base', v_video_base, 'video_mult', v_video_mult, 'video_model', v_video_model_name,
            'image_base', v_image_base, 'image_mult', v_image_mult, 'image_model', v_image_model_name
        );
END;
$$ LANGUAGE plpgsql STABLE;

COMMENT ON FUNCTION fn_estimate_aipub_plan_cost IS
'Pre-submit cost estimation for AI Publish plans.
Returns per-account breakdown (chat + video + image) and total = per_account × account_count.
Used by POST /publish_plans/estimate for real-time cost preview.';
