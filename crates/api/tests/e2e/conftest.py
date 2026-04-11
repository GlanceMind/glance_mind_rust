"""
GlanceMind API Full-Pipeline E2E Test Configuration
=====================================================
Tests the complete flow: API -> Scheduler -> Agent-RS -> DB
Uses gm_e2e_utils for shared test infrastructure.
"""

import os
import time

import httpx
import psycopg2
import pytest
import redis as redis_lib

from gm_e2e_utils import extract_data, NatsTestHelper


def pytest_collection_modifyitems(config, items):
    skip_llm = pytest.mark.skip(reason="requires real LLM (not available in mock E2E)")
    for item in items:
        if "requires_llm" in item.keywords:
            item.add_marker(skip_llm)


API_BASE_URL = os.environ.get("API_BASE_URL", "http://api:8000")
DATABASE_URL = os.environ.get("DATABASE_URL", "postgresql://glancemind:testpassword@postgres:5432/glancemind_e2e")
REDIS_URL = os.environ.get("REDIS_URL", "redis://redis:6379")
NATS_URL = os.environ.get("NATS_URL", "nats://nats:4222")

E2E_EMAIL = "pipeline-e2e@glancemind.test"
E2E_USERNAME = "pipeline_e2e_user"
E2E_PASSWORD = "TestPassword123!"
E2E_WALLET_BALANCE = 50000.00

PLATFORM_REDDIT = 1
PLATFORM_TIKTOK = 2
PLATFORM_FACEBOOK = 3
PLATFORM_INSTAGRAM = 4
PLATFORM_TWITTER = 5


@pytest.fixture(scope="session")
def db():
    conn = None
    for attempt in range(30):
        try:
            conn = psycopg2.connect(DATABASE_URL)
            conn.autocommit = True
            break
        except Exception:
            time.sleep(2)
    assert conn is not None, "Failed to connect to PostgreSQL"
    yield conn
    conn.close()


@pytest.fixture(scope="session")
def redis_client():
    client = None
    for attempt in range(20):
        try:
            client = redis_lib.from_url(REDIS_URL)
            client.ping()
            break
        except Exception:
            time.sleep(2)
    assert client is not None, "Failed to connect to Redis"
    yield client
    client.close()


@pytest.fixture(scope="session")
def api_client(db):
    """Session-scoped authenticated httpx client for full-pipeline tests."""
    client = httpx.Client(base_url=API_BASE_URL, timeout=30.0)

    cur = db.cursor()

    cur.execute("""
        INSERT INTO gm_users (email, username, password_hash, full_name, role, status, is_active, created_at)
        VALUES (%s, %s,
                '$2b$12$Ikc.R4FMMGahbGhfHlLl4.PciMiV37qXfHpPNCjGGQg/yOEgk7k/e',
                'Pipeline E2E User', 'user', 'ACTIVE', true, NOW())
        ON CONFLICT (email) DO NOTHING
    """, (E2E_EMAIL, E2E_USERNAME))

    cur.execute("SELECT id FROM gm_users WHERE email = %s", (E2E_EMAIL,))
    user_id = cur.fetchone()[0]

    cur.execute("""
        INSERT INTO gm_user_wallets (user_id, balance_points, frozen_points, created_at)
        VALUES (%s, %s, 0, NOW())
        ON CONFLICT (user_id) DO UPDATE SET balance_points = %s, updated_at = NOW()
    """, (user_id, E2E_WALLET_BALANCE, E2E_WALLET_BALANCE))

    cur.execute("""
        INSERT INTO gm_email_verifications (email, code, verified, expires_at, created_at)
        VALUES (%s, 'PIPE00', true, NOW() + INTERVAL '1 year', NOW())
        ON CONFLICT DO NOTHING
    """, (E2E_EMAIL,))

    cur.execute(
        "UPDATE gm_users SET permissions = 15 WHERE id = %s",
        (user_id,),
    )
    cur.close()

    resp = client.post(
        "/api/v1/auth/login",
        json={"identifier": E2E_EMAIL, "password": E2E_PASSWORD},
    )
    assert resp.status_code == 200, f"Login failed: {resp.status_code} {resp.text}"
    data = extract_data(resp)
    if isinstance(data.get("token"), dict):
        token = data["token"]["token"]
    else:
        token = data["token"]

    client.headers["Authorization"] = f"Bearer {token}"
    client.e2e_user_id = user_id
    client.e2e_token = token

    yield client
    client.close()


@pytest.fixture(scope="session")
def platform_groups(api_client):
    groups = {}
    for pid, name in [
        (PLATFORM_TIKTOK, "Pipeline TikTok Group"),
        (PLATFORM_INSTAGRAM, "Pipeline Instagram Group"),
        (PLATFORM_REDDIT, "Pipeline Reddit Group"),
        (PLATFORM_TWITTER, "Pipeline Twitter Group"),
        (PLATFORM_FACEBOOK, "Pipeline Facebook Group"),
    ]:
        resp = api_client.post(
            "/api/v1/social-groups",
            json={"platform_id": pid, "group_name": name},
        )
        assert resp.status_code == 200, f"Create group failed for platform {pid}: {resp.status_code} {resp.text}"
        data = extract_data(resp)
        groups[pid] = data["id"]
    return groups


@pytest.fixture(scope="session")
def platform_accounts(api_client, platform_groups):
    accounts = {}
    for pid, gid in platform_groups.items():
        accts = []
        for i in range(3):
            resp = api_client.post(
                "/api/v1/accounts",
                json={
                    "platform_id": pid,
                    "username": f"pipe_{pid}_account_{i}",
                },
            )
            assert resp.status_code == 200, f"Create account failed: {resp.status_code} {resp.text}"
            data = extract_data(resp)
            account_id = data["id"]

            resp = api_client.put(
                f"/api/v1/accounts/{account_id}",
                json={"group_id": gid},
            )
            assert resp.status_code == 200, f"Assign group failed: {resp.status_code} {resp.text}"

            accts.append(account_id)
        accounts[pid] = accts
    return accounts


@pytest.fixture(scope="session")
def user_id(api_client):
    return api_client.e2e_user_id
