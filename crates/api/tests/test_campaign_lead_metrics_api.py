"""
Campaign Lead Metrics API E2E Tests
====================================
Tests for GET /api/v1/campaigns/:id/lead-metrics

Per Task 0 findings (docs/.../2026-04-17-campaign-lead-metrics-task0-findings.md):
- `gm_patrol_reports` is idempotent but `gm_patrol_account_stats` is NOT
- Upstream `(report_id, social_account_id)` is a stable business key
- Decision: Branch B — repository dedups via `DISTINCT ON` before aggregation
"""
import uuid
from datetime import datetime, timedelta, timezone

import pytest
from conftest import (
    assert_response_success,
    extract_data,
)

# Task 0 Section 7.1 decision — see Task 0 findings doc.
TASK0_BRANCH = "B"


def _now_str(delta_minutes: int = 0) -> str:
    t = datetime.now(timezone.utc) + timedelta(minutes=delta_minutes)
    return t.strftime("%Y-%m-%d %H:%M:%S+00")


def _get_auth_user_id(auth_client):
    resp = auth_client.get("/api/v1/user/me")
    assert_response_success(resp)
    data = extract_data(resp.json())
    return data.get("id") or data.get("user_id")


def _get_config_ids(api_client):
    resp = api_client.get("/api/v1/config/platforms")
    platforms = extract_data(resp.json())
    platform_id = platforms[0]["id"]
    resp = api_client.get(f"/api/v1/config/platforms/{platform_id}/regions")
    regions = extract_data(resp.json())
    region_id = regions[0]["id"]
    resp = api_client.get("/api/v1/config/ai-models")
    models = extract_data(resp.json())
    ai_model_id = models[0]["id"]
    return platform_id, region_id, ai_model_id


def _create_campaign(auth_client, api_client, *, end_date=None):
    platform_id, region_id, ai_model_id = _get_config_ids(api_client)
    payload = {
        "name": f"Lead Metrics E2E {uuid.uuid4().hex[:8]}",
        "platform_id": platform_id,
        "region_id": region_id,
        "ai_model_id": ai_model_id,
        "schedule_type": "ONCE",
        "product_prompt": "Lead metrics e2e",
    }
    if end_date is not None:
        payload["end_date"] = end_date
    resp = auth_client.post("/api/v1/campaigns", json=payload)
    assert_response_success(resp)
    data = extract_data(resp.json())
    return data["id"], platform_id


def _ensure_social_account(db_cursor, conn, user_id, platform_id, username_suffix=""):
    username = f"lm_e2e_{username_suffix or uuid.uuid4().hex[:8]}"
    db_cursor.execute(
        """INSERT INTO gm_social_accounts (user_id, platform_id, username, profile_name, status)
           VALUES (%s, %s, %s, %s, 'active') RETURNING id""",
        (user_id, platform_id, username, f"profile_{username}"),
    )
    conn.commit()
    return db_cursor.fetchone()["id"]


def _attach_account_to_campaign(db_cursor, conn, campaign_id, account_id):
    db_cursor.execute(
        """INSERT INTO gm_campaign_accounts (campaign_id, account_id)
           VALUES (%s, %s)
           ON CONFLICT DO NOTHING""",
        (campaign_id, account_id),
    )
    conn.commit()


@pytest.fixture
def patrol_stats_inserter(db_cursor, db_connection):
    """Insert a patrol report + stats row. Returns function.

    All inserted report_ids are tracked for cleanup.
    """
    inserted_report_ids = []

    def _insert(
        *,
        user_id,
        social_account_id,
        platform_id,
        new_followers=0,
        received_dms=0,
        received_friend_requests=0,
        received_mentions=0,
        report_type="notification",
        report_id=None,
        collected_at=None,
    ):
        rid = report_id or f"lm-e2e-{uuid.uuid4().hex}"
        ts = collected_at or _now_str()
        # Insert the report (idempotent on report_id)
        db_cursor.execute(
            """INSERT INTO gm_patrol_reports
                (report_id, report_type, user_id, device_id, started_at, completed_at,
                 total_accounts, success_count, error_count)
               VALUES (%s, %s, %s, 'lm-e2e-device', %s, %s, 1, 1, 0)
               ON CONFLICT (report_id) DO NOTHING""",
            (rid, report_type, user_id, ts, ts),
        )
        # Insert stats (NOT idempotent — same pair can be written many times per Branch B)
        db_cursor.execute(
            """INSERT INTO gm_patrol_account_stats
                (report_id, report_type, social_account_id, device_id, user_id,
                 platform_id, platform_name, username,
                 followers_count, following_count, posts_count, total_likes,
                 new_followers, received_likes, received_comments,
                 received_dms, received_shares, received_mentions,
                 received_friend_requests,
                 partial, error, collected_at)
               VALUES (%s, %s, %s, 'lm-e2e-device', %s,
                       %s, 'tiktok', 'lm_e2e_user',
                       0, 0, 0, 0,
                       %s, 0, 0,
                       %s, 0, %s,
                       %s,
                       false, NULL, %s)""",
            (
                rid, report_type, social_account_id, user_id, platform_id,
                new_followers, received_dms, received_mentions,
                received_friend_requests, ts,
            ),
        )
        db_connection.commit()
        if rid not in inserted_report_ids:
            inserted_report_ids.append(rid)
        return rid

    yield _insert

    if inserted_report_ids:
        db_cursor.execute(
            "DELETE FROM gm_patrol_account_stats WHERE report_id = ANY(%s)",
            (inserted_report_ids,),
        )
        db_cursor.execute(
            "DELETE FROM gm_patrol_reports WHERE report_id = ANY(%s)",
            (inserted_report_ids,),
        )
        db_connection.commit()


@pytest.fixture
def cleanup_campaigns(db_cursor, db_connection):
    """Track campaigns created during a test and hard-delete them afterwards."""
    created = []
    yield created
    if created:
        db_cursor.execute(
            "DELETE FROM gm_campaign_accounts WHERE campaign_id = ANY(%s)",
            (created,),
        )
        db_cursor.execute(
            "DELETE FROM gm_campaigns WHERE id = ANY(%s)",
            (created,),
        )
        db_connection.commit()


# =========================================================================
# Task 6 — Basic E2E cases
# =========================================================================


class TestLeadMetricsBasic:
    def test_returns_zeros_when_no_accounts(
        self, auth_client, api_client, cleanup_campaigns
    ):
        """Campaign with zero attached accounts returns zero metrics."""
        campaign_id, _ = _create_campaign(auth_client, api_client)
        cleanup_campaigns.append(campaign_id)

        resp = auth_client.get(f"/api/v1/campaigns/{campaign_id}/lead-metrics")
        assert_response_success(resp)
        data = extract_data(resp.json())

        assert data["campaign_id"] == campaign_id
        assert data["account_count"] == 0
        assert data["tracked_account_count"] == 0
        assert data["new_followers"] == 0
        assert data["dms"] == 0
        assert data["friend_requests"] == 0
        assert data["mentions"] == 0
        assert data["last_updated_at"] is None
        assert "window_start_at" in data
        assert "window_end_at" in data

    def test_returns_404_for_nonexistent_campaign(self, auth_client):
        resp = auth_client.get("/api/v1/campaigns/99999999/lead-metrics")
        assert resp.status_code in (400, 404)

    def test_returns_401_without_auth(self, api_client):
        resp = api_client.get("/api/v1/campaigns/1/lead-metrics")
        assert resp.status_code == 401

    def test_window_end_clamped_to_now(
        self, auth_client, api_client, cleanup_campaigns
    ):
        """When campaign has no end_date, window_end should be <= now."""
        campaign_id, _ = _create_campaign(auth_client, api_client)
        cleanup_campaigns.append(campaign_id)
        before = datetime.now(timezone.utc)
        resp = auth_client.get(f"/api/v1/campaigns/{campaign_id}/lead-metrics")
        after = datetime.now(timezone.utc)
        assert_response_success(resp)
        data = extract_data(resp.json())

        window_end = datetime.fromisoformat(
            data["window_end_at"].replace("Z", "+00:00")
        )
        # Allow small clock drift (1 second each side)
        assert (before - timedelta(seconds=2)) <= window_end <= (
            after + timedelta(seconds=2)
        )


# =========================================================================
# Task 6b — Branch-specific aggregation tests
# =========================================================================


class TestLeadMetricsAggregation:
    def test_excludes_profile_and_sums_notification(
        self,
        auth_client,
        api_client,
        db_cursor,
        db_connection,
        patrol_stats_inserter,
        cleanup_campaigns,
    ):
        """Profile rows (cumulative) MUST NOT be summed; only notifications."""
        user_id = _get_auth_user_id(auth_client)
        campaign_id, platform_id = _create_campaign(auth_client, api_client)
        cleanup_campaigns.append(campaign_id)

        sa_id = _ensure_social_account(db_cursor, db_connection, user_id, platform_id)
        _attach_account_to_campaign(db_cursor, db_connection, campaign_id, sa_id)

        # 2 notifications + 1 profile (profile should be excluded)
        patrol_stats_inserter(
            user_id=user_id, social_account_id=sa_id, platform_id=platform_id,
            new_followers=3, received_dms=2, received_friend_requests=1,
            received_mentions=1, report_type="notification",
        )
        patrol_stats_inserter(
            user_id=user_id, social_account_id=sa_id, platform_id=platform_id,
            new_followers=4, received_dms=3, received_friend_requests=2,
            received_mentions=1, report_type="notification",
        )
        patrol_stats_inserter(
            user_id=user_id, social_account_id=sa_id, platform_id=platform_id,
            new_followers=999, received_dms=999, received_friend_requests=999,
            received_mentions=999, report_type="profile",
        )

        resp = auth_client.get(f"/api/v1/campaigns/{campaign_id}/lead-metrics")
        assert_response_success(resp)
        data = extract_data(resp.json())

        assert data["new_followers"] == 7
        assert data["dms"] == 5
        assert data["friend_requests"] == 3
        assert data["mentions"] == 2
        assert data["account_count"] == 1
        assert data["tracked_account_count"] == 1
        assert data["last_updated_at"] is not None

    @pytest.mark.skipif(TASK0_BRANCH != "A", reason="Branch A only")
    def test_branch_a_no_duplicate_rows_expected(
        self,
        auth_client,
        api_client,
        db_cursor,
        db_connection,
        patrol_stats_inserter,
        cleanup_campaigns,
    ):
        user_id = _get_auth_user_id(auth_client)
        campaign_id, platform_id = _create_campaign(auth_client, api_client)
        cleanup_campaigns.append(campaign_id)
        sa_id = _ensure_social_account(db_cursor, db_connection, user_id, platform_id)
        _attach_account_to_campaign(db_cursor, db_connection, campaign_id, sa_id)

        # 3 independent patrol rounds
        for nf in (1, 2, 3):
            patrol_stats_inserter(
                user_id=user_id, social_account_id=sa_id, platform_id=platform_id,
                new_followers=nf, report_type="notification",
            )

        resp = auth_client.get(f"/api/v1/campaigns/{campaign_id}/lead-metrics")
        assert_response_success(resp)
        data = extract_data(resp.json())
        assert data["new_followers"] == 6

    @pytest.mark.skipif(TASK0_BRANCH != "B", reason="Branch B only")
    def test_branch_b_dedup_duplicated_report_id(
        self,
        auth_client,
        api_client,
        db_cursor,
        db_connection,
        patrol_stats_inserter,
        cleanup_campaigns,
    ):
        """Writer is not idempotent; same (report_id, account) written 3x MUST dedup."""
        user_id = _get_auth_user_id(auth_client)
        campaign_id, platform_id = _create_campaign(auth_client, api_client)
        cleanup_campaigns.append(campaign_id)
        sa_id = _ensure_social_account(db_cursor, db_connection, user_id, platform_id)
        _attach_account_to_campaign(db_cursor, db_connection, campaign_id, sa_id)

        fixed_report_id = f"lm-e2e-dup-{uuid.uuid4().hex}"
        for _ in range(3):
            patrol_stats_inserter(
                user_id=user_id, social_account_id=sa_id, platform_id=platform_id,
                new_followers=10, received_dms=20, received_friend_requests=5,
                received_mentions=4, report_type="notification",
                report_id=fixed_report_id,
            )

        resp = auth_client.get(f"/api/v1/campaigns/{campaign_id}/lead-metrics")
        assert_response_success(resp)
        data = extract_data(resp.json())

        # Dedup by (report_id, social_account_id) → only one row's values count
        assert data["new_followers"] == 10
        assert data["dms"] == 20
        assert data["friend_requests"] == 5
        assert data["mentions"] == 4
        assert data["account_count"] == 1
        assert data["tracked_account_count"] == 1
