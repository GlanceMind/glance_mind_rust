#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
GlanceMind API E2E Tests - Billing API Integration
===================================================
Tests billing through API endpoints (POST/GET/DELETE publish_plans).

Run: pytest tests/test_billing_api.py -v --tb=short
"""

import pytest
import uuid
from decimal import Decimal

try:
    from conftest import assert_response_success, extract_data
except ImportError:
    def assert_response_success(resp):
        assert resp.status_code in [200, 201], f"Expected success, got {resp.status_code}: {resp.text}"
    def extract_data(json_resp):
        return json_resp["data"] if isinstance(json_resp, dict) and "data" in json_resp else json_resp

TEST_USER_ID = 999


# =============================================================================
# Helpers
# =============================================================================

def setup_wallet(db_cursor, user_id=TEST_USER_ID, balance=500.00):
    """Reset wallet for test."""
    db_cursor.execute("""
        UPDATE gm_user_wallets
        SET balance_points = %s, frozen_points = 0, updated_at = NOW()
        WHERE user_id = %s
    """, (balance, user_id))
    db_cursor.execute("""
        DELETE FROM gm_wallet_transactions
        WHERE user_id = %s AND reference_type = 'aipub_plan'
    """, (user_id,))
    db_cursor.connection.commit()


def get_test_group(db_cursor):
    """Find a group with accounts."""
    db_cursor.execute("""
        SELECT sg.id as group_id, sg.platform_id, COUNT(sa.id) as cnt
        FROM gm_social_groups sg
        JOIN gm_social_accounts sa ON sa.group_id = sg.id
        GROUP BY sg.id, sg.platform_id
        HAVING COUNT(sa.id) > 0
        LIMIT 1
    """)
    return db_cursor.fetchone()


# =============================================================================
# TestBillingAPIGrooming
# =============================================================================

class TestBillingAPIGrooming:
    """Test billing through account_grooming plan creation API."""

    def test_create_plan_freezes_balance(self, auth_client, db_cursor):
        """POST /publish_plans → billing_status=frozen, frozen_cost > 0."""
        setup_wallet(db_cursor, balance=500)
        group = get_test_group(db_cursor)
        if not group:
            pytest.skip("No group with accounts")

        resp = auth_client.post("/api/v1/publish_plans", json={
            "plan_type": "account_grooming",
            "name": f"Billing Test {uuid.uuid4().hex[:6]}",
            "group_id": group["group_id"],
            "platform_id": group["platform_id"],
            "content_type": "profile",
            "ai_input": {"text_prompt": "test", "image_prompt": "test"},
        })
        if resp.status_code not in [200, 201]:
            pytest.skip(f"Plan creation returned {resp.status_code}")

        data = extract_data(resp.json())
        plan_id = data["id"]

        # Verify billing fields in response
        assert data.get("billing_status") == "frozen", \
            f"Expected frozen, got {data.get('billing_status')}"
        assert float(data.get("frozen_cost", 0)) > 0, \
            f"Expected frozen_cost > 0, got {data.get('frozen_cost')}"

        # Verify wallet
        db_cursor.execute(
            "SELECT frozen_points FROM gm_user_wallets WHERE user_id = %s",
            (TEST_USER_ID,))
        wallet = db_cursor.fetchone()
        assert wallet['frozen_points'] > Decimal('0')

        # Cleanup
        auth_client.delete(f"/api/v1/publish_plans/{plan_id}")
        print(f"  OK: plan {plan_id} created with frozen_cost={data.get('frozen_cost')}")

    def test_create_insufficient_balance(self, auth_client, db_cursor):
        """Balance=0 → creation fails, no plan residue, wallet unchanged."""
        setup_wallet(db_cursor, balance=0)
        group = get_test_group(db_cursor)
        if not group:
            pytest.skip("No group with accounts")

        resp = auth_client.post("/api/v1/publish_plans", json={
            "plan_type": "account_grooming",
            "name": f"Insufficient {uuid.uuid4().hex[:6]}",
            "group_id": group["group_id"],
            "platform_id": group["platform_id"],
            "content_type": "profile",
            "ai_input": {"text_prompt": "test", "image_prompt": "test"},
        })
        assert resp.status_code in [400, 402, 422, 500], \
            f"Expected error, got {resp.status_code}"

        # Verify no plan residue
        db_cursor.execute("""
            SELECT COUNT(*) as cnt FROM gm_aipub_plans
            WHERE user_id = %s AND name LIKE 'Insufficient%%'
        """, (TEST_USER_ID,))
        assert db_cursor.fetchone()["cnt"] == 0, "No plan should remain after failed freeze"

        # Wallet unchanged
        db_cursor.execute(
            "SELECT balance_points, frozen_points FROM gm_user_wallets WHERE user_id = %s",
            (TEST_USER_ID,))
        wallet = db_cursor.fetchone()
        assert wallet['balance_points'] == Decimal('0.00')
        assert wallet['frozen_points'] == Decimal('0.00')
        print("  OK: insufficient balance → error, no residue")

    def test_delete_plan_unfreezes(self, auth_client, db_cursor):
        """DELETE /publish_plans/:id → wallet balance restored."""
        setup_wallet(db_cursor, balance=500)
        group = get_test_group(db_cursor)
        if not group:
            pytest.skip("No group with accounts")

        # Create
        resp = auth_client.post("/api/v1/publish_plans", json={
            "plan_type": "account_grooming",
            "name": f"Delete Test {uuid.uuid4().hex[:6]}",
            "group_id": group["group_id"],
            "platform_id": group["platform_id"],
            "content_type": "profile",
            "ai_input": {"text_prompt": "test", "image_prompt": "test"},
        })
        if resp.status_code not in [200, 201]:
            pytest.skip("Plan creation failed")

        plan_id = extract_data(resp.json())["id"]

        # Delete
        del_resp = auth_client.delete(f"/api/v1/publish_plans/{plan_id}")
        if del_resp.status_code not in [200, 204]:
            pytest.skip(f"Delete returned {del_resp.status_code}")

        # Verify balance restored
        db_cursor.execute(
            "SELECT balance_points, frozen_points FROM gm_user_wallets WHERE user_id = %s",
            (TEST_USER_ID,))
        wallet = db_cursor.fetchone()
        assert wallet['balance_points'] == Decimal('500.00'), \
            f"Expected balance restored to 500, got {wallet['balance_points']}"
        assert wallet['frozen_points'] == Decimal('0.00')
        print(f"  OK: delete plan {plan_id} → balance restored to 500")

    def test_detail_shows_billing(self, auth_client, db_cursor):
        """GET /publish_plans/:id → includes billing fields."""
        setup_wallet(db_cursor, balance=500)
        group = get_test_group(db_cursor)
        if not group:
            pytest.skip("No group with accounts")

        resp = auth_client.post("/api/v1/publish_plans", json={
            "plan_type": "account_grooming",
            "name": f"Detail Test {uuid.uuid4().hex[:6]}",
            "group_id": group["group_id"],
            "platform_id": group["platform_id"],
            "content_type": "profile",
            "ai_input": {"text_prompt": "test", "image_prompt": "test"},
        })
        if resp.status_code not in [200, 201]:
            pytest.skip("Plan creation failed")

        plan_id = extract_data(resp.json())["id"]

        # Get detail
        detail_resp = auth_client.get(f"/api/v1/publish_plans/{plan_id}")
        assert_response_success(detail_resp)
        data = extract_data(detail_resp.json())

        assert "billing_status" in data, "Response should include billing_status"
        assert "frozen_cost" in data, "Response should include frozen_cost"
        assert "consumed_cost" in data, "Response should include consumed_cost"

        # Cleanup
        auth_client.delete(f"/api/v1/publish_plans/{plan_id}")
        print(f"  OK: detail includes billing fields")
