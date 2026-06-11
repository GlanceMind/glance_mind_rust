-- INV2 Remediation Script: re-hang gm_social_accounts to their correct group
-- after the M6 post-backfill era (group platform_id is AUTHORITATIVE and FROZEN).
--
-- Usage
-- -----
--   psql --single-transaction -f scripts/social_group_inv2_remediation.sql
--
-- IMPORTANT: This is NOT the backfill migration.
-- The backfill migration (2026-06-11-000001_social_group_platform_backfill/up.sql)
-- established correct platform_id values on all groups.  From M6 onwards,
-- gm_social_groups.platform_id is authoritative and must NEVER be changed.
-- This script is for post-M6 INV2 recheck violations only: it re-hangs
-- mismatched accounts to the correct group, leaving group platform_id untouched.
--
-- Semantics (post-M6, differs from backfill migration)
-- ----------------------------------------------------
-- For every gm_social_accounts row where account.platform_id <> group.platform_id:
--   - The account is on the wrong group.
--   - Re-hang it to group: <group_name>-<account_platform_name>
--     (same SELECT-then-INSERT approach; no unique constraint exists).
--   - The group's platform_id is NEVER modified.
--
-- Safety
-- ------
-- * Runs inside a single transaction (--single-transaction psql flag).
-- * NULL group_id accounts are ungrouped and are never touched.
-- * Every subquery is scoped by user_id (cross-user isolation guaranteed).
-- * SELECT-then-INSERT pattern (not ON CONFLICT) because no unique constraint
--   exists on (user_id, group_name).
-- * Idempotent: all paths guarded so a second run is a no-op.
--
-- Roll-back
-- ---------
-- If the final INV2 assertion fails, the --single-transaction flag ensures
-- the entire script is rolled back atomically.


-- ============================================================
-- Audit snapshot
-- ============================================================
DROP TABLE IF EXISTS _rem_pre_state;
CREATE TEMP TABLE _rem_pre_state AS
SELECT
    a.id          AS account_id,
    a.user_id,
    a.group_id    AS old_group_id,
    a.platform_id AS account_platform_id,
    g.platform_id AS group_platform_id,
    g.group_name
FROM gm_social_accounts a
JOIN gm_social_groups   g ON a.group_id = g.id
WHERE a.platform_id <> g.platform_id;


-- ============================================================
-- Re-hang mismatched accounts
-- ============================================================
-- For each violating account: find or create the correctly-named split group
-- for that account's platform, then update group_id.

DO $$
DECLARE
    rec             RECORD;
    target_group_id INTEGER;
    target_name     TEXT;
BEGIN
    FOR rec IN
        SELECT
            a.id              AS account_id,
            a.user_id,
            a.platform_id     AS account_platform,
            a.group_id        AS current_group_id,
            g.group_name      AS current_group_name
        FROM gm_social_accounts a
        JOIN gm_social_groups   g ON a.group_id = g.id
        WHERE a.platform_id <> g.platform_id
        ORDER BY a.id
    LOOP
        target_name := rec.current_group_name
                       || '-'
                       || (SELECT name FROM gm_platforms WHERE id = rec.account_platform);

        -- Look for an existing group with the correct platform and derived name
        SELECT id
        INTO   target_group_id
        FROM   gm_social_groups
        WHERE  user_id     = rec.user_id
          AND  group_name  = target_name
          AND  platform_id = rec.account_platform
        LIMIT  1;

        -- Create target group only when none exists
        IF target_group_id IS NULL THEN
            INSERT INTO gm_social_groups (user_id, platform_id, group_name)
            VALUES (rec.user_id, rec.account_platform, target_name)
            RETURNING id INTO target_group_id;
        END IF;

        -- Re-hang the account (guard for idempotency)
        UPDATE gm_social_accounts
        SET    group_id = target_group_id
        WHERE  id       = rec.account_id
          AND  group_id <> target_group_id;

    END LOOP;
END;
$$;


-- ============================================================
-- NOTICE report (before assertion so they survive an abort)
-- ============================================================

DO $$
DECLARE
    cnt_rehung       BIGINT;
    cnt_new_groups   BIGINT;
BEGIN
    -- Accounts that were re-hung
    SELECT COUNT(*) INTO cnt_rehung
    FROM _rem_pre_state;

    -- New groups created during this run
    SELECT COUNT(*) INTO cnt_new_groups
    FROM gm_social_groups g
    WHERE NOT EXISTS (
        SELECT 1 FROM _rem_pre_state r
        WHERE  r.old_group_id = g.id
           OR  r.account_id   IS NOT NULL  -- any pre-state entry
    )
    AND g.created_at >= NOW() - INTERVAL '5 minutes';  -- rough heuristic

    RAISE NOTICE '[inv2-remediation] accounts re-hung: %; new split groups created (approx): %',
        cnt_rehung, cnt_new_groups;

    -- Post-remediation report queries (run manually, REPORT ONLY):
    --
    -- Mismatched gm_aipub_plans.platform_id vs their group:
    --   SELECT p.id, p.platform_id AS plan_plat, g.platform_id AS group_plat
    --   FROM gm_aipub_plans p JOIN gm_social_groups g ON p.group_id = g.id
    --   WHERE p.platform_id <> g.platform_id;
    --
    -- Mismatched gm_campaigns.platform_id vs their group:
    --   SELECT c.id, c.platform_id AS camp_plat, g.platform_id AS group_plat
    --   FROM gm_campaigns c JOIN gm_social_groups g ON c.social_group_id = g.id
    --   WHERE c.platform_id <> g.platform_id;
END;
$$;


-- ============================================================
-- INV2 final assertion
-- ============================================================
-- Abort the transaction if any violations remain.
-- Diagnostics are embedded in the exception message (NOTICEs after RAISE
-- EXCEPTION never execute).

DO $$
DECLARE
    violation_count  BIGINT;
    violation_sample TEXT;
BEGIN
    SELECT COUNT(*)
    INTO   violation_count
    FROM   gm_social_accounts a
    JOIN   gm_social_groups g ON a.group_id = g.id
    WHERE  a.platform_id <> g.platform_id;

    IF violation_count > 0 THEN
        SELECT string_agg(
                   format('(g=%s,a=%s,gp=%s,ap=%s)',
                          g.id, a.id, g.platform_id, a.platform_id),
                   ' '
               )
        INTO   violation_sample
        FROM (
            SELECT a.id AS aid, g.id AS gid
            FROM   gm_social_accounts a
            JOIN   gm_social_groups g ON a.group_id = g.id
            WHERE  a.platform_id <> g.platform_id
            ORDER  BY g.id, a.id
            LIMIT  20
        ) sub
        JOIN gm_social_groups   g ON g.id = sub.gid
        JOIN gm_social_accounts a ON a.id = sub.aid;

        RAISE EXCEPTION
            '[inv2-remediation] INV2 VIOLATED: % account(s) still have platform_id != group.platform_id. Sample: %',
            violation_count, violation_sample;
    END IF;
END;
$$;
