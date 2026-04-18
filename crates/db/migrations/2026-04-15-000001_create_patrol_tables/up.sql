CREATE TABLE gm_patrol_reports (
    id              SERIAL PRIMARY KEY,
    report_id       VARCHAR(36) UNIQUE NOT NULL,
    report_type     VARCHAR(20) NOT NULL,
    user_id         INT NOT NULL REFERENCES gm_users(id),
    device_id       VARCHAR(255) NOT NULL,
    started_at      TIMESTAMPTZ NOT NULL,
    completed_at    TIMESTAMPTZ NOT NULL,
    total_accounts  INT NOT NULL DEFAULT 0,
    success_count   INT NOT NULL DEFAULT 0,
    error_count     INT NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE gm_patrol_account_stats (
    id                      SERIAL PRIMARY KEY,
    report_id               VARCHAR(36) NOT NULL REFERENCES gm_patrol_reports(report_id),
    report_type             VARCHAR(20) NOT NULL,
    social_account_id       INT NOT NULL,
    device_id               VARCHAR(255) NOT NULL,
    user_id                 INT NOT NULL,
    platform_id             INT NOT NULL,
    platform_name           VARCHAR(50) NOT NULL,
    username                VARCHAR(255) NOT NULL,
    followers_count         INT NOT NULL DEFAULT 0,
    following_count         INT NOT NULL DEFAULT 0,
    posts_count             INT NOT NULL DEFAULT 0,
    new_followers           INT NOT NULL DEFAULT 0,
    received_likes          INT NOT NULL DEFAULT 0,
    received_comments       INT NOT NULL DEFAULT 0,
    received_dms            INT NOT NULL DEFAULT 0,
    received_shares         INT NOT NULL DEFAULT 0,
    received_mentions       INT NOT NULL DEFAULT 0,
    partial                 BOOL NOT NULL DEFAULT FALSE,
    error                   TEXT,
    collected_at            TIMESTAMPTZ NOT NULL,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_patrol_reports_user ON gm_patrol_reports(user_id, report_type, created_at DESC);
CREATE INDEX idx_patrol_stats_report ON gm_patrol_account_stats(report_id);
CREATE INDEX idx_patrol_stats_account ON gm_patrol_account_stats(social_account_id, report_type, collected_at DESC);
