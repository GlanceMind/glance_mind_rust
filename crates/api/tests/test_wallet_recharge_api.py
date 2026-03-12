"""
GlanceMind API E2E Tests - XunhuPay Recharge Flow
===================================================
Deep tests for the recharge lifecycle: create order, payment callback,
status query, idempotency, signature verification, and DB consistency.

Test Coverage:
 1. Route existence (no-auth smoke)
 2. Create recharge order — success, validation errors
 3. Payment callback (notify) — signature verify, OD/FAILED/repeat
 4. Recharge status query — owner check, status mapping
 5. DB consistency — payment_status, platform_txn_id, balance, deposit txn
 6. Concurrent callback idempotency
 7. Negative paths — bad signature, unknown order, non-OD status

Run: pytest tests/test_wallet_recharge_api.py -v --tb=short
"""

import hashlib
import os
import time
import uuid
import threading
from decimal import Decimal

import pytest

try:
    from conftest import (
        API_BASE_URL,
        TEST_USER_ID,
        assert_response_success,
        extract_data,
    )
except ImportError:
    API_BASE_URL = "http://localhost:8081"
    TEST_USER_ID = 999


# ============================================================================
# Helpers
# ============================================================================

XUNHUPAY_APP_SECRET = os.getenv("XUNHUPAY_APP_SECRET", "mock-xunhupay-secret")
XUNHUPAY_APP_ID = os.getenv("XUNHUPAY_APP_ID", "mock-app-id")


def generate_xunhupay_hash(params: dict, app_secret: str) -> str:
    """Replicate the XunhuPay signing algorithm for test callbacks."""
    filtered = {
        k: v for k, v in sorted(params.items())
        if k != "hash" and v is not None and v != ""
    }
    query = "&".join(f"{k}={v}" for k, v in filtered.items())
    raw = f"{query}{app_secret}"
    return hashlib.md5(raw.encode()).hexdigest()


def build_notify_form(
    trade_order_id: str,
    total_fee: str = "9.90",
    status: str = "OD",
    app_secret: str = XUNHUPAY_APP_SECRET,
    appid: str = XUNHUPAY_APP_ID,
    extra_fields: dict = None,
) -> dict:
    """Build a form-encoded XunhuPay callback payload with correct signature."""
    params = {
        "trade_order_id": trade_order_id,
        "total_fee": total_fee,
        "transaction_id": f"TXN_{uuid.uuid4().hex[:16]}",
        "open_order_id": f"OPEN_{uuid.uuid4().hex[:12]}",
        "order_title": "GlanceMind Points Recharge",
        "status": status,
        "appid": appid,
        "time": str(int(time.time())),
        "nonce_str": uuid.uuid4().hex[:16],
    }
    if extra_fields:
        params.update(extra_fields)
    params["hash"] = generate_xunhupay_hash(params, app_secret)
    return params


def reset_wallet(db_cursor, user_id=TEST_USER_ID, balance=500.0):
    """Reset wallet to a known state for recharge tests."""
    try:
        db_cursor.connection.rollback()
    except Exception:
        pass
    db_cursor.execute("""
        INSERT INTO gm_user_wallets (user_id, balance_points, frozen_points, created_at)
        VALUES (%s, %s, 0, NOW())
        ON CONFLICT (user_id) DO UPDATE
        SET balance_points = EXCLUDED.balance_points,
            frozen_points = 0,
            updated_at = NOW()
    """, (user_id, balance))
    db_cursor.connection.commit()


def cleanup_recharge_txns(db_cursor, user_id=TEST_USER_ID):
    """Remove test recharge/deposit transactions."""
    try:
        db_cursor.connection.rollback()
    except Exception:
        pass
    db_cursor.execute("""
        DELETE FROM gm_wallet_transactions
        WHERE user_id = %s AND type IN ('RECHARGE', 'DEPOSIT')
          AND external_txn_id LIKE 'E2E_%%'
    """, (user_id,))
    db_cursor.connection.commit()


def insert_pending_order(db_cursor, order_no: str, user_id=TEST_USER_ID,
                         amount="9.90", channel="wechat"):
    """Directly insert a PENDING recharge order for callback testing."""
    db_cursor.execute("""
        INSERT INTO gm_wallet_transactions
            (user_id, amount, type, payment_method, external_txn_id,
             description, payment_status, created_at)
        VALUES (%s, %s, 'RECHARGE', %s, %s,
                'E2E test order', 'PENDING', NOW())
        RETURNING id
    """, (user_id, amount, channel, order_no))
    row = db_cursor.fetchone()
    db_cursor.connection.commit()
    return row["id"]


def get_wallet_balance(db_cursor, user_id=TEST_USER_ID):
    """Return (balance_points, frozen_points) as Decimals."""
    db_cursor.execute("""
        SELECT balance_points, frozen_points
        FROM gm_user_wallets WHERE user_id = %s
    """, (user_id,))
    row = db_cursor.fetchone()
    assert row is not None, f"Wallet not found for user {user_id}"
    return row["balance_points"], row["frozen_points"]


def get_txn_by_external_id(db_cursor, external_id: str):
    """Fetch a single wallet_transaction row by external_txn_id."""
    db_cursor.execute("""
        SELECT * FROM gm_wallet_transactions
        WHERE external_txn_id = %s
    """, (external_id,))
    return db_cursor.fetchone()


def count_deposit_txns(db_cursor, reference_id: int):
    """Count DEPOSIT transactions linked to a RECHARGE order."""
    db_cursor.execute("""
        SELECT COUNT(*) as cnt FROM gm_wallet_transactions
        WHERE type = 'DEPOSIT' AND reference_id = %s
    """, (reference_id,))
    return db_cursor.fetchone()["cnt"]


def count_refund_txns(db_cursor, reference_id: int):
    """Count REFUND transactions linked to a RECHARGE order."""
    db_cursor.execute("""
        SELECT COUNT(*) as cnt FROM gm_wallet_transactions
        WHERE type = 'REFUND' AND reference_id = %s
    """, (reference_id,))
    return db_cursor.fetchone()["cnt"]


# ============================================================================
# 1. Route existence (no-auth smoke)
# ============================================================================

class TestRechargeRouteExists:
    """Verify new recharge routes are registered and protected."""

    def test_recharge_create_route_requires_auth(self, api_client):
        resp = api_client.post("/api/v1/wallet/recharge", json={
            "amount": "1.00", "channel": "wechat"
        })
        assert resp.status_code == 401, \
            f"Expected 401 (needs auth), got {resp.status_code}"

    def test_recharge_status_route_requires_auth(self, api_client):
        resp = api_client.get("/api/v1/wallet/recharge/FAKE_ORDER_NO")
        assert resp.status_code == 401, \
            f"Expected 401 (needs auth), got {resp.status_code}"

    def test_payment_notify_route_is_public(self, api_client):
        """Notify endpoint must be reachable without JWT (XunhuPay POSTs to it)."""
        resp = api_client.post("/api/v1/public/payment/notify", data={
            "trade_order_id": "nonexistent"
        })
        # Should NOT be 404 (route missing) or 401 (auth required)
        assert resp.status_code != 404, "Notify route not found"
        assert resp.status_code != 401, "Notify route should not require auth"


# ============================================================================
# 2. Create recharge order — validation
# ============================================================================

class TestCreateRechargeValidation:
    """Input validation for POST /wallet/recharge."""

    def test_invalid_channel_rejected(self, auth_client):
        resp = auth_client.post("/api/v1/wallet/recharge", json={
            "amount": "10.00", "channel": "bitcoin"
        })
        assert resp.status_code in [400, 422, 500], \
            f"Expected 4xx/5xx for bad channel, got {resp.status_code}"

    def test_zero_amount_rejected(self, auth_client):
        resp = auth_client.post("/api/v1/wallet/recharge", json={
            "amount": "0.00", "channel": "wechat"
        })
        assert resp.status_code in [400, 422, 500], \
            f"Expected error for zero amount, got {resp.status_code}"

    def test_negative_amount_rejected(self, auth_client):
        resp = auth_client.post("/api/v1/wallet/recharge", json={
            "amount": "-5.00", "channel": "alipay"
        })
        assert resp.status_code in [400, 422, 500], \
            f"Expected error for negative amount, got {resp.status_code}"

    def test_invalid_amount_format_rejected(self, auth_client):
        resp = auth_client.post("/api/v1/wallet/recharge", json={
            "amount": "not_a_number", "channel": "wechat"
        })
        assert resp.status_code in [400, 422, 500], \
            f"Expected error for non-numeric amount, got {resp.status_code}"


# ============================================================================
# 3. Payment callback — core flow
# ============================================================================

class TestPaymentNotifyCallback:
    """Tests for POST /public/payment/notify (XunhuPay async callback)."""

    def test_notify_success_credits_wallet(self, api_client, db_cursor):
        """OD callback should mark order PAID and credit wallet exactly once."""
        order_no = f"E2E_{uuid.uuid4().hex[:20]}"
        reset_wallet(db_cursor, balance=100.0)
        txn_id = insert_pending_order(db_cursor, order_no, amount="9.90")

        balance_before, _ = get_wallet_balance(db_cursor)

        form = build_notify_form(order_no, total_fee="9.90", status="OD")
        resp = api_client.post(
            "/api/v1/public/payment/notify", data=form
        )
        assert resp.text.strip() == "success", \
            f"Callback must return 'success', got: {resp.text}"

        # Verify DB state
        txn = get_txn_by_external_id(db_cursor, order_no)
        assert txn is not None, "Transaction should exist"
        assert txn["payment_status"] == "PAID", \
            f"Expected PAID, got {txn['payment_status']}"
        assert txn["platform_txn_id"] is not None, \
            "platform_txn_id should be persisted"
        assert txn["open_order_id"] is not None, \
            "open_order_id should be persisted"
        assert txn["paid_at"] is not None, \
            "paid_at should be set"

        # 1 CNY = 100 points => 9.90 CNY = 990 points
        balance_after, _ = get_wallet_balance(db_cursor)
        credited = balance_after - balance_before
        assert credited == Decimal("990.00") or credited == Decimal("990"), \
            f"Expected 990 points credited, got {credited}"

        # Should have exactly 1 DEPOSIT transaction linked to the RECHARGE
        assert count_deposit_txns(db_cursor, txn_id) == 1, \
            "Should have exactly 1 DEPOSIT record"

        # Cleanup
        cleanup_recharge_txns(db_cursor)

    def test_notify_response_body_format(self, api_client, db_cursor):
        """A successful notify must return the exact plain-text body expected by XunhuPay."""
        order_no = f"E2E_{uuid.uuid4().hex[:20]}"
        insert_pending_order(db_cursor, order_no, amount="3.00")

        form = build_notify_form(order_no, total_fee="3.00", status="OD")
        resp = api_client.post("/api/v1/public/payment/notify", data=form)

        assert resp.status_code == 200
        assert resp.text.strip() == "success"

        cleanup_recharge_txns(db_cursor)

    def test_notify_idempotent_duplicate(self, api_client, db_cursor):
        """Sending the same OD callback twice must credit only once."""
        order_no = f"E2E_{uuid.uuid4().hex[:20]}"
        reset_wallet(db_cursor, balance=100.0)
        insert_pending_order(db_cursor, order_no, amount="5.00")

        form = build_notify_form(order_no, total_fee="5.00", status="OD")

        # First callback
        resp1 = api_client.post("/api/v1/public/payment/notify", data=form)
        assert resp1.text.strip() == "success"

        balance_after_first, _ = get_wallet_balance(db_cursor)

        # Second callback (duplicate)
        resp2 = api_client.post("/api/v1/public/payment/notify", data=form)
        assert resp2.text.strip() == "success", \
            "Duplicate callback should still return success"

        balance_after_second, _ = get_wallet_balance(db_cursor)
        assert balance_after_second == balance_after_first, \
            "Duplicate callback must NOT credit again"

        cleanup_recharge_txns(db_cursor)

    def test_notify_bad_signature_rejected(self, api_client, db_cursor):
        """Callback with wrong hash must be rejected."""
        order_no = f"E2E_{uuid.uuid4().hex[:20]}"
        reset_wallet(db_cursor, balance=100.0)
        insert_pending_order(db_cursor, order_no, amount="1.00")

        form = build_notify_form(order_no, total_fee="1.00", status="OD")
        form["hash"] = "00000000000000000000000000000000"  # wrong hash

        resp = api_client.post("/api/v1/public/payment/notify", data=form)
        # Should NOT return "success"
        assert resp.text.strip() != "success", \
            "Bad signature callback must not return success"

        # Balance must not change
        txn = get_txn_by_external_id(db_cursor, order_no)
        assert txn["payment_status"] == "PENDING", \
            "Order should remain PENDING after bad-signature callback"

        cleanup_recharge_txns(db_cursor)

    def test_notify_unknown_order_rejected(self, api_client, db_cursor):
        """Callback for a nonexistent order should fail gracefully."""
        form = build_notify_form("E2E_DOES_NOT_EXIST_12345", status="OD")
        resp = api_client.post("/api/v1/public/payment/notify", data=form)
        assert resp.text.strip() != "success" or resp.status_code != 200, \
            "Unknown order should not return success"

    def test_notify_refunded_status_does_not_credit_wallet(self, api_client, db_cursor):
        """Refund-complete callback should mark order REFUNDED, not credit."""
        order_no = f"E2E_{uuid.uuid4().hex[:20]}"
        reset_wallet(db_cursor, balance=200.0)
        insert_pending_order(db_cursor, order_no, amount="10.00")

        balance_before, _ = get_wallet_balance(db_cursor)

        form = build_notify_form(order_no, total_fee="10.00", status="CD")
        resp = api_client.post("/api/v1/public/payment/notify", data=form)
        # Platform might still expect "success" to stop retrying
        # But balance must NOT change
        balance_after, _ = get_wallet_balance(db_cursor)
        assert balance_after == balance_before, \
            "Refunded status must NOT credit wallet"

        txn = get_txn_by_external_id(db_cursor, order_no)
        assert txn["payment_status"] == "REFUNDED", \
            f"Expected REFUNDED, got {txn['payment_status']}"
        assert count_deposit_txns(db_cursor, txn["id"]) == 0, \
            "Refunded-before-credit order must not create a DEPOSIT record"

        cleanup_recharge_txns(db_cursor)

    def test_notify_refund_failed_restores_paid_state_without_double_credit(self, api_client, db_cursor):
        """Refund failure after REFUNDING should return order to PAID without re-crediting."""
        order_no = f"E2E_{uuid.uuid4().hex[:20]}"
        reset_wallet(db_cursor, balance=200.0)
        insert_pending_order(db_cursor, order_no, amount="10.00")

        paid_form = build_notify_form(order_no, total_fee="10.00", status="OD")
        api_client.post("/api/v1/public/payment/notify", data=paid_form)

        balance_after_paid, _ = get_wallet_balance(db_cursor)
        txn = get_txn_by_external_id(db_cursor, order_no)
        assert txn["payment_status"] == "PAID"
        assert count_deposit_txns(db_cursor, txn["id"]) == 1

        refunding_form = build_notify_form(order_no, total_fee="10.00", status="RD")
        api_client.post("/api/v1/public/payment/notify", data=refunding_form)

        balance_after_refunding, _ = get_wallet_balance(db_cursor)
        assert balance_after_refunding == balance_after_paid, \
            "REFUNDING transition must not change wallet balance"
        txn = get_txn_by_external_id(db_cursor, order_no)
        assert txn["payment_status"] == "REFUNDING"

        refund_failed_form = build_notify_form(order_no, total_fee="10.00", status="UD")
        api_client.post("/api/v1/public/payment/notify", data=refund_failed_form)

        balance_after_refund_failed, _ = get_wallet_balance(db_cursor)
        assert balance_after_refund_failed == balance_after_paid, \
            "Refund failure must not credit wallet twice"
        txn = get_txn_by_external_id(db_cursor, order_no)
        assert txn["payment_status"] == "PAID"
        assert count_deposit_txns(db_cursor, txn["id"]) == 1, \
            "Refund failure must preserve exactly one DEPOSIT record"

        cleanup_recharge_txns(db_cursor)

    def test_notify_paid_then_refunded_reverses_wallet_credit(self, api_client, db_cursor):
        """A refund after successful payment must debit the previously credited points."""
        order_no = f"E2E_{uuid.uuid4().hex[:20]}"
        reset_wallet(db_cursor, balance=200.0)
        txn_id = insert_pending_order(db_cursor, order_no, amount="10.00")

        balance_before, _ = get_wallet_balance(db_cursor)

        api_client.post(
            "/api/v1/public/payment/notify",
            data=build_notify_form(order_no, total_fee="10.00", status="OD"),
        )

        balance_after_paid, _ = get_wallet_balance(db_cursor)
        assert balance_after_paid - balance_before == Decimal("1000") or \
            balance_after_paid - balance_before == Decimal("1000.00"), \
            "Paid callback should credit 1000 points"

        api_client.post(
            "/api/v1/public/payment/notify",
            data=build_notify_form(order_no, total_fee="10.00", status="CD"),
        )

        balance_after_refund, _ = get_wallet_balance(db_cursor)
        assert balance_after_refund == balance_before, \
            "Refunded callback must reverse the previously credited points"

        txn = get_txn_by_external_id(db_cursor, order_no)
        assert txn["payment_status"] == "REFUNDED", \
            f"Expected REFUNDED, got {txn['payment_status']}"
        assert count_deposit_txns(db_cursor, txn_id) == 1, \
            "Original DEPOSIT record should remain as audit trail"
        assert count_refund_txns(db_cursor, txn_id) == 1, \
            "Refund reversal should create exactly one REFUND record"

        cleanup_recharge_txns(db_cursor)

    def test_notify_refund_blocked_when_points_already_spent(self, api_client, db_cursor):
        """Refund should be blocked if available balance cannot cover the refund amount."""
        order_no = f"E2E_{uuid.uuid4().hex[:20]}"
        reset_wallet(db_cursor, balance=0.0)
        txn_id = insert_pending_order(db_cursor, order_no, amount="10.00")

        api_client.post(
            "/api/v1/public/payment/notify",
            data=build_notify_form(order_no, total_fee="10.00", status="OD"),
        )

        txn = get_txn_by_external_id(db_cursor, order_no)
        assert txn["payment_status"] == "PAID"
        assert count_deposit_txns(db_cursor, txn_id) == 1

        # Simulate user spending most of the credited points, leaving only 100 available.
        db_cursor.execute("""
            UPDATE gm_user_wallets
            SET balance_points = %s, frozen_points = %s, updated_at = NOW()
            WHERE user_id = %s
        """, ("100.00", "0.00", TEST_USER_ID))
        db_cursor.connection.commit()

        balance_before_refund, _ = get_wallet_balance(db_cursor)
        assert balance_before_refund == Decimal("100.00") or balance_before_refund == Decimal("100")

        resp = api_client.post(
            "/api/v1/public/payment/notify",
            data=build_notify_form(order_no, total_fee="10.00", status="CD"),
        )
        assert resp.text.strip() == "success", \
            "Blocked refund callback should still return success to stop retries"

        txn = get_txn_by_external_id(db_cursor, order_no)
        assert txn["payment_status"] == "PAID", \
            "Refund should remain blocked and order stay PAID when points are insufficient"

        balance_after_refund, _ = get_wallet_balance(db_cursor)
        assert balance_after_refund == balance_before_refund, \
            "Blocked refund must not change wallet balance"
        assert count_refund_txns(db_cursor, txn_id) == 0, \
            "Blocked refund must not create a REFUND transaction"

        cleanup_recharge_txns(db_cursor)


# ============================================================================
# 4. Recharge status query
# ============================================================================

class TestRechargeStatusQuery:
    """Tests for GET /wallet/recharge/:order_no."""

    def test_query_pending_order(self, auth_client, db_cursor):
        order_no = f"E2E_{uuid.uuid4().hex[:20]}"
        insert_pending_order(db_cursor, order_no)

        resp = auth_client.get(f"/api/v1/wallet/recharge/{order_no}")
        assert_response_success(resp)
        data = extract_data(resp.json())
        assert data["status"] == "PENDING"
        assert data["order_no"] == order_no

        cleanup_recharge_txns(db_cursor)

    def test_query_paid_order(self, auth_client, api_client, db_cursor):
        order_no = f"E2E_{uuid.uuid4().hex[:20]}"
        reset_wallet(db_cursor, balance=100.0)
        insert_pending_order(db_cursor, order_no, amount="1.00")

        form = build_notify_form(order_no, total_fee="1.00", status="OD")
        api_client.post("/api/v1/public/payment/notify", data=form)

        resp = auth_client.get(f"/api/v1/wallet/recharge/{order_no}")
        assert_response_success(resp)
        data = extract_data(resp.json())
        assert data["status"] == "PAID"
        assert data["paid_at"] is not None, "paid_at should be set for PAID orders"

        cleanup_recharge_txns(db_cursor)

    def test_query_refunding_order(self, auth_client, api_client, db_cursor):
        order_no = f"E2E_{uuid.uuid4().hex[:20]}"
        reset_wallet(db_cursor, balance=100.0)
        insert_pending_order(db_cursor, order_no, amount="1.00")

        api_client.post(
            "/api/v1/public/payment/notify",
            data=build_notify_form(order_no, total_fee="1.00", status="OD"),
        )
        api_client.post(
            "/api/v1/public/payment/notify",
            data=build_notify_form(order_no, total_fee="1.00", status="RD"),
        )

        resp = auth_client.get(f"/api/v1/wallet/recharge/{order_no}")
        assert_response_success(resp)
        data = extract_data(resp.json())
        assert data["status"] == "REFUNDING"
        assert data["paid_at"] is not None, "paid_at should stay set for REFUNDING orders"

        cleanup_recharge_txns(db_cursor)

    def test_query_refunded_order(self, auth_client, api_client, db_cursor):
        order_no = f"E2E_{uuid.uuid4().hex[:20]}"
        reset_wallet(db_cursor, balance=100.0)
        insert_pending_order(db_cursor, order_no, amount="1.00")

        api_client.post(
            "/api/v1/public/payment/notify",
            data=build_notify_form(order_no, total_fee="1.00", status="CD"),
        )

        resp = auth_client.get(f"/api/v1/wallet/recharge/{order_no}")
        assert_response_success(resp)
        data = extract_data(resp.json())
        assert data["status"] == "REFUNDED"
        assert data["paid_at"] is None, "paid_at should remain empty when order never settled"

        cleanup_recharge_txns(db_cursor)

    def test_query_nonexistent_order_returns_404(self, auth_client):
        resp = auth_client.get("/api/v1/wallet/recharge/E2E_NONEXISTENT_ORDER")
        assert resp.status_code == 404, \
            f"Expected 404 for missing order, got {resp.status_code}"


# ============================================================================
# 5. Concurrent callback safety
# ============================================================================

class TestConcurrentCallbackSafety:
    """Two threads firing the same OD callback must only credit once."""

    def test_concurrent_duplicate_callbacks(self, api_client, db_cursor):
        order_no = f"E2E_{uuid.uuid4().hex[:20]}"
        reset_wallet(db_cursor, balance=0.0)
        insert_pending_order(db_cursor, order_no, amount="10.00")

        form = build_notify_form(order_no, total_fee="10.00", status="OD")

        results = []

        def fire_callback():
            import requests as req
            resp = req.post(
                f"{API_BASE_URL}/api/v1/public/payment/notify",
                data=form,
            )
            results.append(resp.text.strip())

        t1 = threading.Thread(target=fire_callback)
        t2 = threading.Thread(target=fire_callback)
        t1.start()
        t2.start()
        t1.join(timeout=10)
        t2.join(timeout=10)

        # Both should return "success" (one real, one idempotent no-op)
        assert all(r == "success" for r in results), \
            f"Both callbacks should return success, got {results}"

        # Wallet should have exactly 1000 points (10 CNY * 100)
        balance, _ = get_wallet_balance(db_cursor)
        assert balance == Decimal("1000") or balance == Decimal("1000.00"), \
            f"Expected 1000 points (one credit), got {balance}"

        # Only 1 DEPOSIT record
        txn = get_txn_by_external_id(db_cursor, order_no)
        deposit_count = count_deposit_txns(db_cursor, txn["id"])
        assert deposit_count == 1, \
            f"Expected 1 DEPOSIT, got {deposit_count}"

        cleanup_recharge_txns(db_cursor)


# ============================================================================
# 6. DB field-level consistency
# ============================================================================

class TestDatabaseFieldConsistency:
    """Verify every field in gm_wallet_transactions after a full cycle."""

    def test_full_recharge_cycle_fields(self, api_client, db_cursor):
        """Insert PENDING → callback OD → verify all fields."""
        order_no = f"E2E_{uuid.uuid4().hex[:20]}"
        reset_wallet(db_cursor, balance=50.0)
        txn_id = insert_pending_order(
            db_cursor, order_no, amount="20.00", channel="alipay"
        )

        form = build_notify_form(order_no, total_fee="20.00", status="OD")
        api_client.post("/api/v1/public/payment/notify", data=form)

        # Reload from DB
        txn = get_txn_by_external_id(db_cursor, order_no)
        assert txn is not None

        # RECHARGE order fields
        assert txn["id"] == txn_id
        assert txn["user_id"] == TEST_USER_ID
        assert txn["amount"] == Decimal("20.00")
        assert txn["type"] == "RECHARGE"
        assert txn["payment_method"] == "alipay"
        assert txn["external_txn_id"] == order_no
        assert txn["payment_status"] == "PAID"
        assert txn["platform_txn_id"] is not None
        assert len(txn["platform_txn_id"]) > 0
        assert txn["open_order_id"] is not None
        assert len(txn["open_order_id"]) > 0
        assert txn["paid_at"] is not None
        assert txn["created_at"] is not None
        # reference_type should NOT be used for payment status
        assert txn.get("reference_type") is None or \
            txn["reference_type"] not in ("PENDING", "PAID", "FAILED"), \
            "reference_type must not store payment lifecycle status"

        # Linked DEPOSIT record
        db_cursor.execute("""
            SELECT * FROM gm_wallet_transactions
            WHERE type = 'DEPOSIT' AND reference_id = %s
        """, (txn_id,))
        deposit = db_cursor.fetchone()
        assert deposit is not None, "DEPOSIT record must exist"
        assert deposit["amount"] == Decimal("2000.00") or \
            deposit["amount"] == Decimal("2000"), \
            f"DEPOSIT amount should be 2000 points, got {deposit['amount']}"
        assert deposit["user_id"] == TEST_USER_ID
        assert deposit["reference_type"] == "recharge"

        # Wallet balance: 50 + 2000 = 2050
        balance, frozen = get_wallet_balance(db_cursor)
        assert balance == Decimal("2050") or balance == Decimal("2050.00"), \
            f"Expected balance 2050, got {balance}"
        assert frozen == Decimal("0") or frozen == Decimal("0.00"), \
            f"Frozen should be 0, got {frozen}"

        cleanup_recharge_txns(db_cursor)


# ============================================================================
# 7. Signature algorithm edge cases
# ============================================================================

class TestRechargeOwnership:
    """Ensure recharge orders are isolated per user."""

    def test_query_other_users_order_returns_404(self, auth_client, db_cursor):
        """A user must not be able to query another user's recharge order."""
        other_user_id = TEST_USER_ID + 100
        order_no = f"E2E_{uuid.uuid4().hex[:20]}"

        db_cursor.execute("""
            INSERT INTO gm_users
                (id, email, username, password_hash, full_name, role, status, is_active, invite_code)
            VALUES (%s, %s, %s,
                    '$2b$12$Ikc.R4FMMGahbGhfHlLl4.PciMiV37qXfHpPNCjGGQg/yOEgk7k/e',
                    'Other Recharge User', 'user', 'ACTIVE', true, 'OTHER0999')
            ON CONFLICT (id) DO NOTHING
        """, (other_user_id, f"other_{other_user_id}@glancemind.test", f"other_{other_user_id}"))

        db_cursor.execute("""
            INSERT INTO gm_wallet_transactions
                (user_id, amount, type, payment_method, external_txn_id,
                 description, payment_status, created_at)
            VALUES (%s, %s, 'RECHARGE', 'wechat', %s,
                    'Other user order', 'PENDING', NOW())
        """, (other_user_id, "10.00", order_no))
        db_cursor.connection.commit()

        resp = auth_client.get(f"/api/v1/wallet/recharge/{order_no}")
        assert resp.status_code == 404, \
            f"Should not expose other user's order, got {resp.status_code}"

        db_cursor.execute(
            "DELETE FROM gm_wallet_transactions WHERE external_txn_id = %s",
            (order_no,),
        )
        db_cursor.connection.commit()

    def test_recharge_db_fields_complete_on_create(self, auth_client, db_cursor):
        """After a PENDING insert the DB row has all expected fields set."""
        order_no = f"E2E_{uuid.uuid4().hex[:20]}"
        insert_pending_order(db_cursor, order_no, amount="15.50", channel="alipay")

        txn = get_txn_by_external_id(db_cursor, order_no)
        assert txn is not None, "Transaction should exist"
        assert txn["type"] == "RECHARGE"
        assert txn["payment_method"] == "alipay"
        assert txn["external_txn_id"] == order_no
        assert txn["payment_status"] == "PENDING"
        assert txn["platform_txn_id"] is None
        assert txn["open_order_id"] is None
        assert txn["paid_at"] is None
        assert txn["created_at"] is not None
        assert txn["amount"] == Decimal("15.50")
        assert txn["user_id"] == TEST_USER_ID

        cleanup_recharge_txns(db_cursor)

    def test_create_recharge_without_gateway_returns_error(self, auth_client):
        """If XUNHUPAY_* env vars are not set, create recharge should fail gracefully."""
        resp = auth_client.post("/api/v1/wallet/recharge", json={
            "amount": "10.00", "channel": "wechat"
        })
        # Without XUNHUPAY_APP_ID/APP_SECRET the service returns 500
        if resp.status_code == 200:
            # If env vars are set (e.g., pointed to a mock), this is fine
            data = extract_data(resp.json())
            assert "order_no" in data, "Success response must include order_no"
        else:
            assert resp.status_code in [400, 500], \
                f"Expected graceful error, got {resp.status_code}"


class TestSignatureEdgeCases:
    """Pure-logic tests for the hash generation used in test helpers,
    mirroring the Rust SDK unit tests to ensure test parity."""

    def test_hash_ignores_empty_values(self):
        base = {"appid": "X", "total_fee": "1"}
        with_empty = {**base, "return_url": ""}
        assert generate_xunhupay_hash(base, "S") == \
            generate_xunhupay_hash(with_empty, "S")

    def test_hash_ignores_hash_field(self):
        params = {"appid": "X", "total_fee": "1"}
        with_hash = {**params, "hash": "should_be_ignored"}
        assert generate_xunhupay_hash(params, "S") == \
            generate_xunhupay_hash(with_hash, "S")

    def test_hash_is_32_char_lowercase_hex(self):
        h = generate_xunhupay_hash({"appid": "X"}, "secret")
        assert len(h) == 32
        assert h == h.lower()
        assert all(c in "0123456789abcdef" for c in h)

    def test_hash_includes_extension_fields(self):
        base = {"appid": "X", "total_fee": "1"}
        extended = {**base, "future_field": "val"}
        assert generate_xunhupay_hash(base, "S") != \
            generate_xunhupay_hash(extended, "S")


if __name__ == "__main__":
    pytest.main([__file__, "-v", "--tb=short"])
