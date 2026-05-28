-- Admin console edits `gm_auto_reply_config` with JWT identities from
-- `gm_admin_users`, while `last_modified_by` remains FK to `gm_users`.
ALTER TABLE gm_auto_reply_config
    ADD COLUMN last_modified_by_admin_user_id INTEGER
        REFERENCES gm_admin_users (id);

COMMENT ON COLUMN gm_auto_reply_config.last_modified_by_admin_user_id IS
    'Admin user who last changed this row via glance_mind_admin (JWT sub).';
