# DEFERRED: requires running test DB (:15432) — verify when infra is up.
"""
M4 — Unified Vidu model endpoint test
======================================
Asserts that after the DB unification migration, only the single canonical
"vidu" model_key is active for provider=vidu/model_type=video.

Requires:
- API server running (make up)
- Test DB seeded with init-test-data.sql (includes unified row id=20)
"""

import pytest

from conftest import (
    API_BASE_URL,
    APIClient,
    get_or_create_test_token,
)

AI_MODELS_URL = "/api/v1/config/ai-models"


@pytest.fixture
def api_client():
    """Authenticated API client (config endpoint may require auth)."""
    token = get_or_create_test_token()
    return APIClient(API_BASE_URL, token=token)


def test_only_unified_vidu_video_model_is_active(api_client):
    resp = api_client.get(AI_MODELS_URL)
    assert resp.status_code == 200
    models = resp.json()
    models = models.get("list") or models.get("data") or models
    vidu_video = [m for m in models if m.get("model_type") == "video"
                  and (m.get("provider") == "vidu" or str(m.get("model_key","")).startswith("vidu"))]
    keys = sorted(m["model_key"] for m in vidu_video)
    assert keys == ["vidu"], f"expected only unified 'vidu', got {keys}"
