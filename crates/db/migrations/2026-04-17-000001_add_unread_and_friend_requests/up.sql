ALTER TABLE gm_patrol_account_stats
    ADD COLUMN received_friend_requests INT NOT NULL DEFAULT 0,
    ADD COLUMN unread_total INT NOT NULL DEFAULT 0;
