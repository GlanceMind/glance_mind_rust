-- 1. Create material_folders table
CREATE TABLE gm_material_folders (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL,
    parent_id INTEGER REFERENCES gm_material_folders(id) ON DELETE RESTRICT,
    name VARCHAR(255) NOT NULL,
    depth SMALLINT NOT NULL DEFAULT 0,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ,
    CONSTRAINT uq_folder_name UNIQUE(user_id, parent_id, name)
);

CREATE INDEX idx_material_folders_user_parent ON gm_material_folders(user_id, parent_id);

-- 2. Add new columns to gm_user_materials
ALTER TABLE gm_user_materials
    ADD COLUMN folder_id INTEGER REFERENCES gm_material_folders(id) ON DELETE RESTRICT,
    ADD COLUMN media_type VARCHAR(20) NOT NULL DEFAULT 'video',
    ADD COLUMN mime_type VARCHAR(100),
    ADD COLUMN file_url VARCHAR(1024);

CREATE INDEX idx_materials_folder ON gm_user_materials(folder_id);
CREATE INDEX idx_materials_media_type ON gm_user_materials(media_type);

-- 3. Make video_url nullable (images/audio won't have it)
ALTER TABLE gm_user_materials ALTER COLUMN video_url DROP NOT NULL;
