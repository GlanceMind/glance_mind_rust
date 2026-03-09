-- Revert: deactivate simplified models, re-activate old ones
UPDATE gm_ai_models SET is_active = false
WHERE provider = 'vidu' AND model_key IN (
    'vidu-t2v', 'vidu-i2v', 'vidu-ref2v', 'vidu-startend',
    'vidu-multiframe', 'vidu-fast', 'vidu-template'
);

UPDATE gm_ai_models SET is_active = true
WHERE provider = 'vidu' AND model_key IN (
    'vidu-2.0-t2v-720p', 'vidu-2.0-t2v-1080p',
    'vidu-2.0-i2v-720p', 'vidu-2.0-i2v-1080p',
    'vidu-2.0-ref2v', 'vidu-2.0-startend', 'vidu-2.0-multiframe',
    'vidu-1.5-t2v', 'vidu-q1',
    'vidu-ad-film', 'vidu-trending-replicate', 'vidu-oneclick'
);
