-- Add DM template field to campaign templates table
ALTER TABLE gm_campaign_templates 
ADD COLUMN dm_template TEXT;

-- Add suggested DM field to agent comments table  
ALTER TABLE gm_agent_comments 
ADD COLUMN suggested_dm TEXT;

-- Add comment for documentation
COMMENT ON COLUMN gm_campaign_templates.dm_template IS 'Direct message template for personalized outreach';
COMMENT ON COLUMN gm_agent_comments.suggested_dm IS 'AI-generated suggested direct message for this comment';
