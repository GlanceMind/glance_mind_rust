-- 更新 Veo FL 模型的名称，说明支持 1-2 张图片输入
UPDATE gm_ai_models 
SET name = 'Veo 3.1 FL (竖屏图生，支持1-2张图片)'
WHERE model_key = 'veo-3.1-fl';

UPDATE gm_ai_models 
SET name = 'Veo 3.1 Fast FL (竖屏快速图生，支持1-2张图片)'
WHERE model_key = 'veo-3.1-fast-fl';

UPDATE gm_ai_models 
SET name = 'Veo 3.1 Landscape FL (横屏图生，支持1-2张图片)'
WHERE model_key = 'veo-3.1-landscape-fl';

UPDATE gm_ai_models 
SET name = 'Veo 3.1 Landscape Fast FL (横屏快速图生，支持1-2张图片)'
WHERE model_key = 'veo-3.1-landscape-fast-fl';
