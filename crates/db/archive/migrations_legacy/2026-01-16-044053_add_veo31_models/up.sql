-- Insert Veo-3.1 model variants into gm_ai_models table
-- These models provide various options for video generation with different speeds and orientations

-- 竖屏模型 (Portrait/Vertical 720x1280)
INSERT INTO gm_ai_models (model_key, name, provider, cost_multiplier, is_active, model_type, created_at)
SELECT 'veo-3.1', 'Veo 3.1 (竖屏)', 'laozhang', 1.2, true, 'video', NOW()
WHERE NOT EXISTS (SELECT 1 FROM gm_ai_models WHERE model_key = 'veo-3.1');

INSERT INTO gm_ai_models (model_key, name, provider, cost_multiplier, is_active, model_type, created_at)
SELECT 'veo-3.1-fl', 'Veo 3.1 FL (竖屏图生视频)', 'laozhang', 1.3, true, 'video', NOW()
WHERE NOT EXISTS (SELECT 1 FROM gm_ai_models WHERE model_key = 'veo-3.1-fl');

INSERT INTO gm_ai_models (model_key, name, provider, cost_multiplier, is_active, model_type, created_at)
SELECT 'veo-3.1-fast', 'Veo 3.1 Fast (竖屏快速)', 'laozhang', 0.8, true, 'video', NOW()
WHERE NOT EXISTS (SELECT 1 FROM gm_ai_models WHERE model_key = 'veo-3.1-fast');

INSERT INTO gm_ai_models (model_key, name, provider, cost_multiplier, is_active, model_type, created_at)
SELECT 'veo-3.1-fast-fl', 'Veo 3.1 Fast FL (竖屏快速图生)', 'laozhang', 0.9, true, 'video', NOW()
WHERE NOT EXISTS (SELECT 1 FROM gm_ai_models WHERE model_key = 'veo-3.1-fast-fl');

-- 横屏模型 (Landscape 1280x720)
INSERT INTO gm_ai_models (model_key, name, provider, cost_multiplier, is_active, model_type, created_at)
SELECT 'veo-3.1-landscape', 'Veo 3.1 Landscape (横屏)', 'laozhang', 1.2, true, 'video', NOW()
WHERE NOT EXISTS (SELECT 1 FROM gm_ai_models WHERE model_key = 'veo-3.1-landscape');

INSERT INTO gm_ai_models (model_key, name, provider, cost_multiplier, is_active, model_type, created_at)
SELECT 'veo-3.1-landscape-fl', 'Veo 3.1 Landscape FL (横屏图生)', 'laozhang', 1.3, true, 'video', NOW()
WHERE NOT EXISTS (SELECT 1 FROM gm_ai_models WHERE model_key = 'veo-3.1-landscape-fl');

INSERT INTO gm_ai_models (model_key, name, provider, cost_multiplier, is_active, model_type, created_at)
SELECT 'veo-3.1-landscape-fast', 'Veo 3.1 Landscape Fast (横屏快速)', 'laozhang', 0.8, true, 'video', NOW()
WHERE NOT EXISTS (SELECT 1 FROM gm_ai_models WHERE model_key = 'veo-3.1-landscape-fast');

INSERT INTO gm_ai_models (model_key, name, provider, cost_multiplier, is_active, model_type, created_at)
SELECT 'veo-3.1-landscape-fast-fl', 'Veo 3.1 Landscape Fast FL (横屏快速图生)', 'laozhang', 0.9, true, 'video', NOW()
WHERE NOT EXISTS (SELECT 1 FROM gm_ai_models WHERE model_key = 'veo-3.1-landscape-fast-fl');

-- Model naming conventions:
-- - Standard: Higher quality, slower generation
-- - Fast: Lower cost, faster generation (0.8x multiplier)
-- - FL (First/Last Frame): Support image-to-video generation (1.3x multiplier)
-- - Landscape: 16:9 aspect ratio for horizontal videos
