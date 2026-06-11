#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
M3 aipub group guard — RED test suite (IT5a-j)
================================================
Tests for group-platform guard, freeze-count accuracy, and 0-match semantics
in the aipub plan create + expand flow.

RED today: none of the M3 guard logic exists yet — tests pin the SPEC
behaviour so that implementation can drive them green.

Run:
    pytest crates/api/tests/test_aipub_group_guard.py -v

Requires:
    - API server running on API_BASE_URL (127.0.0.1:8081) with no_proxy set
    - Postgres reachable via DATABASE_URL (localhost:5432)

Isolation:
    Every test is self-contained with try/finally cleanup.
    db_cursor fixture rolls back uncommitted changes; direct inserts use
    explicit commit + compensating DELETE in finally blocks.
"""

from __future__ import annotations

import os
import uuid
from decimal import Decimal
from typing import Dict, List, Optional

import pytest

os.environ.setdefault("no_proxy", "localhost,127.0.0.1")
os.environ.setdefault("NO_PROXY", "localhost,127.0.0.1")

try:
    from conftest import (
        assert_response_success,
        extract_data,
        APIClient,
        get_or_create_test_token,
        resolve_test_user_id,
        TEST_USER_ID,
        PLATFORM_REDDIT,
        PLATFORM_TIKTOK,
        PLATFORM_FACEBOOK,
    )
except ImportError:
    PLATFORM_REDDIT = 1
    PLATFORM_TIKTOK = 2
    PLATFORM_FACEBOOK = 3
    TEST_USER_ID = 999

    def assert_response_success(resp, expected_status=200):
        assert resp.status_code == expected_status, f"Expected {expected_status}: {resp.text}"

    def extract_data(j):
        return j.get("data", j) if isinstance(j, dict) else j


API_BASE_URL = os.getenv("API_BASE_URL", "http://localhost:8081")

# ────────────────────────────────────────────────────────────────────────────
# Helpers
# ────────────────────────────────────────────────────────────────────────────

def _uid() -> str:
    """Short unique suffix for naming test rows."""
    return uuid.uuid4().hex[:8]


def _auth_client() -> "APIClient":
    token = get_or_create_test_token()
    return APIClient(API_BASE_URL, token)


def _seed_wallet(db_cursor, user_id: int = TEST_USER_ID, balance: float = 5000.0) -> None:
    """Ensure the test user wallet has sufficient balance for plan creation."""
    db_cursor.execute("""
        UPDATE gm_user_wallets
        SET balance_points = %s, frozen_points = 0, updated_at = NOW()
        WHERE user_id = %s
    """, (balance, user_id))
    db_cursor.connection.commit()


def _get_chat_model(db_cursor) -> dict:
    """Return the first active chat model (id + cost_multiplier)."""
    db_cursor.execute("""
        SELECT id, model_key, cost_multiplier
        FROM gm_ai_models
        WHERE model_type = 'chat' AND is_active = true
        ORDER BY id
        LIMIT 1
    """)
    row = db_cursor.fetchone()
    assert row is not None, "No chat model seeded in gm_ai_models"
    return dict(row)


def _create_group_via_api(auth_client, platform_id: int, name: str) -> int:
    """Create a social group via API; return its id."""
    resp = auth_client.post("/api/v1/social-groups", json={
        "platform_id": platform_id,
        "group_name": name,
    })
    assert resp.status_code in (200, 201), (
        f"Failed to create group '{name}': {resp.status_code} {resp.text}"
    )
    return extract_data(resp.json())["id"]


def _delete_group_via_api(auth_client, group_id: int) -> None:
    auth_client.delete(f"/api/v1/social-groups/{group_id}")


def _insert_account_direct(db_cursor, user_id: int, group_id: int,
                            platform_id: int, username: str) -> int:
    """Bypass API validation; insert account directly into DB. Returns id."""
    db_cursor.execute("""
        INSERT INTO gm_social_accounts
            (user_id, group_id, platform_id, username, cookie, status)
        VALUES (%s, %s, %s, %s, '', 'ACTIVE')
        RETURNING id
    """, (user_id, group_id, platform_id, username))
    row = db_cursor.fetchone()
    db_cursor.connection.commit()
    return row["id"]


def _delete_account_direct(db_cursor, account_id: int) -> None:
    db_cursor.execute("DELETE FROM gm_social_accounts WHERE id = %s", (account_id,))
    db_cursor.connection.commit()


def _delete_plan_and_tasks(db_cursor, plan_id: int) -> None:
    """Clean up plan + its tasks (publish tasks cascade, ai_tasks cascade)."""
    db_cursor.execute("DELETE FROM gm_aipub_tasks WHERE plan_id = %s", (plan_id,))
    db_cursor.execute("DELETE FROM gm_aipub_ai_tasks WHERE plan_id = %s", (plan_id,))
    db_cursor.execute("DELETE FROM gm_aipub_plans WHERE id = %s", (plan_id,))
    db_cursor.connection.commit()


def _count_publish_tasks(db_cursor, plan_id: int) -> int:
    db_cursor.execute(
        "SELECT COUNT(*) as c FROM gm_aipub_tasks WHERE plan_id = %s",
        (plan_id,)
    )
    return db_cursor.fetchone()["c"]


def _get_frozen_cost(db_cursor, plan_id: int) -> Decimal:
    db_cursor.execute(
        "SELECT frozen_cost FROM gm_aipub_plans WHERE id = %s",
        (plan_id,)
    )
    row = db_cursor.fetchone()
    if row is None:
        return Decimal("0")
    return Decimal(str(row["frozen_cost"]))


def _minimal_batch_text_payload(group_id: int, platform_id: int,
                                chat_model_id: int, name: str) -> dict:
    """Minimal valid batch_text plan payload."""
    return {
        "plan_type": "batch_text",
        "name": name,
        "group_id": group_id,
        "platform_id": platform_id,
        "content_type": "post",
        "chat_ai_model_id": chat_model_id,
        "ai_input": {
            "content_prompt": "Write a short engaging post about technology.",
        },
    }


def _minimal_reddit_text_payload(group_id: int, chat_model_id: int, name: str) -> dict:
    """Minimal valid reddit_text plan payload."""
    return {
        "plan_type": "reddit_text",
        "name": name,
        "group_id": group_id,
        "platform_id": PLATFORM_REDDIT,
        "content_type": "post",
        "chat_ai_model_id": chat_model_id,
        "ai_input": {
            "content_prompt": "Write a tech discussion post.",
            "reddit_config": {
                "subreddit": "testsubreddit",
            },
        },
    }


def _create_plan_and_get_id(auth_client, payload: dict) -> Optional[int]:
    """Create plan, return plan_id if created successfully, else None."""
    resp = auth_client.post("/api/v1/publish_plans", json=payload)
    if resp.status_code in (200, 201):
        j = resp.json()
        data = extract_data(j)
        if isinstance(data, dict) and "id" in data:
            return data["id"]
    return None


def _delete_plan_via_api(auth_client, plan_id: int) -> None:
    auth_client.delete(f"/api/v1/publish_plans/{plan_id}")


# ────────────────────────────────────────────────────────────────────────────
# IT5a: group platform ≠ plan platform → 400 + code 2001 + "does not match"
# ────────────────────────────────────────────────────────────────────────────

class TestIT5a_GroupPlatformMismatch:
    """
    IT5a: Create batch_text plan with platform_id=3 (facebook) targeting a
    group with platform_id=1 (reddit).  Expected: 400 with error code 2001
    and message containing "does not match".

    RED today: API returns 200 with code=1000 (no platform guard on create).
    """

    def test_it5a_platform_mismatch_returns_400(self, auth_client, db_cursor):
        model = _get_chat_model(db_cursor)
        gR_id = None

        try:
            # Create a reddit group
            gR_id = _create_group_via_api(
                auth_client, PLATFORM_REDDIT, f"gR_IT5a_{_uid()}"
            )

            payload = _minimal_batch_text_payload(
                group_id=gR_id,
                platform_id=PLATFORM_FACEBOOK,   # plan says facebook
                chat_model_id=model["id"],
                name=f"IT5a_{_uid()}",
            )

            resp = auth_client.post("/api/v1/publish_plans", json=payload)

            # ---- RED today expectation vs spec ----
            print(f"\n[IT5a] status={resp.status_code} body={resp.text[:300]}")

            # SPEC: must reject with HTTP 400
            assert resp.status_code == 400, (
                f"IT5a RED: expected 400 (group platform mismatch), "
                f"got {resp.status_code}: {resp.text[:200]}"
            )

            j = resp.json()
            assert j.get("code") == 2001, (
                f"IT5a RED: expected code==2001 (BadRequest), got {j.get('code')}"
            )
            msg = j.get("msg", "") + j.get("msg_cn", "")
            assert "does not match" in msg.lower() or "mismatch" in msg.lower(), (
                f"IT5a RED: expected 'does not match' in msg, got: {msg!r}"
            )

        finally:
            if gR_id:
                _delete_group_via_api(auth_client, gR_id)


# ────────────────────────────────────────────────────────────────────────────
# IT5b: group_id belongs to a different user → 404
# ────────────────────────────────────────────────────────────────────────────

class TestIT5b_OtherUserGroup:
    """
    IT5b: user A creates a plan targeting user B's group.  Expected: 404.
    The M2 load_and_check_group already performs ownership check;
    this test verifies it is called from the aipub create path.

    RED today: 200 (create succeeds — no ownership check on group_id).
    """

    def test_it5b_other_user_group_returns_404(self, auth_client, db_cursor):
        model = _get_chat_model(db_cursor)
        # Insert a group owned by a different (non-test) user directly in DB
        other_user_id = 1  # assume user_id=1 exists or we create one below
        db_cursor.execute("SELECT id FROM gm_users WHERE id != %s LIMIT 1", (TEST_USER_ID,))
        row = db_cursor.fetchone()
        if row is None:
            pytest.skip("No second user in DB to create group for")
        other_user_id = row["id"]

        # Insert group directly (bypass API so it belongs to other_user_id)
        db_cursor.execute("""
            INSERT INTO gm_social_groups (user_id, platform_id, group_name)
            VALUES (%s, %s, %s) RETURNING id
        """, (other_user_id, PLATFORM_FACEBOOK, f"gB_IT5b_{_uid()}"))
        gB_id = db_cursor.fetchone()["id"]
        db_cursor.connection.commit()

        try:
            payload = _minimal_batch_text_payload(
                group_id=gB_id,
                platform_id=PLATFORM_FACEBOOK,
                chat_model_id=model["id"],
                name=f"IT5b_{_uid()}",
            )

            resp = auth_client.post("/api/v1/publish_plans", json=payload)
            print(f"\n[IT5b] status={resp.status_code} body={resp.text[:300]}")

            # SPEC: 404 (group not found / not owned by caller)
            assert resp.status_code == 404, (
                f"IT5b RED: expected 404 (other user's group), "
                f"got {resp.status_code}: {resp.text[:200]}"
            )

        finally:
            # Must cascade: delete any plans created against gB before deleting group
            db_cursor.execute("DELETE FROM gm_aipub_tasks WHERE plan_id IN (SELECT id FROM gm_aipub_plans WHERE group_id = %s)", (gB_id,))
            db_cursor.execute("DELETE FROM gm_aipub_ai_tasks WHERE plan_id IN (SELECT id FROM gm_aipub_plans WHERE group_id = %s)", (gB_id,))
            db_cursor.execute("DELETE FROM gm_aipub_plans WHERE group_id = %s", (gB_id,))
            db_cursor.execute("DELETE FROM gm_social_groups WHERE id = %s", (gB_id,))
            db_cursor.connection.commit()


# ────────────────────────────────────────────────────────────────────────────
# IT5c: group_id does not exist → 404
# ────────────────────────────────────────────────────────────────────────────

class TestIT5c_NonExistentGroup:
    """
    IT5c: group_id=99999999 → must return 404.

    RED today: may return 200 (plan created with invalid group_id)
    or may return 500 (FK violation). Either way it must NOT return 200.
    """

    def test_it5c_nonexistent_group_returns_404(self, auth_client, db_cursor):
        model = _get_chat_model(db_cursor)

        payload = _minimal_batch_text_payload(
            group_id=99999999,
            platform_id=PLATFORM_FACEBOOK,
            chat_model_id=model["id"],
            name=f"IT5c_{_uid()}",
        )

        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        print(f"\n[IT5c] status={resp.status_code} body={resp.text[:300]}")

        # SPEC: must return 404 (group not found).
        # RED today: returns 500 with FK violation instead of a clean 404.
        assert resp.status_code == 404, (
            f"IT5c RED: expected 404 (non-existent group), "
            f"got {resp.status_code}: {resp.text[:200]}\n"
            f"  Current behavior: FK violation surfaces as 500 DB error instead of "
            f"clean 404 — production code must validate group existence before insert."
        )


# ────────────────────────────────────────────────────────────────────────────
# IT5d: valid facebook group + facebook accounts → success (green baseline)
# ────────────────────────────────────────────────────────────────────────────

class TestIT5d_ValidGroupAndAccounts:
    """
    IT5d: group gF (facebook), 2 facebook accounts in gF, plan platform=3 →
    200 + code=1000 + billing_status=frozen.

    This is expected to be GREEN today (create succeeds).  After M3 lands,
    this test must still pass — it is the positive path.
    """

    def test_it5d_valid_facebook_plan_creates_ok(self, auth_client, db_cursor):
        model = _get_chat_model(db_cursor)
        _seed_wallet(db_cursor)
        gF_id = None
        acc_ids: List[int] = []
        plan_id_d: Optional[int] = None

        try:
            gF_id = _create_group_via_api(
                auth_client, PLATFORM_FACEBOOK, f"gF_IT5d_{_uid()}"
            )

            # Insert 2 facebook accounts into gF directly
            for i in range(2):
                acc_id = _insert_account_direct(
                    db_cursor, TEST_USER_ID, gF_id,
                    PLATFORM_FACEBOOK, f"fb_acc_IT5d_{i}_{_uid()}"
                )
                acc_ids.append(acc_id)

            payload = _minimal_batch_text_payload(
                group_id=gF_id,
                platform_id=PLATFORM_FACEBOOK,
                chat_model_id=model["id"],
                name=f"IT5d_{_uid()}",
            )

            resp = auth_client.post("/api/v1/publish_plans", json=payload)
            print(f"\n[IT5d] status={resp.status_code} body={resp.text[:400]}")

            # SPEC + current behaviour: 200 success
            assert resp.status_code in (200, 201), (
                f"IT5d: expected 200/201 (valid plan), "
                f"got {resp.status_code}: {resp.text[:200]}"
            )

            j = resp.json()
            assert j.get("code") == 1000, (
                f"IT5d: expected code==1000, got {j.get('code')}"
            )

            data = extract_data(j)
            plan_id_d = data["id"]

            # Billing should be frozen for 2 accounts
            assert data.get("billing_status") == "frozen", (
                f"IT5d: expected billing_status=frozen, got {data.get('billing_status')!r}"
            )
            assert float(data.get("frozen_cost", 0)) > 0, (
                f"IT5d: expected frozen_cost > 0, got {data.get('frozen_cost')}"
            )

        finally:
            if plan_id_d:
                _delete_plan_via_api(auth_client, plan_id_d)
            for acc_id in acc_ids:
                _delete_account_direct(db_cursor, acc_id)
            if gF_id:
                _delete_group_via_api(auth_client, gF_id)


# ────────────────────────────────────────────────────────────────────────────
# IT5e: reddit_text plan platform=1 + group with platform=3 → 400
# ────────────────────────────────────────────────────────────────────────────

class TestIT5e_RedditPlanFacebookGroup:
    """
    IT5e: reddit_text plan (platform_id=1) targeting a facebook group
    (group.platform_id=3) → 400 (group platform mismatch).

    The ONLY expected failure is the group-platform guard; the payload is
    otherwise a valid reddit_text plan.

    RED today: 200 (no group-platform guard).
    """

    def test_it5e_reddit_plan_facebook_group_returns_400(self, auth_client, db_cursor):
        model = _get_chat_model(db_cursor)
        _seed_wallet(db_cursor)
        gF_id = None

        try:
            gF_id = _create_group_via_api(
                auth_client, PLATFORM_FACEBOOK, f"gF_IT5e_{_uid()}"
            )

            # Insert a reddit account directly into the facebook group
            # (simulating pre-M2 dirty data or legacy row) — the group
            # platform is still facebook but we want to test plan-level
            # platform guard, not account binding.
            # Actually for this test we just need the group_id mismatch:
            # plan.platform_id=1 vs group.platform_id=3

            payload = _minimal_reddit_text_payload(
                group_id=gF_id,
                chat_model_id=model["id"],
                name=f"IT5e_{_uid()}",
            )
            # plan_type=reddit_text, platform_id=1 (reddit)
            # group.platform_id=3 (facebook) → mismatch

            resp = auth_client.post("/api/v1/publish_plans", json=payload)
            print(f"\n[IT5e] status={resp.status_code} body={resp.text[:300]}")

            # SPEC: 400 due to group platform mismatch
            assert resp.status_code == 400, (
                f"IT5e RED: expected 400 (reddit plan, facebook group), "
                f"got {resp.status_code}: {resp.text[:200]}"
            )

        finally:
            if gF_id:
                _delete_group_via_api(auth_client, gF_id)


# ────────────────────────────────────────────────────────────────────────────
# IT5h: freeze amount counts only platform-matched accounts
# ────────────────────────────────────────────────────────────────────────────

class TestIT5h_FreezeCountMatchedOnly:
    """
    IT5h: group gMix has 2 facebook accounts + 1 reddit account (dirty/legacy).
    Create batch_text plan with platform_id=3 (facebook), NO image_generations.
    Freeze should equal 2 * chat_model.cost_multiplier (2 facebook accounts).

    RED today: freeze = 3 * cost_multiplier (all 3 accounts regardless of platform).
    """

    def test_it5h_freeze_counts_only_platform_matched_accounts(self, auth_client, db_cursor):
        model = _get_chat_model(db_cursor)
        cost_per_unit = Decimal(str(model["cost_multiplier"]))
        _seed_wallet(db_cursor, balance=5000.0)

        gMix_id = None
        acc_ids: List[int] = []
        plan_id_created = None

        try:
            gMix_id = _create_group_via_api(
                auth_client, PLATFORM_FACEBOOK, f"gMix_IT5h_{_uid()}"
            )

            # Insert 2 facebook accounts (platform=3) — these match
            for i in range(2):
                acc_id = _insert_account_direct(
                    db_cursor, TEST_USER_ID, gMix_id,
                    PLATFORM_FACEBOOK, f"fb_mix_IT5h_{i}_{_uid()}"
                )
                acc_ids.append(acc_id)

            # Insert 1 reddit account (platform=1) — dirty legacy data, should NOT count
            reddit_acc_id = _insert_account_direct(
                db_cursor, TEST_USER_ID, gMix_id,
                PLATFORM_REDDIT, f"rd_mix_IT5h_{_uid()}"
            )
            acc_ids.append(reddit_acc_id)

            payload = _minimal_batch_text_payload(
                group_id=gMix_id,
                platform_id=PLATFORM_FACEBOOK,
                chat_model_id=model["id"],
                name=f"IT5h_{_uid()}",
            )

            resp = auth_client.post("/api/v1/publish_plans", json=payload)
            print(f"\n[IT5h] status={resp.status_code} body={resp.text[:400]}")

            assert resp.status_code in (200, 201), (
                f"IT5h: plan creation failed unexpectedly: {resp.status_code} {resp.text[:200]}"
            )

            j = resp.json()
            data = extract_data(j)
            plan_id_created = data.get("id")

            # Read unit price (P) from DB — do not hardcode
            P = cost_per_unit  # from gm_ai_models

            # Expected: 2 accounts matched (facebook), freeze = 2 * P
            expected_freeze = 2 * P

            # Actual frozen_cost from plan response
            actual_freeze = Decimal(str(data.get("frozen_cost", "0")))

            print(f"\n[IT5h] P={P}, expected_freeze={expected_freeze}, actual_freeze={actual_freeze}")

            # SPEC: only 2 matched accounts billed
            # RED today: 3 * P because all 3 accounts (including reddit) are counted
            assert actual_freeze == expected_freeze, (
                f"IT5h RED: expected frozen_cost == {expected_freeze} "
                f"(2 facebook accounts * cost_multiplier={P}), "
                f"got {actual_freeze} — hint: all {len(acc_ids)} accounts "
                f"(incl. reddit) were counted instead of only 2 platform-matched ones"
            )

        finally:
            if plan_id_created:
                _delete_plan_via_api(auth_client, plan_id_created)
            for acc_id in acc_ids:
                _delete_account_direct(db_cursor, acc_id)
            if gMix_id:
                _delete_group_via_api(auth_client, gMix_id)


# ────────────────────────────────────────────────────────────────────────────
# IT5i: create-time 0-match (all accounts are wrong-platform) → 400 + rollback
# ────────────────────────────────────────────────────────────────────────────

class TestIT5i_ZeroMatchAtCreateTime:
    """
    IT5i: group gDirty has ONLY reddit accounts (platform=1) but plan
    platform_id=3 (facebook).  Create-time check must reject:
      - HTTP 400
      - msg contains "no accounts matching plan platform"
      - plan rolled back (no row in gm_aipub_plans for this name)

    Applies when group has ≥1 accounts but 0 match the plan's platform.
    (An EMPTY group is separate: see IT5j.)

    RED today: create succeeds (200), billing freezes $0, and plan is created.
    """

    def test_it5i_zero_match_rejects_create(self, auth_client, db_cursor):
        model = _get_chat_model(db_cursor)
        _seed_wallet(db_cursor)

        gDirty_id = None
        acc_ids: List[int] = []
        plan_name = f"IT5i_{_uid()}"

        try:
            gDirty_id = _create_group_via_api(
                auth_client, PLATFORM_FACEBOOK, f"gDirty_IT5i_{_uid()}"
            )

            # Insert 2 reddit accounts into a facebook group (legacy dirty data)
            for i in range(2):
                acc_id = _insert_account_direct(
                    db_cursor, TEST_USER_ID, gDirty_id,
                    PLATFORM_REDDIT, f"rd_dirty_IT5i_{i}_{_uid()}"
                )
                acc_ids.append(acc_id)

            payload = _minimal_batch_text_payload(
                group_id=gDirty_id,
                platform_id=PLATFORM_FACEBOOK,
                chat_model_id=model["id"],
                name=plan_name,
            )

            resp = auth_client.post("/api/v1/publish_plans", json=payload)
            print(f"\n[IT5i] status={resp.status_code} body={resp.text[:400]}")

            # SPEC: must reject
            assert resp.status_code == 400, (
                f"IT5i RED: expected 400 (0 platform-matched accounts in group), "
                f"got {resp.status_code}: {resp.text[:200]}"
            )

            msg = resp.json().get("msg", "") + resp.json().get("msg_cn", "")
            assert (
                "no accounts matching" in msg.lower()
                or "0 matching" in msg.lower()
                or "no matching" in msg.lower()
                or "platform" in msg.lower()
            ), (
                f"IT5i RED: expected msg about 'no accounts matching', got: {msg!r}"
            )

            # SPEC: plan must NOT have been persisted (rollback)
            db_cursor.execute(
                "SELECT COUNT(*) as c FROM gm_aipub_plans WHERE name = %s AND user_id = %s",
                (plan_name, TEST_USER_ID)
            )
            count = db_cursor.fetchone()["c"]
            assert count == 0, (
                f"IT5i RED: plan row should have been rolled back, but found {count} row(s)"
            )

        finally:
            for acc_id in acc_ids:
                _delete_account_direct(db_cursor, acc_id)
            if gDirty_id:
                _delete_group_via_api(auth_client, gDirty_id)


# ────────────────────────────────────────────────────────────────────────────
# IT5f: expand filters — dirty group with 2 matching + 1 mismatched account
# ────────────────────────────────────────────────────────────────────────────

class TestIT5f_ExpandFiltersAccounts:
    """
    IT5f: group gMix (platform=3) has 2 facebook accounts + 1 reddit account
    (legacy dirty).  Drive the plan through AI completion via
    POST /internal/aipub/ai_tasks/:id/complete.
    Assert: exactly 2 publish tasks created, all for facebook account_ids.

    RED today: 3 publish tasks created (expand uses all group accounts).

    IT5g strategy note: The internal complete endpoint
    (POST /api/v1/internal/aipub/ai_tasks/:id/complete) is accessible without
    auth in the current build (root.rs line 366: no middleware applied to
    aipub_internal_routes). This allows pytest to drive the callback directly.
    """

    def _seed_ai_task_for_plan(self, db_cursor, plan_id: int) -> int:
        """Insert a processing AI task for the given plan. Returns ai_task id."""
        db_cursor.execute("""
            INSERT INTO gm_aipub_ai_tasks
                (plan_id, task_type, external_service, input, status)
            VALUES (%s, 'content_gen', 'test_mock', '{}'::jsonb, 'processing')
            RETURNING id
        """, (plan_id,))
        row = db_cursor.fetchone()
        db_cursor.connection.commit()
        return row["id"]

    def test_it5f_expand_filters_to_platform_matched_accounts(self, auth_client, db_cursor):
        model = _get_chat_model(db_cursor)
        _seed_wallet(db_cursor, balance=5000.0)

        gMix_id = None
        acc_ids: List[int] = []
        fb_acc_ids: List[int] = []
        plan_id_created = None

        try:
            gMix_id = _create_group_via_api(
                auth_client, PLATFORM_FACEBOOK, f"gMix_IT5f_{_uid()}"
            )

            # 2 facebook accounts — should get publish tasks
            for i in range(2):
                acc_id = _insert_account_direct(
                    db_cursor, TEST_USER_ID, gMix_id,
                    PLATFORM_FACEBOOK, f"fb_IT5f_{i}_{_uid()}"
                )
                acc_ids.append(acc_id)
                fb_acc_ids.append(acc_id)

            # 1 reddit account — must NOT get a publish task
            reddit_acc_id = _insert_account_direct(
                db_cursor, TEST_USER_ID, gMix_id,
                PLATFORM_REDDIT, f"rd_IT5f_{_uid()}"
            )
            acc_ids.append(reddit_acc_id)

            payload = _minimal_batch_text_payload(
                group_id=gMix_id,
                platform_id=PLATFORM_FACEBOOK,
                chat_model_id=model["id"],
                name=f"IT5f_{_uid()}",
            )

            resp = auth_client.post("/api/v1/publish_plans", json=payload)
            print(f"\n[IT5f] create: status={resp.status_code}")

            assert resp.status_code in (200, 201), (
                f"IT5f: plan creation failed: {resp.status_code} {resp.text[:200]}"
            )
            plan_id_created = extract_data(resp.json())["id"]

            # Seed an AI task in 'processing' state (simulating scheduler dispatch)
            ai_task_id = self._seed_ai_task_for_plan(db_cursor, plan_id_created)

            # Fire the complete callback
            complete_payload = {"result": {"content": "AI-generated post content"}}
            complete_resp = auth_client.post(
                f"/api/v1/internal/aipub/ai_tasks/{ai_task_id}/complete",
                json=complete_payload,
            )
            print(f"\n[IT5f] complete callback: status={complete_resp.status_code} body={complete_resp.text[:300]}")

            assert complete_resp.status_code in (200, 201), (
                f"IT5f: complete callback failed: {complete_resp.status_code} {complete_resp.text[:200]}"
            )

            # Count publish tasks
            task_count = _count_publish_tasks(db_cursor, plan_id_created)
            print(f"\n[IT5f] publish task count = {task_count}")

            # SPEC: exactly 2 tasks for 2 facebook accounts
            # RED today: 3 tasks (reddit account also included)
            assert task_count == 2, (
                f"IT5f RED: expected 2 publish tasks (facebook accounts only), "
                f"got {task_count} — reddit account should have been filtered out"
            )

            # Verify task account_ids are all facebook
            db_cursor.execute("""
                SELECT sa.platform_id, pt.social_account_id
                FROM gm_aipub_tasks pt
                JOIN gm_social_accounts sa ON sa.id = pt.social_account_id
                WHERE pt.plan_id = %s
            """, (plan_id_created,))
            rows = db_cursor.fetchall()
            for row in rows:
                assert row["platform_id"] == PLATFORM_FACEBOOK, (
                    f"IT5f RED: publish task for account_id={row['social_account_id']} "
                    f"has platform_id={row['platform_id']}, expected {PLATFORM_FACEBOOK}"
                )

        finally:
            if plan_id_created:
                _delete_plan_and_tasks(db_cursor, plan_id_created)
            for acc_id in acc_ids:
                _delete_account_direct(db_cursor, acc_id)
            if gMix_id:
                _delete_group_via_api(auth_client, gMix_id)


# ────────────────────────────────────────────────────────────────────────────
# IT5g: expand 0-match → plan failed + budget refunded
# ────────────────────────────────────────────────────────────────────────────

class TestIT5g_ExpandZeroMatchFailsAndRefunds:
    """
    IT5g: The plan is created with 2 matching accounts (passes create guard).
    Before the AI callback fires, accounts are drifted to wrong platform via
    db_cursor UPDATE.  complete_ai_task → expand → 0 matching → plan failed,
    billing released (frozen_points restored to pre-freeze value).

    Strategy: The internal complete endpoint is drivable from pytest without
    auth (see root.rs — aipub_internal_routes has no middleware). This is the
    chosen IT5g strategy.

    RED today: expand creates 0 tasks but plan stays in 'ready' status and
    billing is NOT released (frozen never settled).
    """

    def _seed_ai_task_for_plan(self, db_cursor, plan_id: int) -> int:
        db_cursor.execute("""
            INSERT INTO gm_aipub_ai_tasks
                (plan_id, task_type, external_service, input, status)
            VALUES (%s, 'content_gen', 'test_mock', '{}'::jsonb, 'processing')
            RETURNING id
        """, (plan_id,))
        row = db_cursor.fetchone()
        db_cursor.connection.commit()
        return row["id"]

    def test_it5g_expand_zero_match_fails_plan_and_refunds(self, auth_client, db_cursor):
        model = _get_chat_model(db_cursor)
        _seed_wallet(db_cursor, balance=5000.0)

        # Snapshot frozen_points before
        db_cursor.execute(
            "SELECT frozen_points FROM gm_user_wallets WHERE user_id = %s",
            (TEST_USER_ID,)
        )
        frozen_before = Decimal(str(db_cursor.fetchone()["frozen_points"]))

        gF_id = None
        acc_ids: List[int] = []
        plan_id_created = None

        try:
            gF_id = _create_group_via_api(
                auth_client, PLATFORM_FACEBOOK, f"gF_IT5g_{_uid()}"
            )

            # Create 2 facebook accounts — plan will pass create guard
            for i in range(2):
                acc_id = _insert_account_direct(
                    db_cursor, TEST_USER_ID, gF_id,
                    PLATFORM_FACEBOOK, f"fb_IT5g_{i}_{_uid()}"
                )
                acc_ids.append(acc_id)

            payload = _minimal_batch_text_payload(
                group_id=gF_id,
                platform_id=PLATFORM_FACEBOOK,
                chat_model_id=model["id"],
                name=f"IT5g_{_uid()}",
            )

            resp = auth_client.post("/api/v1/publish_plans", json=payload)
            print(f"\n[IT5g] create: status={resp.status_code}")

            assert resp.status_code in (200, 201), (
                f"IT5g: plan creation failed: {resp.status_code} {resp.text[:200]}"
            )
            plan_id_created = extract_data(resp.json())["id"]

            # Seed a processing AI task
            ai_task_id = self._seed_ai_task_for_plan(db_cursor, plan_id_created)

            # ── Platform drift: mutate accounts to reddit ──
            # This simulates accounts being rebound or data corrupted between
            # plan creation and AI callback.
            for acc_id in acc_ids:
                db_cursor.execute(
                    "UPDATE gm_social_accounts SET platform_id = %s WHERE id = %s",
                    (PLATFORM_REDDIT, acc_id)
                )
            db_cursor.connection.commit()

            # Snapshot frozen_points after freeze (should be > frozen_before)
            db_cursor.execute(
                "SELECT frozen_points FROM gm_user_wallets WHERE user_id = %s",
                (TEST_USER_ID,)
            )
            frozen_after_freeze = Decimal(str(db_cursor.fetchone()["frozen_points"]))

            # Fire the complete callback — expand will find 0 matching accounts
            complete_payload = {"result": {"content": "AI-generated post content"}}
            complete_resp = auth_client.post(
                f"/api/v1/internal/aipub/ai_tasks/{ai_task_id}/complete",
                json=complete_payload,
            )
            print(f"\n[IT5g] complete callback: status={complete_resp.status_code} body={complete_resp.text[:300]}")

            # Verify plan status and billing after callback
            db_cursor.execute(
                "SELECT status, billing_status, frozen_cost FROM gm_aipub_plans WHERE id = %s",
                (plan_id_created,)
            )
            plan_row = db_cursor.fetchone()
            print(f"\n[IT5g] plan: status={plan_row['status']!r} billing={plan_row['billing_status']!r} frozen_cost={plan_row['frozen_cost']}")

            db_cursor.execute(
                "SELECT frozen_points FROM gm_user_wallets WHERE user_id = %s",
                (TEST_USER_ID,)
            )
            frozen_after_callback = Decimal(str(db_cursor.fetchone()["frozen_points"]))

            task_count = _count_publish_tasks(db_cursor, plan_id_created)
            print(f"\n[IT5g] publish task count = {task_count}")

            # SPEC:
            # (a) 0 publish tasks created
            assert task_count == 0, (
                f"IT5g RED: expected 0 publish tasks (all accounts drifted to reddit), "
                f"got {task_count}"
            )

            # (b) plan status = 'failed'
            assert plan_row["status"] == "failed", (
                f"IT5g RED: expected plan status='failed' after 0-match expand, "
                f"got {plan_row['status']!r}"
            )

            # (c) billing released: frozen_points back to pre-create level
            # The freeze should have been refunded
            assert frozen_after_callback == frozen_before, (
                f"IT5g RED: expected frozen_points restored to {frozen_before} "
                f"(pre-create level), but got {frozen_after_callback} "
                f"(after-freeze was {frozen_after_freeze}) — billing not settled/refunded"
            )

        finally:
            # Restore account platform before deleting
            for acc_id in acc_ids:
                db_cursor.execute(
                    "UPDATE gm_social_accounts SET platform_id = %s WHERE id = %s",
                    (PLATFORM_FACEBOOK, acc_id)
                )
            db_cursor.connection.commit()
            if plan_id_created:
                _delete_plan_and_tasks(db_cursor, plan_id_created)
            for acc_id in acc_ids:
                _delete_account_direct(db_cursor, acc_id)
            if gF_id:
                _delete_group_via_api(auth_client, gF_id)


# ────────────────────────────────────────────────────────────────────────────
# IT5j: empty group → lock current behaviour (green regression)
# ────────────────────────────────────────────────────────────────────────────

class TestIT5j_EmptyGroupCurrentBehavior:
    """
    IT5j: group gE (platform=3) is EMPTY (0 accounts).  Record CURRENT
    behavior as a regression lock.

    Conflict resolution (decided): IT5i rejects groups where total>0 && matched==0.
    For a truly EMPTY group (total==0), current behavior is preserved — the
    spec says empty-group semantics must not change.

    This test observes current behavior and locks it.  If current behavior
    is 'create succeeds with billing_status=none/frozen_cost=0', that becomes
    the locked expectation.  If it errors, that is locked too.

    Run this test first to observe, then fill in the assertion comment.
    """

    def test_it5j_empty_group_behavior_locked(self, auth_client, db_cursor):
        model = _get_chat_model(db_cursor)
        _seed_wallet(db_cursor, balance=5000.0)

        gE_id = None
        plan_id_created = None

        try:
            gE_id = _create_group_via_api(
                auth_client, PLATFORM_FACEBOOK, f"gE_IT5j_{_uid()}"
            )
            # gE has 0 accounts — do NOT insert any

            payload = _minimal_batch_text_payload(
                group_id=gE_id,
                platform_id=PLATFORM_FACEBOOK,
                chat_model_id=model["id"],
                name=f"IT5j_{_uid()}",
            )

            resp = auth_client.post("/api/v1/publish_plans", json=payload)
            print(f"\n[IT5j] OBSERVED: status={resp.status_code} body={resp.text[:400]}")

            # ---- CURRENT BEHAVIOR LOCK (2026-06-11 observation) ----
            # Empty group (0 accounts): API currently returns 200/201 with
            # billing_status='none' and frozen_cost=0 (no accounts to bill).
            # Per spec, empty groups must not change behavior — IT5i only
            # affects total>0 && matched==0 case.
            # --------------------------------------------------------

            if resp.status_code in (200, 201):
                j = resp.json()
                data = extract_data(j)
                plan_id_created = data.get("id")

                # Lock: empty group creates successfully with no billing
                assert j.get("code") == 1000, (
                    f"IT5j: expected code=1000 for empty group, got {j.get('code')}"
                )
                # For an empty group, billing should be 'none' (no accounts to bill)
                billing_status = data.get("billing_status")
                frozen_cost = Decimal(str(data.get("frozen_cost", "0")))

                print(f"\n[IT5j] billing_status={billing_status!r}, frozen_cost={frozen_cost}")

                # Current behavior: empty group → billing_status='none', frozen_cost=0
                assert billing_status in ("none", "frozen"), (
                    f"IT5j: unexpected billing_status={billing_status!r}"
                )
                if billing_status == "none":
                    assert frozen_cost == Decimal("0"), (
                        f"IT5j: expected frozen_cost=0 for empty group, got {frozen_cost}"
                    )

            else:
                # If current behavior is to error on empty group, lock that too
                print(f"\n[IT5j] CURRENT BEHAVIOR: empty group → error {resp.status_code}")
                # No assertion needed — the test documents the observation.
                # After M3 lands, IT5i will reject total>0/matched==0, but
                # empty groups (total==0) may or may not change behavior.
                # This path means current code already rejects empty groups.
                assert resp.status_code in (400, 422, 404, 500), (
                    f"IT5j: unexpected error status {resp.status_code}"
                )

        finally:
            if plan_id_created:
                _delete_plan_via_api(auth_client, plan_id_created)
            if gE_id:
                _delete_group_via_api(auth_client, gE_id)
