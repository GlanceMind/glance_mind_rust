-- Revert: make video_url NOT NULL (fill empty values first)
UPDATE gm_user_materials SET video_url = '' WHERE video_url IS NULL;
ALTER TABLE gm_user_materials ALTER COLUMN video_url SET NOT NULL;

-- Drop new columns from gm_user_materials
DROP INDEX IF EXISTS idx_materials_media_type;
DROP INDEX IF EXISTS idx_materials_folder;
ALTER TABLE gm_user_materials
    DROP COLUMN IF EXISTS file_url,
    DROP COLUMN IF EXISTS mime_type,
    DROP COLUMN IF EXISTS media_type,
    DROP COLUMN IF EXISTS folder_id;

-- Drop folders table
DROP INDEX IF EXISTS idx_material_folders_user_parent;
DROP TABLE IF EXISTS gm_material_folders;
