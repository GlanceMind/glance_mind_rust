-- Simplify Vidu models: remove per-resolution splits and old versions,
-- keep one model per generation mode + fast generation + template
-- Old one-click endpoints (ad_film, trending_replicate, oneclick) replaced by template2video

-- Step 1: Deactivate all old Vidu models
UPDATE gm_ai_models SET is_active = false
WHERE provider = 'vidu' AND model_key IN (
    'vidu-2.0-t2v-720p', 'vidu-2.0-t2v-1080p',
    'vidu-2.0-i2v-720p', 'vidu-2.0-i2v-1080p',
    'vidu-2.0-ref2v', 'vidu-2.0-startend', 'vidu-2.0-multiframe',
    'vidu-1.5-t2v', 'vidu-q1',
    'vidu-ad-film', 'vidu-trending-replicate', 'vidu-oneclick'
);

-- Step 2: Insert simplified models
INSERT INTO gm_ai_models (name, provider, model_key, model_type, cost_multiplier, is_active)
VALUES
    ('Vidu 文生视频',   'vidu', 'vidu-t2v',        'video', 1.5, true),
    ('Vidu 图生视频',   'vidu', 'vidu-i2v',        'video', 2.0, true),
    ('Vidu 参考生视频', 'vidu', 'vidu-ref2v',      'video', 3.0, true),
    ('Vidu 首尾帧',     'vidu', 'vidu-startend',   'video', 2.5, true),
    ('Vidu 智能多帧',   'vidu', 'vidu-multiframe', 'video', 4.0, true),
    ('Vidu 快速生成',   'vidu', 'vidu-fast',       'video', 1.0, true),
    ('Vidu 模板视频',   'vidu', 'vidu-template',   'video', 3.0, true)
ON CONFLICT (model_key) DO UPDATE SET
    name = EXCLUDED.name,
    cost_multiplier = EXCLUDED.cost_multiplier,
    is_active = true;
