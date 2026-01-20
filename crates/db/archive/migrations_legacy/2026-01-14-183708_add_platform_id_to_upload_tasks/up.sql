ALTER TABLE gm_upload_tasks 
ADD COLUMN platform_id INTEGER REFERENCES gm_platforms(id) ON DELETE SET NULL;

CREATE INDEX idx_upload_tasks_platform_id ON gm_upload_tasks(platform_id);
