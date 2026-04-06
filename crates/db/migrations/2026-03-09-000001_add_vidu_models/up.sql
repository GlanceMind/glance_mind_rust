-- Add Vidu video generation models to gm_ai_models
-- Vidu API: https://api.vidu.com (V2 API)
-- Models: viduq1 (5s/1080p), vidu1.5 (4s/8s), vidu2.0 (4s/8s)

-- Step 1: Remove duplicate model_key rows (keep the one with the lowest id)
DELETE FROM gm_ai_models a
USING gm_ai_models b
WHERE a.model_key = b.model_key
  AND a.id > b.id;

-- Step 2: Now safe to create the unique index
CREATE UNIQUE INDEX IF NOT EXISTS idx_gm_ai_models_model_key ON gm_ai_models (model_key);

-- Step 3: Insert Vidu models
INSERT INTO gm_ai_models (name, provider, model_key, model_type, cost_multiplier, is_active)
VALUES
    -- Vidu 2.0 (latest, best quality)
    ('Vidu 2.0 文生视频 720P',   'vidu', 'vidu-2.0-t2v-720p',   'video', 2.0, true),
    ('Vidu 2.0 文生视频 1080P',  'vidu', 'vidu-2.0-t2v-1080p',  'video', 3.0, true),
    ('Vidu 2.0 图生视频 720P',   'vidu', 'vidu-2.0-i2v-720p',   'video', 2.5, true),
    ('Vidu 2.0 图生视频 1080P',  'vidu', 'vidu-2.0-i2v-1080p',  'video', 3.5, true),
    ('Vidu 2.0 参考生视频',      'vidu', 'vidu-2.0-ref2v',      'video', 4.0, true),
    ('Vidu 2.0 首尾帧生视频',    'vidu', 'vidu-2.0-startend',   'video', 3.0, true),
    ('Vidu 2.0 智能多帧',        'vidu', 'vidu-2.0-multiframe', 'video', 5.0, true),

    -- Vidu 1.5
    ('Vidu 1.5 视频生成',        'vidu', 'vidu-1.5-t2v',        'video', 1.5, true),

    -- Vidu Q1 (fast, 5s only)
    ('Vidu Q1 快速生成',         'vidu', 'vidu-q1',             'video', 1.0, true),

    -- Vidu one-click solutions
    ('Vidu 一键电商成片',        'vidu', 'vidu-ad-film',        'video', 6.0, true),
    ('Vidu 一键视频复刻',        'vidu', 'vidu-trending-replicate', 'video', 6.0, true),
    ('Vidu 一键通用成片',        'vidu', 'vidu-oneclick',       'video', 5.0, true)
ON CONFLICT (model_key) DO NOTHING;
