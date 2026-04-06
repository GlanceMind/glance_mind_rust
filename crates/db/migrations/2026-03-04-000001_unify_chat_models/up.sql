-- Deactivate all old chat models
UPDATE gm_ai_models SET is_active = false WHERE model_type = 'chat';

-- Upsert 4 unified chat models
INSERT INTO gm_ai_models (id, name, provider, model_key, model_type, cost_multiplier, is_active)
VALUES
  (101, 'Gemini 3.1 Pro', 'google', 'gemini-3.1-pro-preview', 'chat', 0.5, true),
  (102, 'GPT-5.2', 'openai', 'gpt-5.2', 'chat', 1.0, true),
  (103, 'Claude Haiku Thinking', 'anthropic', 'claude-haiku-4-5-20251001-thinking', 'chat', 0.8, true),
  (104, 'Grok 4', 'xai', 'grok-4', 'chat', 1.5, true)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  provider = EXCLUDED.provider,
  model_key = EXCLUDED.model_key,
  cost_multiplier = EXCLUDED.cost_multiplier,
  is_active = EXCLUDED.is_active;
