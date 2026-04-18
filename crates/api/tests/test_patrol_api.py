"""
Patrol API E2E Tests — full-field assertions
=============================================
DB mock data → API response → field-by-field == DB value == expected value.
"""
import pytest
from conftest import (
    extract_data,
    assert_response_success,
    assert_json_structure,
)
from datetime import datetime, timezone

MOCK_REPORT_ID = "e2e-patrol-profile-001"
MOCK_NOTIF_REPORT_ID = "e2e-patrol-notif-001"
MOCK_ERROR_REPORT_ID = "e2e-patrol-error-001"
NOW = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M:%S+00")


def _ensure_social_account(db_cursor, conn, user_id):
    db_cursor.execute(
        "SELECT id FROM gm_social_accounts WHERE user_id = %s AND platform_id = 2 LIMIT 1",
        (user_id,),
    )
    row = db_cursor.fetchone()
    if row:
        return row["id"]
    db_cursor.execute(
        """INSERT INTO gm_social_accounts (user_id, platform_id, username, profile_name, status)
           VALUES (%s, 2, 'e2e_patrol_user', 'e2e_profile', 'active') RETURNING id""",
        (user_id,),
    )
    conn.commit()
    return db_cursor.fetchone()["id"]


def _insert_patrol_data(db_cursor, conn, user_id, sa_id):
    db_cursor.execute(
        """INSERT INTO gm_patrol_reports
            (report_id, report_type, user_id, device_id, started_at, completed_at,
             total_accounts, success_count, error_count)
        VALUES (%s, 'profile', %s, 'e2e-device', %s, %s, 1, 1, 0)
        ON CONFLICT (report_id) DO NOTHING""",
        (MOCK_REPORT_ID, user_id, NOW, NOW),
    )
    db_cursor.execute(
        """INSERT INTO gm_patrol_account_stats
            (report_id, report_type, social_account_id, device_id, user_id,
             platform_id, platform_name, username,
             followers_count, following_count, posts_count, total_likes,
             new_followers, received_likes, received_comments,
             received_dms, received_shares, received_mentions,
             partial, error, collected_at)
        VALUES (%s, 'profile', %s, 'e2e-device', %s,
                2, 'tiktok', 'e2e_patrol_user',
                6000, 100, 0, 483,
                0, 0, 0, 0, 0, 0,
                false, NULL, %s)""",
        (MOCK_REPORT_ID, sa_id, user_id, NOW),
    )
    db_cursor.execute(
        """INSERT INTO gm_patrol_reports
            (report_id, report_type, user_id, device_id, started_at, completed_at,
             total_accounts, success_count, error_count)
        VALUES (%s, 'notification', %s, 'e2e-device', %s, %s, 1, 1, 0)
        ON CONFLICT (report_id) DO NOTHING""",
        (MOCK_NOTIF_REPORT_ID, user_id, NOW, NOW),
    )
    db_cursor.execute(
        """INSERT INTO gm_patrol_account_stats
            (report_id, report_type, social_account_id, device_id, user_id,
             platform_id, platform_name, username,
             followers_count, following_count, posts_count, total_likes,
             new_followers, received_likes, received_comments,
             received_dms, received_shares, received_mentions,
             partial, error, collected_at)
        VALUES (%s, 'notification', %s, 'e2e-device', %s,
                2, 'tiktok', 'e2e_patrol_user',
                0, 0, 0, 0,
                15, 120, 8, 0, 3, 5,
                false, NULL, %s)""",
        (MOCK_NOTIF_REPORT_ID, sa_id, user_id, NOW),
    )
    db_cursor.execute(
        """INSERT INTO gm_patrol_reports
            (report_id, report_type, user_id, device_id, started_at, completed_at,
             total_accounts, success_count, error_count)
        VALUES (%s, 'profile', %s, 'e2e-device', %s, %s, 1, 0, 1)
        ON CONFLICT (report_id) DO NOTHING""",
        (MOCK_ERROR_REPORT_ID, user_id, NOW, NOW),
    )
    db_cursor.execute(
        """INSERT INTO gm_patrol_account_stats
            (report_id, report_type, social_account_id, device_id, user_id,
             platform_id, platform_name, username,
             followers_count, following_count, posts_count, total_likes,
             new_followers, received_likes, received_comments,
             received_dms, received_shares, received_mentions,
             partial, error, collected_at)
        VALUES (%s, 'profile', %s, 'e2e-device', %s,
                2, 'tiktok', 'e2e_patrol_user',
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                true, 'Browser timeout after 30s', %s)""",
        (MOCK_ERROR_REPORT_ID, sa_id, user_id, NOW),
    )
    conn.commit()


def _cleanup(db_cursor, conn):
    ids = (MOCK_REPORT_ID, MOCK_NOTIF_REPORT_ID, MOCK_ERROR_REPORT_ID)
    db_cursor.execute(
        "DELETE FROM gm_patrol_account_stats WHERE report_id IN %s", (ids,)
    )
    db_cursor.execute(
        "DELETE FROM gm_patrol_reports WHERE report_id IN %s", (ids,)
    )
    conn.commit()


@pytest.fixture(autouse=True)
def patrol_test_data(db_cursor, db_connection, auth_client):
    from conftest import resolve_test_user_id

    user_id = resolve_test_user_id(db_connection)
    sa_id = _ensure_social_account(db_cursor, db_connection, user_id)
    _cleanup(db_cursor, db_connection)
    _insert_patrol_data(db_cursor, db_connection, user_id, sa_id)
    yield {"user_id": user_id, "social_account_id": sa_id}
    _cleanup(db_cursor, db_connection)


# =====================================================================
# GET /api/v1/patrol/reports/:report_id — PatrolReportDto full fields
# =====================================================================


class TestPatrolReportDetail:
    def test_profile_report_dto_all_fields(self, auth_client, db_cursor):
        resp = auth_client.get(f"/api/v1/patrol/reports/{MOCK_REPORT_ID}")
        assert_response_success(resp)
        data = extract_data(resp.json())

        db_cursor.execute(
            "SELECT * FROM gm_patrol_reports WHERE report_id = %s",
            (MOCK_REPORT_ID,),
        )
        db_report = db_cursor.fetchone()
        assert data["report_id"] == db_report["report_id"] == MOCK_REPORT_ID
        assert data["report_type"] == db_report["report_type"] == "profile"
        assert data["user_id"] == db_report["user_id"]
        assert data["device_id"] == db_report["device_id"] == "e2e-device"
        assert data["total_accounts"] == db_report["total_accounts"] == 1
        assert data["success_count"] == db_report["success_count"] == 1
        assert data["error_count"] == db_report["error_count"] == 0
        assert "started_at" in data
        assert "completed_at" in data
        assert "created_at" in data

        assert len(data["accounts"]) >= 1
        acc = data["accounts"][0]
        db_cursor.execute(
            "SELECT * FROM gm_patrol_account_stats WHERE report_id = %s",
            (MOCK_REPORT_ID,),
        )
        db_acc = db_cursor.fetchone()
        assert acc["id"] == db_acc["id"]
        assert acc["social_account_id"] == db_acc["social_account_id"]
        assert acc["platform_name"] == db_acc["platform_name"] == "tiktok"
        assert acc["username"] == db_acc["username"] == "e2e_patrol_user"
        assert acc["followers_count"] == db_acc["followers_count"] == 6000
        assert acc["following_count"] == db_acc["following_count"] == 100
        assert acc["posts_count"] == db_acc["posts_count"] == 0
        assert acc["total_likes"] == db_acc["total_likes"] == 483
        assert acc["new_followers"] == db_acc["new_followers"] == 0
        assert acc["received_likes"] == db_acc["received_likes"] == 0
        assert acc["received_comments"] == db_acc["received_comments"] == 0
        assert acc["received_dms"] == db_acc["received_dms"] == 0
        assert acc["received_shares"] == db_acc["received_shares"] == 0
        assert acc["received_mentions"] == db_acc["received_mentions"] == 0
        assert acc["partial"] == db_acc["partial"] == False
        assert acc["error"] is None
        assert "collected_at" in acc

    def test_notification_report_dto_all_fields(self, auth_client, db_cursor):
        resp = auth_client.get(f"/api/v1/patrol/reports/{MOCK_NOTIF_REPORT_ID}")
        assert_response_success(resp)
        data = extract_data(resp.json())
        acc = data["accounts"][0]

        db_cursor.execute(
            "SELECT * FROM gm_patrol_account_stats WHERE report_id = %s",
            (MOCK_NOTIF_REPORT_ID,),
        )
        db_acc = db_cursor.fetchone()
        assert acc["new_followers"] == db_acc["new_followers"] == 15
        assert acc["received_likes"] == db_acc["received_likes"] == 120
        assert acc["received_comments"] == db_acc["received_comments"] == 8
        assert acc["received_shares"] == db_acc["received_shares"] == 3
        assert acc["received_mentions"] == db_acc["received_mentions"] == 5
        assert acc["received_dms"] == db_acc["received_dms"] == 0
        assert acc["total_likes"] == db_acc["total_likes"] == 0
        assert acc["partial"] == False
        assert acc["error"] is None

    def test_error_report_has_partial_and_error_text(self, auth_client):
        resp = auth_client.get(f"/api/v1/patrol/reports/{MOCK_ERROR_REPORT_ID}")
        assert_response_success(resp)
        data = extract_data(resp.json())
        assert data["error_count"] == 1
        acc = data["accounts"][0]
        assert acc["partial"] == True
        assert acc["error"] == "Browser timeout after 30s"


# =====================================================================
# GET /api/v1/patrol/latest — PatrolLatestDto full fields
# =====================================================================


class TestPatrolLatest:
    def test_returns_both_report_types(self, auth_client):
        resp = auth_client.get("/api/v1/patrol/latest")
        assert_response_success(resp)
        data = extract_data(resp.json())
        assert data["profile_report"] is not None
        assert data["notification_report"] is not None

    def test_nested_report_dto_fields(self, auth_client):
        resp = auth_client.get("/api/v1/patrol/latest")
        data = extract_data(resp.json())
        report_fields = {
            "id",
            "report_id",
            "report_type",
            "user_id",
            "device_id",
            "started_at",
            "completed_at",
            "total_accounts",
            "success_count",
            "error_count",
            "created_at",
            "accounts",
        }
        for key in ("profile_report", "notification_report"):
            r = data[key]
            assert report_fields.issubset(set(r.keys())), (
                f"{key} missing: {report_fields - set(r.keys())}"
            )

    def test_profile_stats_by_report_id(self, auth_client, db_cursor):
        resp = auth_client.get("/api/v1/patrol/latest")
        data = extract_data(resp.json())
        profile_report_id = data["profile_report"]["report_id"]

        db_cursor.execute(
            "SELECT * FROM gm_patrol_account_stats WHERE report_id = %s",
            (profile_report_id,),
        )
        db_rows = {r["id"]: r for r in db_cursor.fetchall()}

        matched = [s for s in data["account_stats"] if s["id"] in db_rows]
        assert len(matched) >= 1
        acc = matched[0]
        db_acc = db_rows[acc["id"]]
        assert acc["total_likes"] == db_acc["total_likes"]
        assert acc["followers_count"] == db_acc["followers_count"]

    def test_all_stats_dto_fields_present(self, auth_client):
        resp = auth_client.get("/api/v1/patrol/latest")
        data = extract_data(resp.json())
        expected = {
            "id",
            "report_type",
            "social_account_id",
            "platform_name",
            "username",
            "followers_count",
            "following_count",
            "posts_count",
            "total_likes",
            "new_followers",
            "received_likes",
            "received_comments",
            "received_dms",
            "received_shares",
            "received_mentions",
            "partial",
            "error",
            "collected_at",
        }
        for acc in data["account_stats"]:
            assert expected.issubset(set(acc.keys())), (
                f"Missing: {expected - set(acc.keys())}"
            )


# =====================================================================
# GET /api/v1/patrol/reports — PatrolReportsResponse full fields
# =====================================================================


class TestPatrolReportsList:
    def test_paginated_list(self, auth_client):
        resp = auth_client.get("/api/v1/patrol/reports?page=1&per_page=10")
        assert_response_success(resp)
        data = extract_data(resp.json())
        assert "reports" in data
        assert "total" in data
        assert data["total"] >= 2
        assert "page" in data
        assert "per_page" in data

    def test_report_item_all_fields(self, auth_client):
        resp = auth_client.get("/api/v1/patrol/reports?page=1&per_page=10")
        data = extract_data(resp.json())
        report_fields = {
            "id",
            "report_id",
            "report_type",
            "user_id",
            "device_id",
            "started_at",
            "completed_at",
            "total_accounts",
            "success_count",
            "error_count",
            "created_at",
            "accounts",
        }
        for r in data["reports"]:
            assert report_fields.issubset(set(r.keys())), (
                f"Missing: {report_fields - set(r.keys())}"
            )

    def test_filter_by_type(self, auth_client):
        resp = auth_client.get("/api/v1/patrol/reports?report_type=profile")
        assert_response_success(resp)
        data = extract_data(resp.json())
        for r in data["reports"]:
            assert r["report_type"] == "profile"


# =====================================================================
# GET /api/v1/patrol/summary — PatrolSummaryDto + PatrolReportBriefDto
# =====================================================================


class TestPatrolSummary:
    def test_summary_all_fields(self, auth_client):
        resp = auth_client.get("/api/v1/patrol/summary")
        assert_response_success(resp)
        data = extract_data(resp.json())
        assert_json_structure(
            data,
            [
                "total_reports",
                "total_accounts_monitored",
                "latest_profile_report",
                "latest_notification_report",
            ],
        )
        assert data["total_reports"] >= 2
        assert data["total_accounts_monitored"] >= 1

    def test_brief_report_all_fields(self, auth_client):
        resp = auth_client.get("/api/v1/patrol/summary")
        data = extract_data(resp.json())
        brief_fields = [
            "report_id",
            "report_type",
            "total_accounts",
            "success_count",
            "error_count",
            "completed_at",
        ]
        for key in ("latest_profile_report", "latest_notification_report"):
            brief = data[key]
            assert brief is not None, f"{key} should not be None"
            assert_json_structure(brief, brief_fields)


# =====================================================================
# GET /api/v1/patrol/accounts/:id/trend
# =====================================================================


class TestPatrolAccountTrend:
    def test_trend_returns_data(self, auth_client, patrol_test_data):
        sa_id = patrol_test_data["social_account_id"]
        resp = auth_client.get(f"/api/v1/patrol/accounts/{sa_id}/trend?days=7")
        assert_response_success(resp)
        data = extract_data(resp.json())
        assert isinstance(data, list)
        assert len(data) >= 1

    def test_trend_items_have_all_stats_fields(self, auth_client, patrol_test_data):
        sa_id = patrol_test_data["social_account_id"]
        resp = auth_client.get(f"/api/v1/patrol/accounts/{sa_id}/trend?days=7")
        data = extract_data(resp.json())
        expected = {
            "id",
            "report_type",
            "social_account_id",
            "platform_name",
            "username",
            "followers_count",
            "following_count",
            "posts_count",
            "total_likes",
            "new_followers",
            "received_likes",
            "received_comments",
            "received_dms",
            "received_shares",
            "received_mentions",
            "partial",
            "error",
            "collected_at",
        }
        for item in data:
            assert expected.issubset(set(item.keys())), (
                f"Missing: {expected - set(item.keys())}"
            )


# =====================================================================
# DB full-column integrity (columns not exposed by API)
# =====================================================================


class TestDbFullColumnIntegrity:
    def test_account_stats_hidden_columns(self, db_cursor):
        db_cursor.execute(
            "SELECT * FROM gm_patrol_account_stats WHERE report_id = %s",
            (MOCK_REPORT_ID,),
        )
        row = db_cursor.fetchone()
        assert row is not None
        assert row["report_id"] == MOCK_REPORT_ID
        assert row["report_type"] == "profile"
        assert row["device_id"] == "e2e-device"
        assert row["platform_id"] == 2
        assert row["created_at"] is not None

    def test_report_hidden_columns(self, db_cursor):
        db_cursor.execute(
            "SELECT * FROM gm_patrol_reports WHERE report_id = %s",
            (MOCK_REPORT_ID,),
        )
        row = db_cursor.fetchone()
        assert row is not None
        assert row["device_id"] == "e2e-device"
        assert row["created_at"] is not None
