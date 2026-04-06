#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
GlanceMind API E2E Tests - AI Short Drama Billing Guard
========================================================
Tests that the Rust drama facade enforces billing rules:
- Balance check before project creation
- Cost visibility on project detail
- No direct wallet mutation from gateway

These tests require:
- Rust API running with drama facade
- gm_agent_hub gateway running (or in mock mode)
- Shared Postgres with wallet tables
"""

import pytest
import time

try:
    from conftest import (
        assert_response_success,
        extract_data,
        API_BASE_URL,
        DATABASE_URL,
    )
except ImportError:
    def assert_response_success(resp):
        assert resp.status_code in [200, 201], \
            f"Expected success, got {resp.status_code}: {resp.text}"

    def extract_data(json_resp):
        if "data" in json_resp:
            return json_resp["data"]
        return json_resp


DRAMA_BASE = "/api/v1/drama"


def _valid_brief(**overrides):
    defaults = {
        "title": f"Billing Test {int(time.time())}",
        "description": "A drama to test billing guard.",
        "content_type": "short_video",
        "target_duration_seconds": 60,
        "platform": "tiktok",
    }
    defaults.update(overrides)
    return defaults


class TestDramaBillingGuard:
    """T9: Billing guard integration tests."""

    def test_create_project_checks_balance(self, auth_client, db_cursor):
        """Project creation should not crash even if balance is low.

        The billing guard checks available balance vs estimated cost.
        If balance is insufficient, it returns 402.
        If balance is sufficient (or guard fails softly), it proceeds to gateway.
        """
        brief = _valid_brief()
        resp = auth_client.post(f"{DRAMA_BASE}/projects", json=brief)

        if resp.status_code == 402:
            body = resp.json()
            assert "available_points" in str(body) or "balance" in str(body).lower()
            assert "estimated_cost" in str(body) or "cost" in str(body).lower()
        else:
            assert resp.status_code in [200, 201, 502]

    def test_cost_visible_on_project_detail(self, auth_client):
        """Cost endpoint should return cost information after project creation."""
        brief = _valid_brief()
        resp = auth_client.post(f"{DRAMA_BASE}/projects", json=brief)
        if resp.status_code not in [200, 201]:
            pytest.skip("Could not create project for cost test")

        pid = extract_data(resp.json())["project_id"]
        cost_resp = auth_client.get(f"{DRAMA_BASE}/projects/{pid}/cost")
        assert cost_resp.status_code == 200

    def test_wallet_not_mutated_by_gateway_directly(self, auth_client, db_cursor):
        """Verify that creating a drama project does not directly INSERT into
        gm_wallet_transactions from the gateway side.

        The billing authority must stay in glance_mind_rust. This test captures
        the wallet transaction count before and after project creation and
        verifies no new drama-related transaction appeared from an external source.
        """
        db_cursor.execute(
            "SELECT COUNT(*) as cnt FROM gm_wallet_transactions "
            "WHERE reference_type = 'drama_project'"
        )
        result = db_cursor.fetchone()
        before_count = result["cnt"] if result else 0

        brief = _valid_brief()
        auth_client.post(f"{DRAMA_BASE}/projects", json=brief)

        db_cursor.execute(
            "SELECT COUNT(*) as cnt FROM gm_wallet_transactions "
            "WHERE reference_type = 'drama_project'"
        )
        result = db_cursor.fetchone()
        after_count = result["cnt"] if result else 0

        assert after_count == before_count, \
            f"Gateway should not create wallet transactions directly. " \
            f"Before: {before_count}, After: {after_count}"
