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

# Test user credentials matching init-test-data.sql by default.
# These can be overridden so API E2E tests can run against an already-running
# local environment without requiring the legacy seed user.
TEST_USER_IDENTIFIER = os.getenv(
    "E2E_TEST_IDENTIFIER",
    os.getenv("E2E_TEST_EMAIL", "e2e@glancemind.test"),
)
TEST_USER_EMAIL = os.getenv("E2E_TEST_EMAIL", "e2e@glancemind.test")
TEST_USER_PASSWORD = os.getenv("E2E_TEST_PASSWORD", "TestPassword123!")
TEST_USER_ID = int(os.getenv("E2E_TEST_USER_ID", "999"))

TEST_ADMIN_EMAIL = os.getenv("E2E_TEST_ADMIN_EMAIL", TEST_USER_EMAIL)
TEST_ADMIN_PASSWORD = os.getenv("E2E_TEST_ADMIN_PASSWORD", TEST_USER_PASSWORD)
TEST_ADMIN_ID = int(os.getenv("E2E_TEST_ADMIN_ID", str(TEST_USER_ID)))


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
        url = f"{self.base_url}{path}"

        # The video generate endpoint uses Axum's multipart extractor, so
        # text-only form submissions also need to be encoded as multipart.
        if path == "/api/v1/video/generate" and "json" not in kwargs and "data" in kwargs:
            data = kwargs.pop("data") or {}
            raw_files = kwargs.pop("files", None) or {}

            headers = kwargs.get("headers")
            if headers:
                headers = dict(headers)
                headers.pop("Content-Type", None)
                kwargs["headers"] = headers

            if isinstance(data, dict) and isinstance(raw_files, dict):
                files = dict(raw_files)
                for key, value in data.items():
                    if value is None:
                        continue
                    files[key] = (None, str(value))
                return self.session.post(url, files=files, **kwargs)

            kwargs["data"] = data
            if raw_files:
                kwargs["files"] = raw_files

        return self.session.post(url, **kwargs)
    
    def put(self, path: str, **kwargs) -> requests.Response:
        return self.session.put(f"{self.base_url}{path}", **kwargs)
    
    def delete(self, path: str, **kwargs) -> requests.Response:
        return self.session.delete(f"{self.base_url}{path}", **kwargs)

    def patch(self, path: str, **kwargs) -> requests.Response:
        return self.session.patch(f"{self.base_url}{path}", **kwargs)


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


def login_test_user() -> str:
    """Backward-compatible helper used by older API E2E tests."""
    return get_or_create_test_token()


@pytest.fixture
def api_client():
    """Create an unauthenticated API client."""
    return APIClient(API_BASE_URL)


# Store token for reuse across tests
_cached_token = None


def resolve_test_user_id(db_connection) -> int:
    """Resolve the authenticated test user's database id."""
    if TEST_USER_ID and os.getenv("E2E_TEST_USER_ID"):
        return TEST_USER_ID

    cursor = db_connection.cursor(cursor_factory=RealDictCursor)
    candidates = []
    if TEST_USER_EMAIL:
        candidates.append(("SELECT id FROM gm_users WHERE email = %s", (TEST_USER_EMAIL,)))
    if TEST_USER_IDENTIFIER and TEST_USER_IDENTIFIER != TEST_USER_EMAIL:
        candidates.append(
            (
                "SELECT id FROM gm_users WHERE email = %s OR username = %s",
                (TEST_USER_IDENTIFIER, TEST_USER_IDENTIFIER),
            )
        )

    try:
        for sql, params in candidates:
            cursor.execute(sql, params)
            row = cursor.fetchone()
            if row:
                return int(row["id"])
    finally:
        cursor.close()

    raise LookupError(
        f"Could not resolve test user id for identifier={TEST_USER_IDENTIFIER!r}, email={TEST_USER_EMAIL!r}"
    )


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
            "identifier": TEST_USER_IDENTIFIER,
            "password": TEST_USER_PASSWORD
        }
    )
    
    print(f"\nLogin response for {TEST_USER_IDENTIFIER}: {login_resp.status_code}")
    
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
        user_id = resolve_test_user_id(db_connection)
        cursor = db_connection.cursor()
        cursor.execute(
            "UPDATE gm_users SET permissions = 15 WHERE id = %s",
            (user_id,),
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

# =============================================================================
# Auto-skip @requires_llm when mock LLM is unreliable
# =============================================================================

def pytest_collection_modifyitems(config, items):
    """Skip tests marked with @requires_llm unless RUN_LLM_TESTS=1 is set."""
    if os.getenv("RUN_LLM_TESTS", "0") == "1":
        return
    skip_llm = pytest.mark.skip(reason="requires_llm: set RUN_LLM_TESTS=1 to enable")
    for item in items:
        if "requires_llm" in item.keywords:
            item.add_marker(skip_llm)


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
