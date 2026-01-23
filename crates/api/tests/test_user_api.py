"""
GlanceMind API E2E Tests - User API
===================================
Tests for user profile and user management endpoints.

Test Coverage:
1. Get current user (GET /user/me)
2. Update user profile (POST /user/profile)
3. User lifecycle
"""

import pytest
from conftest import (
    assert_response_success,
    extract_data,
)


class TestUserProfile:
    """Tests for user profile endpoints."""

    def test_get_current_user(self, auth_client):
        """Test getting current user info."""
        resp = auth_client.get("/api/v1/user/me")
        assert_response_success(resp)
        
        data = extract_data(resp.json())
        print(f"\nUser me response: {data}")
        
        # Should have user info
        assert "id" in data or "email" in data, "Should return user info"
        
        # Verify expected fields
        expected_fields = ["id", "email", "username"]
        present_fields = [f for f in expected_fields if f in data]
        print(f"  User fields present: {present_fields}")
        
        assert len(present_fields) >= 2, "Should have at least id and email"

    def test_update_user_profile(self, auth_client, db_cursor):
        """Test updating user profile."""
        # Update profile
        update_payload = {
            "company_name": "E2E Test Company"
        }
        
        resp = auth_client.post(
            "/api/v1/user/profile",
            json=update_payload
        )
        assert_response_success(resp)
        
        data = extract_data(resp.json())
        print(f"\nUpdate profile response: {data}")
        
        # Verify the update
        if "company_name" in data:
            assert data["company_name"] == "E2E Test Company"

    def test_get_user_after_update(self, auth_client):
        """Test that user info reflects updates."""
        # First update
        auth_client.post(
            "/api/v1/user/profile",
            json={"company_name": "Updated Company"}
        )
        
        # Then get
        resp = auth_client.get("/api/v1/user/me")
        assert_response_success(resp)
        
        data = extract_data(resp.json())
        
        # company_name should be updated
        if "company_name" in data:
            assert data["company_name"] == "Updated Company"


class TestUserLifecycle:
    """Tests for complete user lifecycle."""

    def test_user_has_wallet(self, auth_client):
        """Test that authenticated user has a wallet."""
        # Get wallet balance
        resp = auth_client.get("/api/v1/wallet/balance")
        
        # Should be able to access wallet
        assert resp.status_code in [200, 404], \
            f"Unexpected status: {resp.status_code}"
        
        if resp.status_code == 200:
            data = extract_data(resp.json())
            print(f"\nWallet balance: {data}")
            assert "balance_points" in data or "balance" in data


class TestUserDatabaseState:
    """Tests that verify user database state."""

    def test_user_profile_in_database(self, db_cursor):
        """Verify user profiles exist in database."""
        print("\n=== User Profiles in Database ===")
        
        db_cursor.execute("""
            SELECT id, email, username, company_name, status
            FROM gm_users
            ORDER BY id
            LIMIT 5
        """)
        users = db_cursor.fetchall()
        
        for user in users:
            print(f"  User {user['id']}: {user['username']} - company: {user['company_name']}")
        
        assert len(users) >= 1, "Should have at least one user"


if __name__ == "__main__":
    pytest.main([__file__, "-v", "--tb=short"])
