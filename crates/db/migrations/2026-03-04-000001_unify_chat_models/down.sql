-- Remove the new unified chat models
DELETE FROM gm_ai_models WHERE id IN (101, 102, 103, 104);

-- Re-activate old chat models
UPDATE gm_ai_models SET is_active = true WHERE model_type = 'chat';
