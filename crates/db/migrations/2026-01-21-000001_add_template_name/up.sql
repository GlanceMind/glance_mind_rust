-- Add name column to gm_campaign_templates table
ALTER TABLE gm_campaign_templates ADD COLUMN IF NOT EXISTS name VARCHAR(255);

-- Update existing templates with default names
UPDATE gm_campaign_templates SET name = 'Template #' || id WHERE name IS NULL;
