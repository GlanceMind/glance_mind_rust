#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
M4 campaign group guard — RED test suite (IT6a-h)
=================================================
Tests for group-platform guard on campaign create/update.

RED today: campaign_service.rs create/update pass social_group_id verbatim
with NO platform validation — all guard tests that expect 400 currently get
200 (pass-through).

Run:
    pytest crates/api/tests/test_campaign_group_guard.py -v

Requires:
    - API server running on API_BASE_URL (127.0.0.1:8081) with no_proxy set
    - Postgres reachable via DATABASE_URL

Isolation:
    Class-scoped fixture seeds groups/campaigns; try/finally cleans up.
    db_cursor fixture rolls back uncommitted changes; explicit commits in
    fixture with compensating DELETE in teardown.
"""

from __future__ import annotations

import os
import uuid
from typing import Optional

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
        PLATFORM_FACEBOOK,
        DATABASE_URL,
    )
except ImportError:
    PLATFORM_REDDIT = 1
    PLATFORM_FACEBOOK = 3
    TEST_USER_ID = 999
    DATABASE_URL = os.getenv(
        "DATABASE_URL",
        "postgres://glancemind:testpassword@localhost:5432/glancemind_test",
    )

    def assert_response_success(resp, expected_status=200):
        assert resp.status_code == expected_status, f"Expected {expected_status}: {resp.text}"

    def extract_data(j):
        return j.get("data", j) if isinstance(j, dict) else j


API_BASE_URL = os.getenv("API_BASE_URL", "http://127.0.0.1:8081")


# ────────────────────────────────────────────────────────────────────────────
# Helpers
# ────────────────────────────────────────────────────────────────────────────

def _uid() -> str:
    return uuid.uuid4().hex[:8]


def _auth_client() -> APIClient:
    token = get_or_create_test_token()
    return APIClient(API_BASE_URL, token)


def _get_region_and_model(db_cursor, platform_id: int):
    """Return (region_id, ai_model_id) for a given platform."""
    db_cursor.execute(
        """
        SELECT id FROM gm_regions
        WHERE platform_id = %s AND is_active = true
        ORDER BY id LIMIT 1
        """,
        (platform_id,),
    )
    region_row = db_cursor.fetchone()
    assert region_row is not None, f"No active region for platform_id={platform_id}"

    db_cursor.execute("SELECT id FROM gm_ai_models ORDER BY id LIMIT 1")
    ai_model_row = db_cursor.fetchone()
    assert ai_model_row is not None, "No AI model in DB"

    return region_row["id"], ai_model_row["id"]


def _create_group(auth_client, platform_id: int, name: str) -> int:
    resp = auth_client.post(
        "/api/v1/social-groups",
        json={"platform_id": platform_id, "group_name": name},
    )
    assert resp.status_code in (200, 201), (
        f"Failed to create group '{name}' for platform {platform_id}: "
        f"{resp.status_code} {resp.text}"
    )
    return extract_data(resp.json())["id"]


def _delete_group(auth_client, group_id: int) -> None:
    try:
        auth_client.delete(f"/api/v1/social-groups/{group_id}")
    except Exception:
        pass


def _create_fb_campaign(auth_client, db_cursor, name: str, group_id: Optional[int] = None) -> int:
    """Create a facebook (platform_id=3) campaign; optionally bind a group."""
    region_id, ai_model_id = _get_region_and_model(db_cursor, PLATFORM_FACEBOOK)
    payload = {
        "name": name,
        "platform_id": PLATFORM_FACEBOOK,
        "region_id": region_id,
        "ai_model_id": ai_model_id,
        "schedule_type": "ONCE",
        "product_prompt": f"M4 guard test product {name}",
        "max_scan_count": 1,
    }
    if group_id is not None:
        payload["social_group_id"] = group_id

    resp = auth_client.post("/api/v1/campaigns", json=payload)
    assert_response_success(resp)
    return extract_data(resp.json())["id"]


def _delete_campaign(db_cursor, campaign_id: int) -> None:
    """Direct DB delete of a campaign and its crawler tasks."""
    try:
        db_cursor.execute(
            "DELETE FROM gm_crawler_tasks WHERE campaign_id = %s", (campaign_id,)
        )
        db_cursor.execute(
            "DELETE FROM gm_campaign_templates WHERE campaign_id = %s", (campaign_id,)
        )
        db_cursor.execute(
            "DELETE FROM gm_campaigns WHERE id = %s", (campaign_id,)
        )
        db_cursor.connection.commit()
    except Exception:
        db_cursor.connection.rollback()


def _seed_user_b_group(db_cursor, platform_id: int) -> int:
    """Insert a group owned by a different user (user_id != 999); return group id."""
    uid = _uid()
    db_cursor.execute(
        """
        INSERT INTO gm_users (email, username, password_hash, status, full_name, role,
                               is_active, created_at, permissions)
        VALUES (%s, %s, 'unused', 'active', 'User B M4', 'user', true, NOW(), 0)
        RETURNING id
        """,
        (f"user_b_m4_{uid}@example.com", f"user_b_m4_{uid}"),
    )
    user_b_id = db_cursor.fetchone()["id"]

    db_cursor.execute(
        """
        INSERT INTO gm_social_groups (user_id, platform_id, group_name, created_at)
        VALUES (%s, %s, %s, NOW())
        RETURNING id
        """,
        (user_b_id, platform_id, f"user_b_m4_group_{uid}"),
    )
    group_b_id = db_cursor.fetchone()["id"]
    db_cursor.connection.commit()
    return group_b_id, user_b_id


def _delete_user_b_artifacts(db_cursor, group_b_id: int, user_b_id: int) -> None:
    try:
        db_cursor.execute("DELETE FROM gm_social_groups WHERE id = %s", (group_b_id,))
        db_cursor.execute("DELETE FROM gm_users WHERE id = %s", (user_b_id,))
        db_cursor.connection.commit()
    except Exception:
        db_cursor.connection.rollback()


# ────────────────────────────────────────────────────────────────────────────
# Test class
# ────────────────────────────────────────────────────────────────────────────

class TestCampaignGroupGuard:
    """
    IT6a-h: platform guard for social_group_id on campaign create/update.

    Seed (once per class, cleaned up in teardown):
        user A  = test user 999
        gF      = facebook group (platform_id=3) owned by user A
        gF2     = second facebook group (platform_id=3) owned by user A
        gR      = reddit group (platform_id=1) owned by user A
        gB      = facebook group (platform_id=3) owned by user B (foreign)

    RED tests (expect 400 or 404 but currently get 200):
        IT6a, IT6b, IT6e, IT6h

    GREEN-baseline tests (should pass today and after M4):
        IT6c, IT6d, IT6f, IT6g
    """

    # ── class-level seed state ────────────────────────────────────────────

    @pytest.fixture(autouse=True, scope="class")
    def seed(self, db_connection):
        """
        Seed groups and clean up at end.
        Uses class scope so groups are created once and shared across tests.
        """
        import psycopg2
        from psycopg2.extras import RealDictCursor

        conn = psycopg2.connect(DATABASE_URL)
        conn.autocommit = False
        cursor = conn.cursor(cursor_factory=RealDictCursor)

        auth = _auth_client()
        uid = _uid()

        gF_id = None
        gF2_id = None
        gR_id = None
        gB_id = None
        user_b_id = None

        try:
            # Create groups via API (user A owns them)
            gF_id = _create_group(auth, PLATFORM_FACEBOOK, f"m4_gF_{uid}")
            gF2_id = _create_group(auth, PLATFORM_FACEBOOK, f"m4_gF2_{uid}")
            gR_id = _create_group(auth, PLATFORM_REDDIT, f"m4_gR_{uid}")

            # Create user B's group directly in DB
            gB_id, user_b_id = _seed_user_b_group(cursor, PLATFORM_FACEBOOK)

            # Store on class so individual tests can access
            TestCampaignGroupGuard._gF_id = gF_id
            TestCampaignGroupGuard._gF2_id = gF2_id
            TestCampaignGroupGuard._gR_id = gR_id
            TestCampaignGroupGuard._gB_id = gB_id
            TestCampaignGroupGuard._user_b_id = user_b_id
            TestCampaignGroupGuard._auth = auth

            yield {
                "gF": gF_id,
                "gF2": gF2_id,
                "gR": gR_id,
                "gB": gB_id,
            }
        finally:
            # Clean up groups (best-effort; DELETE cascade handles accounts)
            if gF_id is not None:
                _delete_group(auth, gF_id)
            if gF2_id is not None:
                _delete_group(auth, gF2_id)
            if gR_id is not None:
                _delete_group(auth, gR_id)
            if gB_id is not None and user_b_id is not None:
                _delete_user_b_artifacts(cursor, gB_id, user_b_id)
            cursor.close()
            conn.close()

    def _region_model(self, db_cursor, platform_id=PLATFORM_FACEBOOK):
        return _get_region_and_model(db_cursor, platform_id)

    # ── IT6a: create with mismatched platform → 400 code==2001 ──────────────

    def test_IT6a_create_fb_platform_with_reddit_group_returns_400(
        self, db_cursor, db_connection
    ):
        """
        RED: create platform_id=3 (facebook) + social_group_id=gR (reddit=1)
        Expected after M4: 400 with code==2001, msg containing "does not match".
        Current (pre-M4): 200 — guard not implemented.
        """
        auth = self._auth
        region_id, ai_model_id = self._region_model(db_cursor, PLATFORM_FACEBOOK)

        payload = {
            "name": f"IT6a_fb_reddit_group_{_uid()}",
            "platform_id": PLATFORM_FACEBOOK,
            "region_id": region_id,
            "ai_model_id": ai_model_id,
            "schedule_type": "ONCE",
            "product_prompt": "IT6a test",
            "max_scan_count": 1,
            "social_group_id": self._gR_id,
        }
        resp = auth.post("/api/v1/campaigns", json=payload)

        campaign_id = None
        try:
            # After M4: expect 400
            assert resp.status_code == 400, (
                f"IT6a RED: expected 400, got {resp.status_code}. "
                f"Group {self._gR_id} is reddit but campaign is facebook. "
                f"Body: {resp.text}"
            )
            body = resp.json()
            assert body.get("code") == 2001, (
                f"IT6a: expected code==2001 (BadRequest), got {body.get('code')}"
            )
            assert "does not match" in (body.get("msg", "") + body.get("msg_cn", "")).lower(), (
                f"IT6a: expected 'does not match' in msg, got: {body}"
            )
        except AssertionError:
            # If the API returned 200 (pre-M4), clean up the created campaign
            if resp.status_code == 200:
                campaign_id = extract_data(resp.json()).get("id")
            raise
        finally:
            if campaign_id is not None:
                _delete_campaign(db_cursor, campaign_id)

    # ── IT6b: create with foreign user's group → 404 ────────────────────────

    def test_IT6b_create_with_foreign_user_group_returns_404(
        self, db_cursor, db_connection
    ):
        """
        RED: create campaign with social_group_id=gB (owned by user B)
        Expected after M4: 404 (group not found for caller).
        Current (pre-M4): 200 — no ownership check.
        """
        auth = self._auth
        region_id, ai_model_id = self._region_model(db_cursor, PLATFORM_FACEBOOK)

        payload = {
            "name": f"IT6b_foreign_group_{_uid()}",
            "platform_id": PLATFORM_FACEBOOK,
            "region_id": region_id,
            "ai_model_id": ai_model_id,
            "schedule_type": "ONCE",
            "product_prompt": "IT6b test",
            "max_scan_count": 1,
            "social_group_id": self._gB_id,
        }
        resp = auth.post("/api/v1/campaigns", json=payload)

        campaign_id = None
        try:
            assert resp.status_code == 404, (
                f"IT6b RED: expected 404, got {resp.status_code}. "
                f"Group {self._gB_id} belongs to user B, caller is user 999. "
                f"Body: {resp.text}"
            )
        except AssertionError:
            if resp.status_code == 200:
                campaign_id = extract_data(resp.json()).get("id")
            raise
        finally:
            if campaign_id is not None:
                _delete_campaign(db_cursor, campaign_id)

    # ── IT6c: create without social_group_id → GREEN baseline ───────────────

    def test_IT6c_create_without_group_succeeds(self, db_cursor, db_connection):
        """
        GREEN baseline: create campaign without social_group_id must succeed.
        This behavior must be preserved after M4.
        """
        auth = self._auth
        region_id, ai_model_id = self._region_model(db_cursor, PLATFORM_FACEBOOK)

        payload = {
            "name": f"IT6c_no_group_{_uid()}",
            "platform_id": PLATFORM_FACEBOOK,
            "region_id": region_id,
            "ai_model_id": ai_model_id,
            "schedule_type": "ONCE",
            "product_prompt": "IT6c test",
            "max_scan_count": 1,
            # no social_group_id
        }
        resp = auth.post("/api/v1/campaigns", json=payload)
        campaign_id = None
        try:
            assert_response_success(resp)
            data = extract_data(resp.json())
            assert "id" in data, "IT6c: response must include id"
            campaign_id = data["id"]
            assert data.get("social_group_id") is None, (
                f"IT6c: social_group_id should be null, got {data.get('social_group_id')}"
            )
        finally:
            if campaign_id is not None:
                _delete_campaign(db_cursor, campaign_id)

    # ── IT6d: create with matching group → GREEN baseline ───────────────────

    def test_IT6d_create_with_matching_fb_group_succeeds_and_persists(
        self, db_cursor, db_connection
    ):
        """
        GREEN baseline: create fb campaign + gF (facebook group) → success + DB row has correct group.
        """
        auth = self._auth
        region_id, ai_model_id = self._region_model(db_cursor, PLATFORM_FACEBOOK)

        payload = {
            "name": f"IT6d_fb_match_{_uid()}",
            "platform_id": PLATFORM_FACEBOOK,
            "region_id": region_id,
            "ai_model_id": ai_model_id,
            "schedule_type": "ONCE",
            "product_prompt": "IT6d test",
            "max_scan_count": 1,
            "social_group_id": self._gF_id,
        }
        resp = auth.post("/api/v1/campaigns", json=payload)
        campaign_id = None
        try:
            assert_response_success(resp)
            data = extract_data(resp.json())
            campaign_id = data["id"]
            assert data.get("social_group_id") == self._gF_id, (
                f"IT6d: response social_group_id should be {self._gF_id}, got {data.get('social_group_id')}"
            )
            # Verify in DB
            db_connection.commit()
            db_cursor.execute(
                "SELECT social_group_id FROM gm_campaigns WHERE id = %s", (campaign_id,)
            )
            row = db_cursor.fetchone()
            assert row is not None, "IT6d: campaign not found in DB"
            assert row["social_group_id"] == self._gF_id, (
                f"IT6d: DB social_group_id should be {self._gF_id}, got {row['social_group_id']}"
            )
        finally:
            if campaign_id is not None:
                _delete_campaign(db_cursor, campaign_id)

    # ── IT6e: update from fb group to reddit group → 400 ────────────────────

    def test_IT6e_update_fb_campaign_to_reddit_group_returns_400(
        self, db_cursor, db_connection
    ):
        """
        RED: create valid fb campaign with gF, then PUT social_group_id=gR (reddit).
        Expected after M4: 400.
        Current (pre-M4): 200.
        """
        auth = self._auth
        campaign_id = None
        try:
            campaign_id = _create_fb_campaign(
                auth, db_cursor, f"IT6e_setup_{_uid()}", group_id=self._gF_id
            )

            resp = auth.put(
                f"/api/v1/campaigns/{campaign_id}",
                json={"social_group_id": self._gR_id},
            )
            assert resp.status_code == 400, (
                f"IT6e RED: expected 400 when updating fb campaign to reddit group, "
                f"got {resp.status_code}. Body: {resp.text}"
            )
            body = resp.json()
            assert body.get("code") == 2001, (
                f"IT6e: expected code==2001, got {body.get('code')}"
            )
            assert "does not match" in (body.get("msg", "") + body.get("msg_cn", "")).lower(), (
                f"IT6e: expected 'does not match' in msg, got: {body}"
            )
        finally:
            if campaign_id is not None:
                _delete_campaign(db_cursor, campaign_id)

    # ── IT6f: update without changing group → GREEN baseline ────────────────

    def test_IT6f_update_without_group_change_keeps_group(
        self, db_cursor, db_connection
    ):
        """
        GREEN baseline: PUT without social_group_id / without platform change
        must leave social_group_id unchanged.
        """
        auth = self._auth
        campaign_id = None
        try:
            campaign_id = _create_fb_campaign(
                auth, db_cursor, f"IT6f_setup_{_uid()}", group_id=self._gF_id
            )

            resp = auth.put(
                f"/api/v1/campaigns/{campaign_id}",
                json={"name": f"IT6f_updated_{_uid()}"},
            )
            assert_response_success(resp)
            data = extract_data(resp.json())
            assert data.get("social_group_id") == self._gF_id, (
                f"IT6f: group should stay {self._gF_id}, got {data.get('social_group_id')}"
            )
            # Verify DB
            db_connection.commit()
            db_cursor.execute(
                "SELECT social_group_id FROM gm_campaigns WHERE id = %s", (campaign_id,)
            )
            row = db_cursor.fetchone()
            assert row["social_group_id"] == self._gF_id, (
                f"IT6f: DB social_group_id should stay {self._gF_id}"
            )
        finally:
            if campaign_id is not None:
                _delete_campaign(db_cursor, campaign_id)

    # ── IT6g: update to matching fb group → GREEN baseline ──────────────────

    def test_IT6g_update_to_matching_fb_group_succeeds(
        self, db_cursor, db_connection
    ):
        """
        GREEN baseline: PUT social_group_id=gF2 (also facebook) → success + DB updated.
        """
        auth = self._auth
        campaign_id = None
        try:
            campaign_id = _create_fb_campaign(
                auth, db_cursor, f"IT6g_setup_{_uid()}", group_id=self._gF_id
            )

            resp = auth.put(
                f"/api/v1/campaigns/{campaign_id}",
                json={"social_group_id": self._gF2_id},
            )
            assert_response_success(resp)
            data = extract_data(resp.json())
            assert data.get("social_group_id") == self._gF2_id, (
                f"IT6g: social_group_id should be {self._gF2_id}, got {data.get('social_group_id')}"
            )
            # DB check
            db_connection.commit()
            db_cursor.execute(
                "SELECT social_group_id FROM gm_campaigns WHERE id = %s", (campaign_id,)
            )
            row = db_cursor.fetchone()
            assert row["social_group_id"] == self._gF2_id, (
                f"IT6g: DB social_group_id should be {self._gF2_id}"
            )
        finally:
            if campaign_id is not None:
                _delete_campaign(db_cursor, campaign_id)

    # ── IT6h: THE BYPASS — update platform only, group stays mismatched ─────

    def test_IT6h_update_platform_only_leaves_existing_group_mismatched_returns_400(
        self, db_cursor, db_connection
    ):
        """
        RED (THE BYPASS): fb campaign bound to gF; PUT with ONLY platform_id=1 (reddit),
        no social_group_id in body.

        After M4: the effective group (gF=facebook) mismatches the new platform (reddit=1)
        → 400 must be returned.

        Current (pre-M4): 200 — service builds changeset as:
            platform_id = 1 (from dto)
            social_group_id = existing gF (facebook group, unchanged)
        resulting in a reddit campaign with a facebook group — the bypass.
        """
        auth = self._auth
        campaign_id = None
        try:
            campaign_id = _create_fb_campaign(
                auth, db_cursor, f"IT6h_setup_{_uid()}", group_id=self._gF_id
            )

            # PUT only platform_id=1 (reddit), no social_group_id
            resp = auth.put(
                f"/api/v1/campaigns/{campaign_id}",
                json={"platform_id": PLATFORM_REDDIT},
            )
            assert resp.status_code == 400, (
                f"IT6h RED (THE BYPASS): expected 400, got {resp.status_code}. "
                f"Campaign {campaign_id} has group {self._gF_id} (facebook), "
                f"new platform=reddit → effective group-platform mismatch. "
                f"Body: {resp.text}"
            )
            body = resp.json()
            assert body.get("code") == 2001, (
                f"IT6h: expected code==2001, got {body.get('code')}"
            )
            assert "does not match" in (body.get("msg", "") + body.get("msg_cn", "")).lower(), (
                f"IT6h: expected 'does not match' in error msg, got: {body}"
            )
        except AssertionError:
            # If it wrongly returned 200, verify the bypass scenario exists in DB
            if resp.status_code == 200:
                db_connection.commit()
                db_cursor.execute(
                    """
                    SELECT c.platform_id, c.social_group_id, g.platform_id AS group_platform_id
                    FROM gm_campaigns c
                    LEFT JOIN gm_social_groups g ON g.id = c.social_group_id
                    WHERE c.id = %s
                    """,
                    (campaign_id,),
                )
                row = db_cursor.fetchone()
                # Document the bypass: campaign.platform_id=reddit, group.platform_id=facebook
                print(
                    f"\nIT6h BYPASS CONFIRMED: campaign platform={row['platform_id']} "
                    f"(reddit=1), group platform={row['group_platform_id']} (facebook=3)"
                )
            raise
        finally:
            if campaign_id is not None:
                _delete_campaign(db_cursor, campaign_id)
