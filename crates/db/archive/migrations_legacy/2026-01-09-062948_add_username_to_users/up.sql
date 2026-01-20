-- Add username column to gm_users table
ALTER TABLE gm_users 
ADD COLUMN username VARCHAR(50) UNIQUE;

-- Add unique constraint on username (case-insensitive)
CREATE UNIQUE INDEX idx_users_username_unique ON gm_users (LOWER(username));

-- Make email nullable to support username-only registration
-- Note: We need to handle existing data first
ALTER TABLE gm_users 
ALTER COLUMN email DROP NOT NULL;

-- Update the unique constraint on email to allow NULL
-- The existing UNIQUE constraint still works with NULL values
