"""
GlanceMind API E2E Test Configuration
=====================================
Shared fixtures and utilities for E2E tests.
"""

import os
import pytest
import requests
import psycopg2
from psycopg2.extras import RealDictCursor


# =============================================================================
# Configuration
# =============================================================================

API_BASE_URL = os.getenv("API_BASE_URL", "http://localhost:8081")
DATABASE_URL = os.getenv(
    "DATABASE_URL", 
    "postgres://glancemind:testpassword@localhost:5434/glancemind_test"
)

# Test user credentials matching init-test-data.sql
# Password: TestPassword123!
TEST_USER_EMAIL = "e2e@glancemind.test"
TEST_USER_PASSWORD = "TestPassword123!"
TEST_USER_ID = 999

TEST_ADMIN_EMAIL = "e2e@glancemind.test"
TEST_ADMIN_PASSWORD = "TestPassword123!"
TEST_ADMIN_ID = 999


# =============================================================================
# Database Fixtures
# =============================================================================

@pytest.fixture(scope="session")
def db_connection():
    """Create a database connection for the entire test session."""
    conn = psycopg2.connect(DATABASE_URL)
    yield conn
    conn.close()


@pytest.fixture
def db_cursor(db_connection):
    """Create a cursor with dict results for each test."""
    cursor = db_connection.cursor(cursor_factory=RealDictCursor)
    yield cursor
    db_connection.rollback()  # Rollback any uncommitted changes
    cursor.close()


# =============================================================================
# API Client Fixtures
# =============================================================================

class APIClient:
    """HTTP client for API requests."""
    
    def __init__(self, base_url: str, token: str = None):
        self.base_url = base_url
        self.token = token
        self.session = requests.Session()
        if token:
            self.session.headers["Authorization"] = f"Bearer {token}"
    
    def get(self, path: str, **kwargs) -> requests.Response:
        return self.session.get(f"{self.base_url}{path}", **kwargs)
    
    def post(self, path: str, **kwargs) -> requests.Response:
        return self.session.post(f"{self.base_url}{path}", **kwargs)
    
    def put(self, path: str, **kwargs) -> requests.Response:
        return self.session.put(f"{self.base_url}{path}", **kwargs)
    
    def delete(self, path: str, **kwargs) -> requests.Response:
        return self.session.delete(f"{self.base_url}{path}", **kwargs)


def extract_data(response_json: dict):
    """Extract data from wrapped API response format.
    
    API returns: {"code": 1000, "data": [...], "msg": "Success"}
    This helper extracts the 'data' field if present.
    """
    if isinstance(response_json, dict) and "data" in response_json:
        return response_json["data"]
    return response_json


def register_test_user() -> str:
    """Register a new test user and return the token."""
    import uuid
    unique_id = str(uuid.uuid4())[:8]
    email = f"e2e_{unique_id}@test.com"
    username = f"e2e_{unique_id}"
    
    resp = requests.post(
        f"{API_BASE_URL}/api/v1/auth/register",
        json={
            "email": email,
            "username": username,
            "password": TEST_USER_PASSWORD
        }
    )
    
    if resp.status_code == 200:
        data = resp.json()
        # Response may be {"user": {...}, "token": {...}} or {"token": "..."}
        if "token" in data:
            token = data["token"]
            if isinstance(token, dict):
                return token.get("token")
            return token
    
    # If registration failed, try to login with existing mock user
    return None


def login_user(email: str, password: str) -> str:
    """Login and return JWT token."""
    resp = requests.post(
        f"{API_BASE_URL}/api/v1/auth/login",
        json={"identifier": email, "password": password}
    )
    if resp.status_code != 200:
        raise Exception(f"Login failed: {resp.status_code} - {resp.text}")
    
    data = resp.json()
    # Handle wrapped response: {"code": 1000, "data": {"token": "..."}}
    if "data" in data and isinstance(data["data"], dict):
        return data["data"].get("token")
    return data.get("token")


@pytest.fixture
def api_client():
    """Create an unauthenticated API client."""
    return APIClient(API_BASE_URL)


# Store token for reuse across tests
_cached_token = None


def get_or_create_test_token() -> str:
    """Get cached token or create a new test user.
    
    Strategy:
    1. Try to login with mock user from database (password: TestPassword123!)
    2. If login fails, try to register a new user
    3. As fallback, skip authenticated tests
    """
    global _cached_token
    if _cached_token:
        return _cached_token
    
    # First, try to login with the mock user from database
    login_resp = requests.post(
        f"{API_BASE_URL}/api/v1/auth/login",
        json={
            "identifier": TEST_USER_EMAIL,
            "password": TEST_USER_PASSWORD
        }
    )
    
    print(f"\nLogin response for {TEST_USER_EMAIL}: {login_resp.status_code}")
    
    if login_resp.status_code == 200:
        login_data = login_resp.json()
        print(f"Login data: {login_data}")
        
        # Extract token from various response formats
        if "data" in login_data and isinstance(login_data["data"], dict):
            token = login_data["data"].get("token")
        else:
            token = login_data.get("token")
        
        if token:
            _cached_token = token
            return token
    
    # Fallback: try to register a new user
    import uuid
    unique_id = str(uuid.uuid4())[:8]
    email = f"e2e_{unique_id}@test.com"
    username = f"e2e_{unique_id}"
    
    resp = requests.post(
        f"{API_BASE_URL}/api/v1/auth/register",
        json={
            "email": email,
            "username": username,
            "password": TEST_USER_PASSWORD
        }
    )
    
    print(f"Registration response: {resp.status_code}")
    
    if resp.status_code == 200:
        data = resp.json()
        
        # Extract token from various response formats
        if "token" in data:
            token = data["token"]
            if isinstance(token, dict):
                token = token.get("token")
            if token:
                _cached_token = token
                return token
    
    # If we get here, we couldn't get a token
    raise pytest.skip("Could not login or create test user")


@pytest.fixture
def auth_client(db_connection):
    """Create an authenticated API client with test user.
    
    Grants all permissions (bits=15) so existing tests aren't blocked
    by the permission middleware. Permission-specific tests override
    this via set_permissions().
    """
    try:
        token = get_or_create_test_token()
        cursor = db_connection.cursor()
        cursor.execute(
            "UPDATE gm_users SET permissions = 15 WHERE id = %s",
            (TEST_USER_ID,),
        )
        db_connection.commit()
        cursor.close()
        return APIClient(API_BASE_URL, token)
    except Exception as e:
        pytest.skip(f"Auth client not available: {e}")


@pytest.fixture
def admin_client():
    """Create an authenticated API client with admin user."""
    try:
        token = get_or_create_test_token()
        return APIClient(API_BASE_URL, token)
    except Exception as e:
        pytest.skip(f"Admin client not available: {e}")


# =============================================================================
# Utility Functions
# =============================================================================

def assert_response_success(response: requests.Response, expected_status: int = 200):
    """Assert that response has expected status code."""
    assert response.status_code == expected_status, \
        f"Expected {expected_status}, got {response.status_code}: {response.text}"


def assert_json_structure(data: dict, required_fields: list):
    """Assert that JSON data has required fields."""
    for field in required_fields:
        assert field in data, f"Missing required field: {field}"


# =============================================================================
# Test Data Constants
# =============================================================================

# Platform IDs (from init-test-data.sql)
PLATFORM_REDDIT = 1
PLATFORM_TIKTOK = 2
PLATFORM_FACEBOOK = 3
PLATFORM_INSTAGRAM = 4
PLATFORM_TWITTER = 5

# Campaign IDs with content
CAMPAIGN_REDDIT = 1
CAMPAIGN_TIKTOK = 2
CAMPAIGN_FACEBOOK = 3
CAMPAIGN_INSTAGRAM = 4
CAMPAIGN_TWITTER = 5

# Content IDs
TIKTOK_VIDEO_IDS = [1, 2, 3]
FACEBOOK_POST_IDS = [1, 2]
INSTAGRAM_POST_IDS = [1, 2, 3]
REDDIT_POST_IDS = [1, 2]
TWITTER_TWEET_IDS = [1, 2]
