"""
GlanceMind API E2E Tests - Insufficient Balance Gate
====================================================

Verifies that the backend returns the unified ``code: 4100`` (HTTP 402)
response when a user attempts to create a Campaign or Novel project
without sufficient wallet balance.

The contract tested here is consumed by the front-end
``insufficientBalanceStore`` / ``InsufficientBalanceDialog`` (see
``docs/superpowers/specs/2026-04-16-insufficient-balance-popup-design.md``):

    {
      "code": 4100,
      "msg": "Insufficient balance, please top up",
      "msg_cn": "余额不足，请先充值",
      ...
    }

Implementation notes:
- Campaign balance gate lives in ``campaign_service::create_campaign`` and
  compares available wallet balance against ``max(min_cost, budget_cap)``.
- Novel balance gate lives in ``novel_service::check_balance_threshold`` and
  enforces a flat ``NOVEL_MIN_BALANCE_POINTS`` (100) threshold on the
  create/generate endpoints via ``enforce_novel_balance`` in
  ``novel_handler``.

These tests require the Rust API and Postgres to be running (see
``conftest.py``). They operate on the shared test user's wallet and
*always* restore the original balance/frozen points on teardown so
subsequent test modules are not affected.
"""

from __future__ import annotations

import time
from contextlib import contextmanager

import psycopg2.extras
import pytest

from conftest import extract_data, resolve_test_user_id


NOVEL_BASE = "/api/v1/novel"
CAMPAIGN_BASE = "/api/v1/campaigns"


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _fetch_wallet(conn, user_id: int):
    cur = conn.cursor(cursor_factory=psycopg2.extras.RealDictCursor)
    try:
        cur.execute(
            "SELECT balance_points, frozen_points FROM gm_user_wallets WHERE user_id = %s",
            (user_id,),
        )
        return cur.fetchone()
    finally:
        cur.close()


def _set_wallet(conn, user_id: int, balance, frozen):
    cur = conn.cursor()
    try:
        cur.execute(
            """
            UPDATE gm_user_wallets
            SET balance_points = %s,
                frozen_points = %s,
                updated_at = NOW()
            WHERE user_id = %s
            """,
            (balance, frozen, user_id),
        )
        conn.commit()
    finally:
        cur.close()


@contextmanager
def zero_balance(db_connection, user_id: int):
    """Temporarily drain the user's wallet balance / frozen points.

    Restores the original values on exit, even when the test body fails.
    A wallet row is guaranteed to exist for the primary test user; if
    missing, one is created so the rest of the flow is still exercised.
    """
    existing = _fetch_wallet(db_connection, user_id)
    if existing is None:
        cur = db_connection.cursor()
        try:
            cur.execute(
                """
                INSERT INTO gm_user_wallets (user_id, balance_points, frozen_points, created_at, updated_at)
                VALUES (%s, 0, 0, NOW(), NOW())
                ON CONFLICT (user_id) DO NOTHING
                """,
                (user_id,),
            )
            db_connection.commit()
        finally:
            cur.close()
        original = {"balance_points": 0, "frozen_points": 0}
    else:
        original = dict(existing)

    _set_wallet(db_connection, user_id, 0, 0)
    try:
        yield
    finally:
        _set_wallet(
            db_connection,
            user_id,
            original["balance_points"],
            original["frozen_points"],
        )


def _assert_insufficient_balance(resp):
    """Assert the response matches the unified 4100 / HTTP 402 contract."""
    assert resp.status_code == 402, (
        f"Expected HTTP 402 PAYMENT_REQUIRED, got {resp.status_code}: {resp.text}"
    )
    body = resp.json()
    assert body.get("code") == 4100, f"Expected code 4100, got {body!r}"
    assert body.get("msg"), "msg must be populated for i18n fallback"
    assert body.get("msg_cn"), "msg_cn must be populated for zh UI"


def _get_config_ids(api_client):
    """Fetch a valid (platform_id, region_id, ai_model_id) triple."""
    platforms = extract_data(api_client.get("/api/v1/config/platforms").json())
    platform_id = platforms[0]["id"]
    regions = extract_data(
        api_client.get(f"/api/v1/config/platforms/{platform_id}/regions").json()
    )
    region_id = regions[0]["id"]
    models = extract_data(api_client.get("/api/v1/config/ai-models").json())
    ai_model_id = models[0]["id"]
    return platform_id, region_id, ai_model_id


def _valid_novel_body(suffix: str = ""):
    return {
        "title": f"Novel InsufBalance {suffix or int(time.time())}",
        "topic": "A test topic for insufficient balance",
        "genre": "科幻",
        "description": "Insufficient balance guard test",
        "num_chapters": 3,
        "target_words_per_chapter": 1000,
        "default_user_guidance": "Keep it short.",
        # The handler's `Json<NovelProjectCreateRequest>` extractor parses the
        # body before the balance gate runs, so the payload must satisfy DTO
        # deserialization or we'd see 422 instead of the unified 402/4100.
        "config_snapshot": {
            "proxy_setting": {},
            "webdav_config": {},
            "other_params": {},
        },
    }


def _valid_campaign_payload(platform_id: int, region_id: int, ai_model_id: int, **overrides):
    """Campaign payload that passes DTO + service-level input validation.

    ``max_scan_count`` must be > 0 (see ``campaign_service::create_campaign``).
    Any fields consumed before the balance gate must be supplied so the gate
    is the first rejection reason, not argument validation.
    """
    payload = {
        "name": f"InsufBalance Campaign {int(time.time() * 1000)}",
        "platform_id": platform_id,
        "region_id": region_id,
        "ai_model_id": ai_model_id,
        "schedule_type": "ONCE",
        "product_prompt": "Insufficient balance guard",
        "max_scan_count": 10,
    }
    payload.update(overrides)
    return payload


# ---------------------------------------------------------------------------
# Novel — create + generate_* must all enforce the balance threshold
# ---------------------------------------------------------------------------


class TestNovelInsufficientBalance:
    def test_create_project_returns_4100_when_balance_is_zero(
        self, auth_client, db_connection
    ):
        user_id = resolve_test_user_id(db_connection)
        with zero_balance(db_connection, user_id):
            resp = auth_client.post(
                f"{NOVEL_BASE}/projects",
                json=_valid_novel_body("create"),
            )
            _assert_insufficient_balance(resp)

    def test_create_project_succeeds_after_topup(
        self, auth_client, db_connection
    ):
        """Happy-path regression: with enough balance, create must pass (status
        201) — guards against the balance gate from rejecting legitimate
        users."""
        user_id = resolve_test_user_id(db_connection)
        existing = _fetch_wallet(db_connection, user_id)
        if existing is None or int(existing["balance_points"]) < 100:
            pytest.skip(
                "Test user wallet has <100 points; top up before running"
                " positive-path balance assertion."
            )

        resp = auth_client.post(
            f"{NOVEL_BASE}/projects",
            json=_valid_novel_body("happy"),
        )
        assert resp.status_code == 201, (
            f"Expected 201 with sufficient balance, got {resp.status_code}: {resp.text}"
        )
        data = extract_data(resp.json())
        assert data.get("project_id")

    def test_generate_architecture_returns_4100_when_balance_is_zero(
        self, auth_client, db_connection
    ):
        """Even after a project exists, subsequent generate_* endpoints must
        re-check balance and short-circuit with the unified 4100 response."""
        user_id = resolve_test_user_id(db_connection)

        create_resp = auth_client.post(
            f"{NOVEL_BASE}/projects",
            json=_valid_novel_body("gen-arch"),
        )
        assert create_resp.status_code == 201, (
            f"Pre-setup create failed: {create_resp.status_code} {create_resp.text}"
        )
        project_id = extract_data(create_resp.json())["project_id"]

        # generate_architecture requires config_snapshot_id (see
        # NovelArchitectureGenerateRequest). Fetch the current snapshot so the
        # body deserializes and the balance gate is the first rejection.
        detail_resp = auth_client.get(f"{NOVEL_BASE}/projects/{project_id}")
        assert detail_resp.status_code == 200, (
            f"Project detail fetch failed: {detail_resp.status_code} {detail_resp.text}"
        )
        snapshot_id = extract_data(detail_resp.json())["current_config_snapshot"]["id"]

        with zero_balance(db_connection, user_id):
            resp = auth_client.post(
                f"{NOVEL_BASE}/projects/{project_id}/architecture/generate",
                json={"config_snapshot_id": snapshot_id},
            )
            _assert_insufficient_balance(resp)


# ---------------------------------------------------------------------------
# Campaign — create must enforce max(min_cost, budget_cap) balance gate
# ---------------------------------------------------------------------------


class TestCampaignInsufficientBalance:
    def test_create_campaign_returns_4100_when_balance_is_zero(
        self, auth_client, api_client, db_connection
    ):
        platform_id, region_id, ai_model_id = _get_config_ids(api_client)
        user_id = resolve_test_user_id(db_connection)

        payload = _valid_campaign_payload(platform_id, region_id, ai_model_id)

        with zero_balance(db_connection, user_id):
            resp = auth_client.post(CAMPAIGN_BASE, json=payload)
            _assert_insufficient_balance(resp)

    def test_create_campaign_returns_4100_when_budget_exceeds_balance(
        self, auth_client, api_client, db_connection
    ):
        """When ``budget_cap`` is provided and exceeds available balance, the
        gate must trigger even if min_cost is otherwise affordable."""
        platform_id, region_id, ai_model_id = _get_config_ids(api_client)
        user_id = resolve_test_user_id(db_connection)

        existing = _fetch_wallet(db_connection, user_id)
        original = (
            dict(existing) if existing else {"balance_points": 0, "frozen_points": 0}
        )

        # Budget massively beyond any reasonable wallet balance.
        payload = _valid_campaign_payload(
            platform_id,
            region_id,
            ai_model_id,
            name=f"HighBudget Campaign {int(time.time() * 1000)}",
            product_prompt="High budget balance gate",
            budget_cap="99999999.00",
        )

        # Force a small but positive wallet so we know the gate is failing
        # due to budget_cap, not due to zero balance.
        _set_wallet(db_connection, user_id, 500, 0)
        try:
            resp = auth_client.post(CAMPAIGN_BASE, json=payload)
            _assert_insufficient_balance(resp)
        finally:
            _set_wallet(
                db_connection,
                user_id,
                original["balance_points"],
                original["frozen_points"],
            )

    def test_create_campaign_succeeds_with_sufficient_balance(
        self, auth_client, api_client, db_connection
    ):
        """Happy-path regression: legitimate create is not blocked by the
        new balance gate."""
        user_id = resolve_test_user_id(db_connection)
        existing = _fetch_wallet(db_connection, user_id)
        if existing is None or int(existing["balance_points"]) < 100:
            pytest.skip(
                "Test user wallet has <100 points; top up before running"
                " positive-path balance assertion."
            )

        platform_id, region_id, ai_model_id = _get_config_ids(api_client)
        payload = _valid_campaign_payload(
            platform_id,
            region_id,
            ai_model_id,
            name=f"Happy Campaign {int(time.time() * 1000)}",
            product_prompt="Happy path",
        )
        resp = auth_client.post(CAMPAIGN_BASE, json=payload)
        assert resp.status_code in (200, 201), (
            f"Expected create to succeed, got {resp.status_code}: {resp.text}"
        )
        data = extract_data(resp.json())
        assert data.get("id")
