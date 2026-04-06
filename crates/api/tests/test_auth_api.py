"""
GlanceMind API E2E Tests - Authentication API
=============================================
Tests for authentication endpoints.

Test Coverage:
1. User registration
2. User login (email and username)
3. Token validation
4. Database state verification

Note: These tests adapt to whether email verification is required.
"""

import pytest
import uuid
from conftest import (
    assert_response_success,
    extract_data,
    API_BASE_URL,
)


class TestUserLogin:
    """Tests for user login functionality."""

    def test_login_wrong_password(self, api_client, db_cursor):
        """Test login with wrong password returns error."""
        # Use a mock user from the database
        db_cursor.execute("SELECT email FROM gm_users LIMIT 1")
        user = db_cursor.fetchone()
        
        if user:
            resp = api_client.post(
                "/api/v1/auth/login",
                json={
                    "identifier": user["email"],
                    "password": "definitely_wrong_password_123"
                }
            )
            
            # Should return error (400 or 401)
            assert resp.status_code in [400, 401], \
                f"Expected 400/401 for wrong password, got {resp.status_code}"

    def test_login_nonexistent_user(self, api_client):
        """Test login with non-existent user returns error."""
        resp = api_client.post(
            "/api/v1/auth/login",
            json={
                "identifier": f"nonexistent_{uuid.uuid4().hex[:8]}@test.com",
                "password": "anypassword"
            }
        )
        
        # Should return error
        assert resp.status_code in [400, 401, 404], \
            f"Expected error for non-existent user, got {resp.status_code}"

    def test_login_endpoint_exists(self, api_client):
        """Test that login endpoint exists and returns proper response format."""
        resp = api_client.post(
            "/api/v1/auth/login",
            json={
                "identifier": "test@test.com",
                "password": "password"
            }
        )
        
        # Response should be JSON - endpoint exists if we get a JSON response
        # Note: 404 for "user not found" is a valid response, endpoint still exists
        data = resp.json()
        assert isinstance(data, dict), "Response should be JSON object"
        
        # Should have error structure for bad credentials
        assert "code" in data or "msg" in data or "token" in data, \
            "Response should have expected structure"


class TestUserRegistration:
    """Tests for user registration functionality."""

    def test_register_endpoint_exists(self, api_client):
        """Test that registration endpoint exists."""
        unique_id = str(uuid.uuid4())[:8]
        
        resp = api_client.post(
            "/api/v1/auth/register",
            json={
                "email": f"test_{unique_id}@example.com",
                "username": f"user_{unique_id}",
                "password": "StrongPassword123!"
            }
        )
        
        # Should not be 404 (endpoint exists)
        assert resp.status_code != 404, "Register endpoint should exist"
        
        # Response should be JSON
        data = resp.json()
        assert isinstance(data, dict), "Response should be JSON object"
        print(f"\nRegistration response status: {resp.status_code}")
        print(f"Registration response: {data}")

    def test_register_requires_valid_email(self, api_client):
        """Test that registration requires valid email format."""
        unique_id = str(uuid.uuid4())[:8]
        
        resp = api_client.post(
            "/api/v1/auth/register",
            json={
                "email": "invalid-email",  # Invalid email format
                "username": f"user_{unique_id}",
                "password": "StrongPassword123!"
            }
        )
        
        # Should return validation error
        assert resp.status_code in [400, 422], \
            f"Expected 400/422 for invalid email, got {resp.status_code}"

    def test_register_requires_password(self, api_client):
        """Test that registration requires password."""
        unique_id = str(uuid.uuid4())[:8]
        
        resp = api_client.post(
            "/api/v1/auth/register",
            json={
                "email": f"test_{unique_id}@example.com",
                "username": f"user_{unique_id}",
                # Missing password
            }
        )
        
        # Should return validation error
        assert resp.status_code in [400, 422], \
            f"Expected 400/422 for missing password, got {resp.status_code}"


class TestTokenValidation:
    """Tests for JWT token validation."""

    def test_access_protected_route_without_token(self, api_client):
        """Test accessing protected route without token returns 401."""
        resp = api_client.get("/api/v1/user/me")
        
        assert resp.status_code == 401, \
            f"Expected 401 without token, got {resp.status_code}"

    def test_access_protected_route_invalid_token(self, api_client):
        """Test accessing protected route with invalid token returns 401."""
        api_client.session.headers["Authorization"] = "Bearer invalid_token_123"
        resp = api_client.get("/api/v1/user/me")
        
        assert resp.status_code == 401, \
            f"Expected 401 with invalid token, got {resp.status_code}"

    def test_access_protected_route_with_valid_token(self, auth_client):
        """Test accessing protected route with valid token."""
        resp = auth_client.get("/api/v1/user/me")
        
        # If we get here, auth_client successfully obtained a token
        assert_response_success(resp)
        
        data = resp.json()
        user_data = extract_data(data)
        print(f"\nUser me response: {user_data}")
        
        # Should return user info (either directly or wrapped)
        if isinstance(user_data, dict):
            assert any(k in user_data for k in ["id", "email", "username"]), \
                "Should return user information"


class TestDatabaseStateAuth:
    """Tests that verify authentication database state."""

    def test_mock_users_exist(self, db_cursor):
        """Verify mock users exist in database."""
        print("\n=== Mock Users Verification ===")
        
        db_cursor.execute("""
            SELECT id, email, username, status, role
            FROM gm_users
            ORDER BY id
            LIMIT 10
        """)
        users = db_cursor.fetchall()
        
        for user in users:
            print(f"  User {user['id']}: {user['username']} ({user['email']}) - {user['status']}")
        
        assert len(users) >= 2, "Should have at least 2 mock users"

    def test_mock_users_have_wallets(self, db_cursor):
        """Verify mock users have associated wallets."""
        print("\n=== User Wallets Verification ===")
        
        db_cursor.execute("""
            SELECT u.id, u.username, w.balance_points, w.frozen_points
            FROM gm_users u
            JOIN gm_user_wallets w ON u.id = w.user_id
            ORDER BY u.id
            LIMIT 10
        """)
        results = db_cursor.fetchall()
        
        for row in results:
            print(f"  User {row['id']} ({row['username']}): balance={row['balance_points']}, frozen={row['frozen_points']}")
        
        assert len(results) >= 2, "At least 2 users should have wallets"


if __name__ == "__main__":
    pytest.main([__file__, "-v", "--tb=short"])
