"""
IT7: Backfill migration test for gm_social_groups.platform_id
=============================================================
Self-contained DB-only test (psycopg2, no API calls).

Every test runs inside a single transaction with ROLLBACK in teardown,
so the shared test DB is never permanently modified.

Fixtures:
  F1 same-platform mislabel  — homogeneous group with wrong platform_id
  F2 mixed group (majority)  — multi-platform accounts, clear majority
  F3 tie                     — equal split, tie-break by earliest created_at
  F4 empty group             — no member accounts, must be untouched
  F5 split-name collision    — minority split target name pre-exists
  F6 cross-user isolation    — user-B groups must be unaffected

RED contract: assertions FAIL until up.sql contains executable SQL.
"""

import uuid
from datetime import datetime, timezone, timedelta
from pathlib import Path

import psycopg2
import pytest

# ---------------------------------------------------------------------------
# Config
# ---------------------------------------------------------------------------

import os

DATABASE_URL = os.getenv(
    "DATABASE_URL",
    "postgres://glancemind:testpassword@localhost:5434/glancemind_test",
)

MIGRATION_UP = (
    Path(__file__).resolve().parents[2]
    / "db"
    / "migrations"
    / "2026-06-11-000001_social_group_platform_backfill"
    / "up.sql"
)

# ---------------------------------------------------------------------------
# Platform constants
# ---------------------------------------------------------------------------
# IDs verified against gm_platforms seed data in the test DB.
# Names are fetched dynamically via _load_platforms() to avoid string coupling.

PLATFORM_REDDIT = 1
PLATFORM_FACEBOOK = 3
PLATFORM_TWITTER = 5


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _uid() -> str:
    """8-char unique suffix for fixture names."""
    return uuid.uuid4().hex[:8]


def _load_platforms(cur) -> dict[int, str]:
    """Return {platform_id: name} from gm_platforms."""
    cur.execute("SELECT id, name FROM gm_platforms")
    return {row[0]: row[1] for row in cur.fetchall()}


def _execute_migration(cur, sql: str) -> None:
    """
    Execute up.sql content inside the current transaction.

    If the file contains no executable statements (empty shell) the migration
    has not been implemented yet — fail loudly with pytest.fail so the cause
    is obvious rather than surfacing as a confusing assertion error downstream.
    """
    # Strip SQL comments (-- line comments and /* block */ comments) and whitespace
    import re
    stripped = re.sub(r'--[^\n]*', '', sql)
    stripped = re.sub(r'/\*.*?\*/', '', stripped, flags=re.DOTALL)
    stripped = stripped.strip()
    # Remove statement-only whitespace/semicolons
    statements = [s.strip() for s in stripped.split(';') if s.strip()]
    if not statements:
        pytest.fail("up.sql contains no executable statements — migration not yet implemented")
    cur.execute(sql)


def _insert_user(cur, suffix: str) -> int:
    """Insert a throwaway user and return its id."""
    cur.execute(
        """
        INSERT INTO gm_users (
            email, password_hash, full_name, role, is_active, status, permissions
        ) VALUES (
            %s, 'x', %s, 'user', true, 'ACTIVE', 14
        ) RETURNING id
        """,
        (f"it7_{suffix}@test.invalid", f"IT7 User {suffix}"),
    )
    return cur.fetchone()[0]


def _insert_group(cur, user_id: int, platform_id: int, name: str) -> int:
    """Insert a social group and return its id."""
    cur.execute(
        """
        INSERT INTO gm_social_groups (user_id, platform_id, group_name)
        VALUES (%s, %s, %s) RETURNING id
        """,
        (user_id, platform_id, name),
    )
    return cur.fetchone()[0]


def _insert_account(
    cur,
    user_id: int,
    platform_id: int,
    group_id: int | None,
    suffix: str,
    created_at: datetime | None = None,
) -> int:
    """Insert a social account and return its id."""
    if created_at is None:
        cur.execute(
            """
            INSERT INTO gm_social_accounts (
                user_id, platform_id, group_id, username, status, cookie
            ) VALUES (%s, %s, %s, %s, 'ACTIVE', '') RETURNING id
            """,
            (user_id, platform_id, group_id, f"acct_{suffix}"),
        )
    else:
        cur.execute(
            """
            INSERT INTO gm_social_accounts (
                user_id, platform_id, group_id, username, status, cookie, created_at
            ) VALUES (%s, %s, %s, %s, 'ACTIVE', '', %s) RETURNING id
            """,
            (user_id, platform_id, group_id, f"acct_{suffix}", created_at),
        )
    return cur.fetchone()[0]


def _violations_for_users(cur, user_ids: list[int]) -> int:
    """
    Count gm_social_accounts rows whose platform_id != their group's platform_id,
    scoped to the given user_ids to avoid counting unrelated dirty data.
    """
    cur.execute(
        """
        SELECT COUNT(*)
        FROM gm_social_accounts a
        JOIN gm_social_groups g ON a.group_id = g.id
        WHERE a.platform_id <> g.platform_id
          AND a.user_id = ANY(%s)
        """,
        (user_ids,),
    )
    return cur.fetchone()[0]


# ---------------------------------------------------------------------------
# Fixture data builder
# ---------------------------------------------------------------------------

def _build_fixtures(cur, platforms: dict[int, str]) -> dict:
    """
    Insert all six fixture scenarios inside the current transaction.
    Returns a dict with all ids and expected-state info needed by assertions.
    """
    s = _uid()  # shared suffix for this test run
    reddit_name = platforms[PLATFORM_REDDIT]
    facebook_name = platforms[PLATFORM_FACEBOOK]
    twitter_name = platforms[PLATFORM_TWITTER]

    # --- User A (owns F1–F5) ---
    user_a = _insert_user(cur, f"A_{s}")

    # F1: same-platform mislabel
    # group platform=1(reddit), 2 accounts platform=3(facebook) → expect group→3
    f1_group = _insert_group(cur, user_a, PLATFORM_REDDIT, f"f1_group_{s}")
    _insert_account(cur, user_a, PLATFORM_FACEBOOK, f1_group, f"f1_a_{s}")
    _insert_account(cur, user_a, PLATFORM_FACEBOOK, f1_group, f"f1_b_{s}")

    # F2: mixed group (majority=facebook)
    # group platform=1, members: 3 facebook + 1 reddit → group→3, new group suffix reddit for reddit acct
    f2_group_name = f"f2_group_{s}"
    f2_group = _insert_group(cur, user_a, PLATFORM_REDDIT, f2_group_name)
    f2_acct_fb_1 = _insert_account(cur, user_a, PLATFORM_FACEBOOK, f2_group, f"f2_fb1_{s}")
    f2_acct_fb_2 = _insert_account(cur, user_a, PLATFORM_FACEBOOK, f2_group, f"f2_fb2_{s}")
    f2_acct_fb_3 = _insert_account(cur, user_a, PLATFORM_FACEBOOK, f2_group, f"f2_fb3_{s}")
    f2_acct_reddit = _insert_account(cur, user_a, PLATFORM_REDDIT, f2_group, f"f2_rd_{s}")
    f2_expected_split_name = f"{f2_group_name}-{reddit_name}"

    # F3: tie  (1 facebook account, earlier; 1 twitter account, later)
    # tie-break: earliest created_at wins → expect group→3 (facebook)
    t_early = datetime(2025, 1, 1, tzinfo=timezone.utc)
    t_late = datetime(2025, 6, 1, tzinfo=timezone.utc)
    f3_group = _insert_group(cur, user_a, PLATFORM_REDDIT, f"f3_group_{s}")
    _insert_account(cur, user_a, PLATFORM_FACEBOOK, f3_group, f"f3_fb_{s}", created_at=t_early)
    _insert_account(cur, user_a, PLATFORM_TWITTER, f3_group, f"f3_tw_{s}", created_at=t_late)

    # F4: empty group — no members → must remain platform=1
    f4_group = _insert_group(cur, user_a, PLATFORM_REDDIT, f"f4_group_{s}")

    # F5: split-name collision
    # mixed group: majority facebook + 1 reddit account.
    # Pre-existing group with name=<orig>-<reddit_name> and platform=1 already exists.
    # The reddit account must be re-hung to the PRE-EXISTING group (no new insert).
    f5_group_name = f"f5_group_{s}"
    f5_group = _insert_group(cur, user_a, PLATFORM_REDDIT, f5_group_name)
    _insert_account(cur, user_a, PLATFORM_FACEBOOK, f5_group, f"f5_fb1_{s}")
    _insert_account(cur, user_a, PLATFORM_FACEBOOK, f5_group, f"f5_fb2_{s}")
    f5_acct_reddit = _insert_account(cur, user_a, PLATFORM_REDDIT, f5_group, f"f5_rd_{s}")
    f5_preexisting_name = f"{f5_group_name}-{reddit_name}"
    f5_preexisting_group = _insert_group(cur, user_a, PLATFORM_REDDIT, f5_preexisting_name)

    # --- User B (F6: cross-user isolation) ---
    user_b = _insert_user(cur, f"B_{s}")
    # User B has a group with the same naming pattern; must be untouched
    f6_group_name = f"f6_group_{s}"
    f6_group = _insert_group(cur, user_b, PLATFORM_REDDIT, f6_group_name)
    _insert_account(cur, user_b, PLATFORM_FACEBOOK, f6_group, f"f6_fb_{s}")

    return {
        "suffix": s,
        "user_a": user_a,
        "user_b": user_b,
        "all_users": [user_a, user_b],
        "platforms": platforms,
        "reddit_name": reddit_name,
        "facebook_name": facebook_name,
        "twitter_name": twitter_name,
        # F1
        "f1_group": f1_group,
        # F2
        "f2_group": f2_group,
        "f2_group_name": f2_group_name,
        "f2_acct_reddit": f2_acct_reddit,
        "f2_expected_split_name": f2_expected_split_name,
        # F3
        "f3_group": f3_group,
        # F4
        "f4_group": f4_group,
        # F5
        "f5_group": f5_group,
        "f5_acct_reddit": f5_acct_reddit,
        "f5_preexisting_group": f5_preexisting_group,
        "f5_preexisting_name": f5_preexisting_name,
        # F6
        "f6_group": f6_group,
    }


# ---------------------------------------------------------------------------
# Main IT7 test
# ---------------------------------------------------------------------------

def test_it7_backfill_migration():
    """
    IT7: platform backfill migration produces zero violations after up.sql.

    RED phase: fails at the post-migration assertions until up.sql has SQL.
    GREEN phase: all assertions pass after implementation fills up.sql.
    """
    conn = psycopg2.connect(DATABASE_URL)
    conn.autocommit = False
    cur = conn.cursor()
    try:
        platforms = _load_platforms(cur)

        # --- Insert fixtures ---
        fx = _build_fixtures(cur, platforms)
        user_ids = fx["all_users"]

        # --- Pre-migration sanity: violations > 0 ---
        pre_violations = _violations_for_users(cur, user_ids)
        assert pre_violations > 0, (
            f"Expected fixture violations > 0 before migration, got {pre_violations}. "
            "Fixtures may not be inserting mislabelled groups correctly."
        )

        # --- Execute migration ---
        sql = MIGRATION_UP.read_text()
        _execute_migration(cur, sql)

        # --- Post-migration: zero violations (scoped to fixture users) ---
        post_violations = _violations_for_users(cur, user_ids)
        assert post_violations == 0, (
            f"After migration, expected 0 violations but found {post_violations}. "
            "up.sql did not fix all platform mismatches."
        )

        # --- F1: homogeneous mislabel → platform updated to 3 ---
        cur.execute(
            "SELECT platform_id FROM gm_social_groups WHERE id = %s",
            (fx["f1_group"],),
        )
        f1_platform = cur.fetchone()[0]
        assert f1_platform == PLATFORM_FACEBOOK, (
            f"F1: expected group platform_id={PLATFORM_FACEBOOK}, got {f1_platform}"
        )

        # --- F2: mixed majority facebook ---
        # F2a: original group platform flipped to facebook
        cur.execute(
            "SELECT platform_id FROM gm_social_groups WHERE id = %s",
            (fx["f2_group"],),
        )
        f2_platform = cur.fetchone()[0]
        assert f2_platform == PLATFORM_FACEBOOK, (
            f"F2a: expected original group platform_id={PLATFORM_FACEBOOK}, got {f2_platform}"
        )

        # F2b: a new (or existing) group exists with the split name and platform=reddit
        cur.execute(
            """
            SELECT id, platform_id FROM gm_social_groups
            WHERE user_id = %s AND group_name = %s
            """,
            (fx["user_a"], fx["f2_expected_split_name"]),
        )
        f2_split_rows = cur.fetchall()
        assert len(f2_split_rows) == 1, (
            f"F2b: expected exactly 1 group named '{fx['f2_expected_split_name']}' "
            f"for user_a, found {len(f2_split_rows)}"
        )
        f2_split_group_id, f2_split_platform = f2_split_rows[0]
        assert f2_split_platform == PLATFORM_REDDIT, (
            f"F2b: split group should have platform_id={PLATFORM_REDDIT}, got {f2_split_platform}"
        )

        # F2c: the reddit account re-hung to the split group
        cur.execute(
            "SELECT group_id FROM gm_social_accounts WHERE id = %s",
            (fx["f2_acct_reddit"],),
        )
        f2_reddit_group_id = cur.fetchone()[0]
        assert f2_reddit_group_id == f2_split_group_id, (
            f"F2c: reddit account should be re-hung to split group {f2_split_group_id}, "
            f"got group_id={f2_reddit_group_id}"
        )

        # --- F3: tie → earliest account platform wins (facebook=3) ---
        cur.execute(
            "SELECT platform_id FROM gm_social_groups WHERE id = %s",
            (fx["f3_group"],),
        )
        f3_platform = cur.fetchone()[0]
        assert f3_platform == PLATFORM_FACEBOOK, (
            f"F3: tie-break by earliest created_at expected platform={PLATFORM_FACEBOOK}, got {f3_platform}"
        )

        # --- F4: empty group → untouched (still platform=1) ---
        cur.execute(
            "SELECT platform_id FROM gm_social_groups WHERE id = %s",
            (fx["f4_group"],),
        )
        f4_platform = cur.fetchone()[0]
        assert f4_platform == PLATFORM_REDDIT, (
            f"F4: empty group must remain platform_id={PLATFORM_REDDIT}, got {f4_platform}"
        )

        # --- F5: split-name collision → re-hang to pre-existing group ---
        # F5a: the reddit account re-hung to the pre-existing group
        cur.execute(
            "SELECT group_id FROM gm_social_accounts WHERE id = %s",
            (fx["f5_acct_reddit"],),
        )
        f5_reddit_group_id = cur.fetchone()[0]
        assert f5_reddit_group_id == fx["f5_preexisting_group"], (
            f"F5a: reddit account should be re-hung to pre-existing group "
            f"{fx['f5_preexisting_group']}, got group_id={f5_reddit_group_id}"
        )

        # F5b: exactly one group with the colliding name for user_a
        cur.execute(
            """
            SELECT COUNT(*) FROM gm_social_groups
            WHERE user_id = %s AND group_name = %s
            """,
            (fx["user_a"], fx["f5_preexisting_name"]),
        )
        f5_count = cur.fetchone()[0]
        assert f5_count == 1, (
            f"F5b: expected exactly 1 group named '{fx['f5_preexisting_name']}' "
            f"for user_a, found {f5_count} (duplicate insert?)"
        )

        # --- F6: user-B homogeneous mislabel — migration is global, not user-A-scoped ---
        # F6 group had platform=reddit but 1 facebook account (homogeneous mislabel).
        # The migration must fix user-B's group too (F1-style relabel).
        cur.execute(
            "SELECT platform_id FROM gm_social_groups WHERE id = %s",
            (fx["f6_group"],),
        )
        f6_platform = cur.fetchone()[0]
        assert f6_platform == PLATFORM_FACEBOOK, (
            f"F6: user-B's homogeneous-mislabel group should be relabeled to "
            f"platform_id={PLATFORM_FACEBOOK}, got {f6_platform}"
        )

        # Zero violations for user_b (belt-and-suspenders)
        cur.execute(
            """
            SELECT COUNT(*)
            FROM gm_social_accounts a
            JOIN gm_social_groups g ON a.group_id = g.id
            WHERE a.platform_id <> g.platform_id
              AND a.user_id = %s
            """,
            (fx["user_b"],),
        )
        f6_violations = cur.fetchone()[0]
        assert f6_violations == 0, (
            f"F6: user-B should have 0 violations after migration, got {f6_violations}"
        )

    finally:
        conn.rollback()
        cur.close()
        conn.close()


# ---------------------------------------------------------------------------
# Idempotency test
# ---------------------------------------------------------------------------

def test_it7_backfill_migration_idempotent():
    """
    IT7 idempotency: running up.sql twice must leave the DB in the same state.

    RED phase: also fails until up.sql has SQL (first run already fails).
    """
    conn = psycopg2.connect(DATABASE_URL)
    conn.autocommit = False
    cur = conn.cursor()
    try:
        platforms = _load_platforms(cur)
        s = _uid()
        reddit_name = platforms[PLATFORM_REDDIT]

        # Insert a minimal mixed fixture (F2-style) for the idempotency check
        user = _insert_user(cur, f"idem_{s}")
        grp_name = f"idem_group_{s}"
        grp = _insert_group(cur, user, PLATFORM_REDDIT, grp_name)
        _insert_account(cur, user, PLATFORM_FACEBOOK, grp, f"idem_fb1_{s}")
        _insert_account(cur, user, PLATFORM_FACEBOOK, grp, f"idem_fb2_{s}")
        _insert_account(cur, user, PLATFORM_REDDIT, grp, f"idem_rd_{s}")

        sql = MIGRATION_UP.read_text()

        # First run
        _execute_migration(cur, sql)

        violations_after_first = _violations_for_users(cur, [user])
        assert violations_after_first == 0, (
            f"Idempotency: after first run expected 0 violations, got {violations_after_first}"
        )

        # Snapshot state after run 1: {group_id: platform_id} and {account_id: group_id}
        # for all groups/accounts belonging to this fixture user.
        cur.execute(
            "SELECT id, platform_id FROM gm_social_groups WHERE user_id = %s",
            (user,),
        )
        groups_snap1: dict[int, int] = {row[0]: row[1] for row in cur.fetchall()}

        cur.execute(
            """
            SELECT a.id, a.group_id
            FROM gm_social_accounts a
            JOIN gm_social_groups g ON a.group_id = g.id
            WHERE g.user_id = %s
            """,
            (user,),
        )
        accounts_snap1: dict[int, int] = {row[0]: row[1] for row in cur.fetchall()}

        # Second run — must be a true no-op
        _execute_migration(cur, sql)

        # Re-snapshot after run 2 and assert dict-equality (true no-op proof)
        cur.execute(
            "SELECT id, platform_id FROM gm_social_groups WHERE user_id = %s",
            (user,),
        )
        groups_snap2: dict[int, int] = {row[0]: row[1] for row in cur.fetchall()}

        cur.execute(
            """
            SELECT a.id, a.group_id
            FROM gm_social_accounts a
            JOIN gm_social_groups g ON a.group_id = g.id
            WHERE g.user_id = %s
            """,
            (user,),
        )
        accounts_snap2: dict[int, int] = {row[0]: row[1] for row in cur.fetchall()}

        assert groups_snap2 == groups_snap1, (
            f"Idempotency: group platform_ids changed on second run.\n"
            f"  run-1: {groups_snap1}\n"
            f"  run-2: {groups_snap2}"
        )
        assert accounts_snap2 == accounts_snap1, (
            f"Idempotency: account group_ids changed on second run.\n"
            f"  run-1: {accounts_snap1}\n"
            f"  run-2: {accounts_snap2}"
        )

        # Still zero violations (belt-and-suspenders)
        violations_after_second = _violations_for_users(cur, [user])
        assert violations_after_second == 0, (
            f"Idempotency: after second run expected 0 violations, got {violations_after_second}"
        )

    finally:
        conn.rollback()
        cur.close()
        conn.close()
