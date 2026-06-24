-- Revert: drop the users contact phone column.
ALTER TABLE gm_users
    DROP COLUMN phone;
