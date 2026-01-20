-- Rollback: Remove Veo-3.1 model variants
DELETE FROM gm_ai_models 
WHERE model_key IN (
  'veo-3.1', 
  'veo-3.1-fl', 
  'veo-3.1-fast', 
  'veo-3.1-fast-fl',
  'veo-3.1-landscape', 
  'veo-3.1-landscape-fl', 
  'veo-3.1-landscape-fast', 
  'veo-3.1-landscape-fast-fl'
);
