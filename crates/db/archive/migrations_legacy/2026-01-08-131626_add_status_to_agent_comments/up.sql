-- Add status field to agent_comments
-- 0=init, 1=processing, 2=completed
ALTER TABLE gm_agent_comments
ADD COLUMN status SMALLINT NOT NULL DEFAULT 0;

COMMENT ON COLUMN gm_agent_comments.status IS '0=init, 1=processing, 2=completed';

-- Add index for performance
CREATE INDEX idx_agent_comments_status ON gm_agent_comments(status);
