-- Deactivate all old chat models
UPDATE gm_ai_models SET is_active = false WHERE model_type = 'chat';

-- Upsert 4 unified chat models
INSERT INTO gm_ai_models (id, name, provider, model_key, model_type, cost_multiplier, is_active)
VALUES
  (101, 'Claude Haiku', 'anthropic', 'claude-haiku-4-5-20251001', 'chat', 0.3, true),
  (102, 'Claude Sonnet', 'anthropic', 'claude-sonnet-4-6', 'chat', 1.0, true),
  (103, 'GPT-5.2', 'openai', 'gpt-5.2', 'chat', 1.2, true),
  (104, 'Claude Opus', 'anthropic', 'claude-opus-4-6', 'chat', 2.0, true)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  provider = EXCLUDED.provider,
  model_key = EXCLUDED.model_key,
  cost_multiplier = EXCLUDED.cost_multiplier,
  is_active = EXCLUDED.is_active;
