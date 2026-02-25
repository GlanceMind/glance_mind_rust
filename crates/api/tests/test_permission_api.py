#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
GlanceMind API E2E Tests - Permission System (Bitmask)
======================================================
Tests permission middleware using gm_users.permissions bitmask.

Bit layout:
  bit 0 (1):  dm_control
  bit 1 (2):  ai_publish
  bit 2 (4):  ai_content_gen
  bit 3 (8):  ai_lead_gen

Default = 14 (dm off, others on)

Run: pytest tests/test_permission_api.py -v --tb=short
"""

import pytest

try:
    from conftest import (
        assert_response_success,
        extract_data,
        TEST_USER_ID,
    )
except ImportError:
    def assert_response_success(resp):
        assert resp.status_code in [200, 201], f"Expected success, got {resp.status_code}: {resp.text}"
    def extract_data(json_resp):
        return json_resp["data"] if isinstance(json_resp, dict) and "data" in json_resp else json_resp
    TEST_USER_ID = 999


# Permission bits
DM_CONTROL = 1
AI_PUBLISH = 2
AI_CONTENT_GEN = 4
AI_LEAD_GEN = 8
DEFAULT_PERMS = AI_PUBLISH | AI_CONTENT_GEN | AI_LEAD_GEN  # 14


def set_permissions(db_cursor, user_id, bits):
    """Set permissions bitmask directly."""
    db_cursor.execute(
        "UPDATE gm_users SET permissions = %s, updated_at = NOW() WHERE id = %s",
        (bits, user_id),
    )
    db_cursor.connection.commit()


def get_permissions(db_cursor, user_id):
    """Get current permissions bitmask."""
    db_cursor.execute("SELECT permissions FROM gm_users WHERE id = %s", (user_id,))
    row = db_cursor.fetchone()
    return row["permissions"] if row else None


# =============================================================================
# TestPermissionDefaults
# =============================================================================

class TestPermissionDefaults:
    """Test DB column default and reset-to-default behaviour."""

    def test_01_db_column_default_is_14(self, db_cursor):
        """The DB column default for gm_users.permissions should be 14."""
        db_cursor.execute("""
            SELECT column_default FROM information_schema.columns
            WHERE table_name = 'gm_users' AND column_name = 'permissions'
        """)
        row = db_cursor.fetchone()
        assert row is not None, "permissions column not found"
        assert str(row["column_default"]) == "14", \
            f"Expected DB default 14, got {row['column_default']}"
        print("  OK: DB column default = 14")

    def test_02_reset_to_default_dm_disabled(self, db_cursor):
        """After setting permissions to default (14), dm_control bit should be 0."""
        set_permissions(db_cursor, TEST_USER_ID, DEFAULT_PERMS)
        bits = get_permissions(db_cursor, TEST_USER_ID)
        if bits is None:
            pytest.skip("User not found")
        assert (bits & DM_CONTROL) == 0, "dm_control should be disabled"
        print("  OK: dm_control disabled at default=14")

    def test_03_reset_to_default_others_enabled(self, db_cursor):
        """After setting permissions to default (14), other features should be on."""
        set_permissions(db_cursor, TEST_USER_ID, DEFAULT_PERMS)
        bits = get_permissions(db_cursor, TEST_USER_ID)
        if bits is None:
            pytest.skip("User not found")
        assert (bits & AI_PUBLISH) != 0
        assert (bits & AI_CONTENT_GEN) != 0
        assert (bits & AI_LEAD_GEN) != 0
        print("  OK: other features enabled at default=14")


# =============================================================================
# TestPermissionMiddleware
# =============================================================================

class TestPermissionMiddleware:
    """Test permission middleware blocks/allows based on bitmask."""

    def test_04_dm_blocked(self, auth_client, db_cursor):
        """DM endpoints return 403 when dm_control bit is 0."""
        set_permissions(db_cursor, TEST_USER_ID, DEFAULT_PERMS)  # dm off

        resp = auth_client.get("/api/v1/dm/conversations")
        assert resp.status_code == 403, f"Expected 403, got {resp.status_code}"
        data = resp.json()
        assert data.get("code") == 4050
        print("  OK: DM blocked")

    def test_05_dm_allowed(self, auth_client, db_cursor):
        """DM endpoints succeed when dm_control bit is set."""
        set_permissions(db_cursor, TEST_USER_ID, DEFAULT_PERMS | DM_CONTROL)  # 15

        resp = auth_client.get("/api/v1/dm/conversations")
        assert resp.status_code != 403
        print(f"  OK: DM allowed (status: {resp.status_code})")

    def test_06_campaigns_blocked(self, auth_client, db_cursor):
        """Campaign endpoints return 403 when ai_lead_gen bit is 0."""
        set_permissions(db_cursor, TEST_USER_ID, DEFAULT_PERMS & ~AI_LEAD_GEN)  # 6

        resp = auth_client.get("/api/v1/campaigns")
        assert resp.status_code == 403
        print("  OK: campaigns blocked")

    def test_07_campaigns_allowed(self, auth_client, db_cursor):
        """Campaign endpoints succeed when ai_lead_gen bit is set."""
        set_permissions(db_cursor, TEST_USER_ID, DEFAULT_PERMS)

        resp = auth_client.get("/api/v1/campaigns")
        assert resp.status_code != 403
        print(f"  OK: campaigns allowed (status: {resp.status_code})")

    def test_08_video_blocked(self, auth_client, db_cursor):
        """Video endpoints return 403 when ai_content_gen bit is 0."""
        set_permissions(db_cursor, TEST_USER_ID, DEFAULT_PERMS & ~AI_CONTENT_GEN)  # 10

        resp = auth_client.get("/api/v1/video/tasks")
        assert resp.status_code == 403
        print("  OK: video blocked")

    def test_09_video_allowed(self, auth_client, db_cursor):
        """Video endpoints succeed when ai_content_gen bit is set."""
        set_permissions(db_cursor, TEST_USER_ID, DEFAULT_PERMS)

        resp = auth_client.get("/api/v1/video/tasks")
        assert resp.status_code != 403
        print(f"  OK: video allowed (status: {resp.status_code})")

    def test_10_ai_blocked(self, auth_client, db_cursor):
        """AI endpoints return 403 when ai_content_gen bit is 0."""
        set_permissions(db_cursor, TEST_USER_ID, DEFAULT_PERMS & ~AI_CONTENT_GEN)

        resp = auth_client.get("/api/v1/ai/models")
        assert resp.status_code == 403, f"Expected 403, got {resp.status_code}"
        print("  OK: AI blocked")

    def test_11_ai_allowed(self, auth_client, db_cursor):
        """AI endpoints succeed when ai_content_gen bit is set."""
        set_permissions(db_cursor, TEST_USER_ID, DEFAULT_PERMS)

        resp = auth_client.get("/api/v1/ai/models")
        assert resp.status_code != 403
        print(f"  OK: AI allowed (status: {resp.status_code})")

    def test_12_publish_plans_blocked(self, auth_client, db_cursor):
        """Publish plans return 403 when ai_publish bit is 0."""
        set_permissions(db_cursor, TEST_USER_ID, DEFAULT_PERMS & ~AI_PUBLISH)

        resp = auth_client.get("/api/v1/publish_plans")
        assert resp.status_code == 403, f"Expected 403, got {resp.status_code}"
        print("  OK: publish_plans blocked")

    def test_13_publish_plans_allowed(self, auth_client, db_cursor):
        """Publish plans succeed when ai_publish bit is set."""
        set_permissions(db_cursor, TEST_USER_ID, DEFAULT_PERMS)

        resp = auth_client.get("/api/v1/publish_plans")
        assert resp.status_code != 403
        print(f"  OK: publish_plans allowed (status: {resp.status_code})")

    def test_14_upload_tasks_blocked(self, auth_client, db_cursor):
        """Upload-tasks return 403 when ai_publish bit is 0."""
        set_permissions(db_cursor, TEST_USER_ID, DEFAULT_PERMS & ~AI_PUBLISH)

        resp = auth_client.get("/api/v1/upload-tasks")
        assert resp.status_code == 403, f"Expected 403, got {resp.status_code}"
        print("  OK: upload-tasks blocked")

    def test_15_upload_tasks_allowed(self, auth_client, db_cursor):
        """Upload-tasks succeed when ai_publish bit is set."""
        set_permissions(db_cursor, TEST_USER_ID, DEFAULT_PERMS)

        resp = auth_client.get("/api/v1/upload-tasks")
        assert resp.status_code != 403
        print(f"  OK: upload-tasks allowed (status: {resp.status_code})")

    def test_16_agent_blocked(self, auth_client, db_cursor):
        """Agent endpoints return 403 when ai_lead_gen bit is 0."""
        set_permissions(db_cursor, TEST_USER_ID, DEFAULT_PERMS & ~AI_LEAD_GEN)

        resp = auth_client.get("/api/v1/agent/comments")
        assert resp.status_code == 403, f"Expected 403, got {resp.status_code}"
        print("  OK: agent blocked")

    def test_17_agent_allowed(self, auth_client, db_cursor):
        """Agent endpoints succeed when ai_lead_gen bit is set."""
        set_permissions(db_cursor, TEST_USER_ID, DEFAULT_PERMS)

        resp = auth_client.get("/api/v1/agent/comments")
        assert resp.status_code != 403
        print(f"  OK: agent allowed (status: {resp.status_code})")

    def test_18_scan_blocked(self, auth_client, db_cursor):
        """Scan endpoints return 403 when ai_lead_gen bit is 0."""
        set_permissions(db_cursor, TEST_USER_ID, DEFAULT_PERMS & ~AI_LEAD_GEN)

        resp = auth_client.get("/api/v1/scan/tasks")
        assert resp.status_code == 403, f"Expected 403, got {resp.status_code}"
        print("  OK: scan blocked")

    def test_19_scan_allowed(self, auth_client, db_cursor):
        """Scan endpoints succeed when ai_lead_gen bit is set."""
        set_permissions(db_cursor, TEST_USER_ID, DEFAULT_PERMS)

        resp = auth_client.get("/api/v1/scan/tasks")
        assert resp.status_code != 403
        print(f"  OK: scan allowed (status: {resp.status_code})")


# =============================================================================
# TestPermissionBoundary
# =============================================================================

class TestPermissionBoundary:
    """Test boundary values: all off (0) and all on (15)."""

    ALL_ON = DM_CONTROL | AI_PUBLISH | AI_CONTENT_GEN | AI_LEAD_GEN  # 15

    PROTECTED_ENDPOINTS = [
        ("/api/v1/dm/conversations", "dm_control"),
        ("/api/v1/campaigns", "ai_lead_gen"),
        ("/api/v1/video/tasks", "ai_content_gen"),
        ("/api/v1/upload-tasks", "ai_publish"),
    ]

    def test_20_all_off_blocks_everything(self, auth_client, db_cursor):
        """bits=0 should block all protected endpoints."""
        set_permissions(db_cursor, TEST_USER_ID, 0)

        for endpoint, perm_name in self.PROTECTED_ENDPOINTS:
            resp = auth_client.get(endpoint)
            assert resp.status_code == 403, \
                f"{endpoint} ({perm_name}) should be 403 with bits=0, got {resp.status_code}"
        print("  OK: bits=0 blocks all")

    def test_21_all_on_allows_everything(self, auth_client, db_cursor):
        """bits=15 should allow all protected endpoints."""
        set_permissions(db_cursor, TEST_USER_ID, self.ALL_ON)

        for endpoint, perm_name in self.PROTECTED_ENDPOINTS:
            resp = auth_client.get(endpoint)
            assert resp.status_code != 403, \
                f"{endpoint} ({perm_name}) should not be 403 with bits=15, got {resp.status_code}"
        print("  OK: bits=15 allows all")

    def test_22_error_message_contains_permission_name(self, auth_client, db_cursor):
        """403 response body should include the denied permission name."""
        set_permissions(db_cursor, TEST_USER_ID, DEFAULT_PERMS)  # dm off

        resp = auth_client.get("/api/v1/dm/conversations")
        assert resp.status_code == 403
        body = resp.json()
        assert body.get("code") == 4050
        msg = body.get("message", body.get("msg", ""))
        assert "dm_control" in msg.lower() or "permission" in msg.lower(), \
            f"Expected permission name in message, got: {msg}"
        print(f"  OK: error message = {msg}")


# =============================================================================
# TestPermissionAndCharging
# =============================================================================

class TestPermissionAndCharging:
    """Permission check happens before charging."""

    def test_30_no_charge_on_permission_denied(self, auth_client, db_cursor):
        """Balance unchanged when permission denies the request."""
        set_permissions(db_cursor, TEST_USER_ID, DEFAULT_PERMS & ~AI_CONTENT_GEN)

        db_cursor.execute(
            "SELECT balance_points FROM gm_user_wallets WHERE user_id = %s",
            (TEST_USER_ID,),
        )
        wallet = db_cursor.fetchone()
        if not wallet:
            pytest.skip("No wallet")
        before = wallet["balance_points"]

        resp = auth_client.post("/api/v1/ai/generate", json={"prompt": "test"})
        assert resp.status_code == 403

        db_cursor.execute(
            "SELECT balance_points FROM gm_user_wallets WHERE user_id = %s",
            (TEST_USER_ID,),
        )
        after = db_cursor.fetchone()["balance_points"]
        assert after == before, "Balance should be unchanged"
        print("  OK: no charge on permission denied")


# =============================================================================
# Cleanup
# =============================================================================

class TestPermissionCleanup:
    """Restore all permissions so other test files are not affected."""

    ALL_ON = DM_CONTROL | AI_PUBLISH | AI_CONTENT_GEN | AI_LEAD_GEN  # 15

    def test_99_restore(self, db_cursor):
        """Reset permissions to all-on (15) for other tests."""
        set_permissions(db_cursor, TEST_USER_ID, self.ALL_ON)
        print("  OK: restored to all-on (15)")
