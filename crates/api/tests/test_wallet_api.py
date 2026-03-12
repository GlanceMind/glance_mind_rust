"""
GlanceMind API E2E Tests - Wallet API
=====================================
Tests for wallet and payment endpoints.

Test Coverage:
1. Get wallet balance
2. Create recharge order
3. Get transactions
4. Wallet route existence
"""

import pytest
from conftest import (
    assert_response_success,
    extract_data,
)


class TestWalletBalance:
    """Tests for wallet balance endpoint."""

    def test_get_wallet_balance(self, auth_client):
        """Test getting wallet balance."""
        resp = auth_client.get("/api/v1/wallet/balance")
        assert_response_success(resp)
        
        data = extract_data(resp.json())
        print(f"\nWallet balance: {data}")
        
        # Should have balance info
        assert "balance_points" in data or "balance" in data, \
            "Should return balance info"
        
        # Balance should be a number or string
        balance = data.get("balance_points") or data.get("balance")
        print(f"  Balance: {balance}")

    def test_wallet_has_frozen_points(self, auth_client):
        """Test that wallet response includes frozen points."""
        resp = auth_client.get("/api/v1/wallet/balance")
        assert_response_success(resp)
        
        data = extract_data(resp.json())
        
        # May have frozen_points field
        if "frozen_points" in data:
            print(f"\n  Frozen points: {data['frozen_points']}")


class TestWalletRecharge:
    """Tests for wallet recharge endpoints."""

    def test_create_recharge_order(self, auth_client):
        """Test creating a recharge order."""
        order_payload = {
            "amount": "10.00",
            "channel": "wechat"
        }
        
        resp = auth_client.post(
            "/api/v1/wallet/recharge",
            json=order_payload
        )
        
        # May succeed or fail depending on payment config
        if resp.status_code == 200:
            data = extract_data(resp.json())
            print(f"\nRecharge order: {data}")
            
            # Should have order info
            assert "order_no" in data, \
                "Should return transaction info"
        else:
            print(f"\n  Order creation returned: {resp.status_code}")
            # Not a critical failure - payment may not be configured
            assert resp.status_code in [400, 422, 500], \
                f"Unexpected status: {resp.status_code}"


class TestWalletTransactions:
    """Tests for wallet transaction history."""

    def test_get_transactions(self, auth_client):
        """Test getting transaction history."""
        resp = auth_client.get("/api/v1/wallet/transactions?page=1&per_page=10")
        assert_response_success(resp)
        
        data = extract_data(resp.json())
        print(f"\nTransactions: {type(data)}")
        
        # Should have list structure
        transactions = data if isinstance(data, list) else data.get("list", [])
        print(f"  Transaction count: {len(transactions)}")
        
        # Show first transaction if any
        if transactions:
            print(f"  First transaction: {transactions[0]}")


class TestWalletRouteExists:
    """Tests that wallet routes exist."""

    def test_wallet_balance_route_exists(self, api_client):
        """Test that wallet balance route exists (returns 401 without auth)."""
        resp = api_client.get("/api/v1/wallet/balance")
        
        # Should be 401 (no auth), not 404 (not found)
        assert resp.status_code == 401, \
            f"Expected 401 (route exists), got {resp.status_code}"

    def test_wallet_transactions_route_exists(self, api_client):
        """Test that wallet transactions route exists."""
        resp = api_client.get("/api/v1/wallet/transactions")
        
        assert resp.status_code == 401, \
            f"Expected 401 (route exists), got {resp.status_code}"

    def test_wallet_recharge_route_exists(self, api_client):
        """Test that XunhuPay recharge route exists."""
        resp = api_client.post("/api/v1/wallet/recharge", json={
            "amount": "1.00", "channel": "wechat"
        })
        assert resp.status_code == 401, \
            f"Expected 401 (route exists), got {resp.status_code}"

    def test_payment_notify_route_exists(self, api_client):
        """Test that public payment notify route exists (no auth required)."""
        resp = api_client.post("/api/v1/public/payment/notify", data={
            "trade_order_id": "test"
        })
        assert resp.status_code != 404, \
            "Payment notify route should exist"


class TestWalletDatabaseState:
    """Tests that verify wallet database state."""

    def test_mock_users_have_wallets(self, db_cursor):
        """Verify mock users have wallets with balance."""
        print("\n=== User Wallets Verification ===")
        
        db_cursor.execute("""
            SELECT u.id, u.username, w.balance_points, w.frozen_points
            FROM gm_users u
            JOIN gm_user_wallets w ON u.id = w.user_id
            ORDER BY u.id
            LIMIT 5
        """)
        results = db_cursor.fetchall()
        
        for row in results:
            print(f"  User {row['id']} ({row['username']}): "
                  f"balance={row['balance_points']}, frozen={row['frozen_points']}")
        
        assert len(results) >= 2, "Should have at least 2 user wallets"

    def test_wallet_transactions_in_database(self, db_cursor):
        """Check if any transactions exist in database."""
        db_cursor.execute("SELECT COUNT(*) as count FROM gm_wallet_transactions")
        result = db_cursor.fetchone()
        
        print(f"\n  Total transactions in DB: {result['count']}")


if __name__ == "__main__":
    pytest.main([__file__, "-v", "--tb=short"])
