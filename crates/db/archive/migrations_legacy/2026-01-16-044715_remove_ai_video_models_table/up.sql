-- Remove the redundant gm_ai_video_models table
-- All video model queries now use gm_ai_models with model_type='video' filter

DROP TABLE IF EXISTS gm_ai_video_models;
