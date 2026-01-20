-- Remove suggested DM field from agent comments table
ALTER TABLE gm_agent_comments 
DROP COLUMN IF EXISTS suggested_dm;

-- Remove DM template field from campaign templates table
ALTER TABLE gm_campaign_templates 
DROP COLUMN IF EXISTS dm_template;
