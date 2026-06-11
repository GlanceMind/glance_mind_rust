-- Migration: backfill gm_social_groups.platform_id from member accounts' real platforms
--
-- Background
-- ----------
-- Legacy group rows were created with platform_id=1 (reddit) regardless of the
-- platform of the member accounts that belong to them.  This migration corrects
-- the mismatch so that gm_social_groups.platform_id always matches
-- gm_social_accounts.platform_id for every member account.
--
-- Algorithm
-- ---------
-- For each mislabelled group (i.e. any group whose platform_id differs from at
-- least one member account's platform_id):
--
--   1. Determine the "correct" platform: majority-vote over member accounts;
--      ties broken by earliest account created_at; empty groups left untouched.
--
--   2. Homogeneous group (all members agree on one platform, differs from
--      current group.platform_id): UPDATE gm_social_groups SET platform_id=<correct>.
--
--   3. Mixed group (accounts span more than one platform):
--      a. UPDATE the group's platform_id to the majority/tie-break winner.
--      b. For each minority platform P ≠ winner:
--         - Look for an existing group owned by the same user with
--           name = '<original_name>-<platform.name>' and platform_id = P.
--         - If found: re-hang the minority accounts to that group's id.
--         - If not found: INSERT a new group with that name and platform_id = P,
--           then re-hang the minority accounts to the new group's id.
--
-- Invariants enforced (INV2 assertion inside this transaction)
-- ------------------------------------------------------------
-- After all updates, the following query must return 0 rows:
--   SELECT a.id FROM gm_social_accounts a
--   JOIN gm_social_groups g ON a.group_id = g.id
--   WHERE a.platform_id <> g.platform_id
--
-- Idempotency
-- -----------
-- Running this migration twice must be a no-op on the second run:
-- all groups will already have the correct platform_id after the first pass,
-- and mixed-group splitting produces no work when accounts are already
-- correctly distributed.
--
-- Transaction boundary
-- --------------------
-- Diesel executes each migration file inside its own BEGIN/COMMIT wrapper.
-- DO NOT add explicit BEGIN or COMMIT statements here.
--
-- Deploy coordination
-- -------------------
-- Same pattern as 2026-06-03-000001_instagram_post_campaign_scoped_uniqueness:
-- this migration is data-only (pure DML).  No schema changes, no schema.rs
-- updates required.  Safe to run against production with zero downtime.
-- Roll-out order: apply migration → observe NOTICEs in migration log →
-- run scripts/social_group_inv2_remediation.sql for any post-M6 violations.
--
-- Safety
-- ------
-- * All UPDATE/INSERT paths are guarded so they match 0 rows on a clean DB
--   (idempotent).
-- * NULL group_id accounts are ungrouped; they are never touched (the JOIN
--   in every subquery requires group_id IS NOT NULL implicitly via JOIN ON).
-- * Every subquery is scoped by user_id so no cross-user contamination occurs.
-- * The F5 "name collision" path uses SELECT-then-INSERT (not ON CONFLICT)
--   because there is no unique constraint on (user_id, group_name).
-- * Tie-break: most accounts → earliest created_at among tied platforms →
--   MIN(account id) as final tie-breaker — fully deterministic.


-- ============================================================
-- Step 0: TEMP audit table of pre-migration state
-- ============================================================
-- Captures (group_id, old platform_id, per-platform member counts) for the
-- NOTICE report that follows.  Transaction-scoped; dropped automatically on
-- ROLLBACK or COMMIT.

DROP TABLE IF EXISTS pg_temp._mig_pre_state;
CREATE TEMP TABLE pg_temp._mig_pre_state AS
SELECT
    g.id          AS group_id,
    g.user_id,
    g.platform_id AS old_platform_id,
    g.group_name,
    (SELECT COUNT(*) FROM gm_social_accounts a2
     WHERE a2.group_id = g.id) AS member_count,
    (SELECT COUNT(DISTINCT a2.platform_id) FROM gm_social_accounts a2
     WHERE a2.group_id = g.id) AS distinct_platforms
FROM gm_social_groups g;


-- ============================================================
-- Step 1: Same-platform mislabelled groups (homogeneous)
-- ============================================================
-- Groups where ALL member accounts share exactly one platform and that
-- platform differs from group.platform_id.  Simply relabel the group.

UPDATE gm_social_groups g
SET    platform_id = sub.acct_platform
FROM (
    SELECT
        a.group_id,
        MAX(a.platform_id) AS acct_platform  -- MAX = MIN because all equal (COUNT DISTINCT = 1)
    FROM gm_social_accounts a
    WHERE a.group_id IS NOT NULL
    GROUP BY a.group_id
    HAVING COUNT(DISTINCT a.platform_id) = 1
) sub
WHERE g.id            = sub.group_id
  AND g.platform_id  <> sub.acct_platform;


-- ============================================================
-- Step 2: Mixed-platform groups — split & relabel
-- ============================================================
-- Uses a PL/pgSQL DO block to iterate over each mixed group and perform the
-- SELECT-then-INSERT + UPDATE sequence for each minority platform.

DO $$
DECLARE
    rec             RECORD;
    minority_rec    RECORD;
    winner_platform INTEGER;
    target_group_id INTEGER;
    target_name     TEXT;
BEGIN

    -- Iterate over every group that still has accounts on >= 2 distinct platforms
    -- after Step 1.  ORDER BY makes iteration deterministic.
    FOR rec IN
        SELECT
            g.id          AS group_id,
            g.user_id,
            g.group_name,
            g.platform_id AS current_platform
        FROM gm_social_groups g
        WHERE EXISTS (
            SELECT 1
            FROM   gm_social_accounts a
            WHERE  a.group_id = g.id
            GROUP BY a.group_id
            HAVING COUNT(DISTINCT a.platform_id) > 1
        )
        ORDER BY g.id
    LOOP

        -- ----------------------------------------------------------
        -- 2a. Determine winner platform (majority; tie-break logic)
        --     Primary:   most accounts on that platform
        --     Secondary: earliest MIN(created_at) among tied platforms
        --     Tertiary:  MIN(account id) as final tie-breaker
        -- ----------------------------------------------------------
        SELECT ranked.platform_id
        INTO   winner_platform
        FROM (
            SELECT
                a.platform_id,
                COUNT(*)          AS cnt,
                MIN(a.created_at) AS earliest,
                MIN(a.id)         AS min_id
            FROM gm_social_accounts a
            WHERE a.group_id = rec.group_id
            GROUP BY a.platform_id
            ORDER BY cnt DESC, earliest ASC, min_id ASC
            LIMIT 1
        ) ranked;

        -- ----------------------------------------------------------
        -- 2b. Relabel the original group to the winner platform
        --     Guard ensures idempotency on second run
        -- ----------------------------------------------------------
        UPDATE gm_social_groups
        SET    platform_id = winner_platform
        WHERE  id          = rec.group_id
          AND  platform_id <> winner_platform;

        -- ----------------------------------------------------------
        -- 2c. For each minority platform: find/create split group,
        --     then re-hang accounts
        -- ----------------------------------------------------------
        FOR minority_rec IN
            SELECT DISTINCT a.platform_id
            FROM gm_social_accounts a
            WHERE a.group_id    = rec.group_id
              AND a.platform_id <> winner_platform
        LOOP

            target_name := rec.group_name
                           || '-'
                           || (SELECT name FROM gm_platforms WHERE id = minority_rec.platform_id);

            -- SELECT existing target group first (plain SELECT; no unique constraint)
            SELECT id
            INTO   target_group_id
            FROM   gm_social_groups
            WHERE  user_id     = rec.user_id
              AND  group_name  = target_name
              AND  platform_id = minority_rec.platform_id
            LIMIT  1;

            -- INSERT only when no matching group exists
            IF target_group_id IS NULL THEN
                INSERT INTO gm_social_groups (user_id, platform_id, group_name)
                VALUES (rec.user_id, minority_rec.platform_id, target_name)
                RETURNING id INTO target_group_id;
            END IF;

            -- Re-hang minority accounts; guard prevents double-move on re-run
            UPDATE gm_social_accounts
            SET    group_id = target_group_id
            WHERE  group_id     = rec.group_id
              AND  platform_id  = minority_rec.platform_id
              AND  group_id    <> target_group_id;

        END LOOP;

    END LOOP;

END;
$$;


-- ============================================================
-- Step 3: NOTICE report
-- (Placed BEFORE the assertion so NOTICEs survive a potential abort)
-- ============================================================

DO $$
DECLARE
    cnt_relabeled    BIGINT;
    cnt_split_groups BIGINT;
    cnt_rehung       BIGINT;
    cnt_empty        BIGINT;
BEGIN
    -- Groups whose platform_id changed (homogeneous relabel or winner relabel)
    SELECT COUNT(*)
    INTO   cnt_relabeled
    FROM   pg_temp._mig_pre_state pre
    JOIN   gm_social_groups g ON g.id = pre.group_id
    WHERE  g.platform_id <> pre.old_platform_id
      AND  pre.member_count > 0;

    -- New split groups created during this migration (absent from pre-state snapshot)
    SELECT COUNT(*)
    INTO   cnt_split_groups
    FROM   gm_social_groups g
    WHERE  NOT EXISTS (
        SELECT 1 FROM pg_temp._mig_pre_state pre WHERE pre.group_id = g.id
    );

    -- Accounts now in newly-created split groups (re-hung accounts)
    SELECT COUNT(*)
    INTO   cnt_rehung
    FROM   gm_social_accounts a
    WHERE  a.group_id IN (
        SELECT g2.id FROM gm_social_groups g2
        WHERE NOT EXISTS (
            SELECT 1 FROM pg_temp._mig_pre_state pre WHERE pre.group_id = g2.id
        )
    );

    -- Empty groups (zero members at migration start; left untouched)
    SELECT COUNT(*)
    INTO   cnt_empty
    FROM   pg_temp._mig_pre_state
    WHERE  member_count = 0;

    RAISE NOTICE '[backfill] groups relabeled: %; split groups created: %; accounts re-hung: %; empty groups skipped: %',
        cnt_relabeled, cnt_split_groups, cnt_rehung, cnt_empty;

    -- Post-deploy export queries (run manually after migration, REPORT ONLY — do not modify plans/campaigns):
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
-- Step 4: INV2 final assertion
-- ============================================================
-- Any remaining violation is a bug in this migration.  Abort the transaction
-- with diagnostics embedded in the exception message.
-- NOTE: NOTICEs after a RAISE EXCEPTION never execute — diagnostics must be
-- inside the RAISE text itself.

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
            '[backfill] INV2 VIOLATED: % account(s) have platform_id != group.platform_id. Sample: %',
            violation_count, violation_sample;
    END IF;
END;
$$;
