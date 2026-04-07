-- Add GLM-5 chat model from LaoZhang provider
INSERT INTO gm_ai_models (id, name, provider, model_key, model_type, cost_multiplier, is_active)
VALUES (105, 'GLM-5', 'laozhang', 'glm-5', 'chat', 1.0, true)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  provider = EXCLUDED.provider,
  model_key = EXCLUDED.model_key,
  cost_multiplier = EXCLUDED.cost_multiplier,
  is_active = EXCLUDED.is_active;
