#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
GlanceMind API E2E Tests - Batch Create API (Module C)
======================================================
Tests for POST /api/v1/ai-tasks/batch (single-item MVP).

These require a running API (API_BASE_URL) and a usable auth client.
They assert the request-validation / business-error status codes from the
behavioral contract:
  - missing/empty idempotency_key  -> 400
  - unknown task_kind              -> 400
  - more than one item             -> 400 (MultiItemNotSupported, code 4503)
"""

import uuid

import pytest

try:
    from conftest import extract_data
except ImportError:
    def extract_data(json_resp):
        if isinstance(json_resp, dict) and "data" in json_resp:
            return json_resp["data"]
        return json_resp


BATCH_PATH = "/api/v1/ai-tasks/batch"

# Business error codes (mirror crates/api/src/response/error_code.rs).
CODE_MULTI_ITEM_NOT_SUPPORTED = 4503


def _key() -> str:
    return f"pytest-{uuid.uuid4().hex}"


class TestBatchCreateValidation:
    """Request-validation and business-error cases for the batch endpoint."""

    def test_batch_requires_idempotency_key(self, auth_client):
        """An empty idempotency_key must be rejected with 400."""
        payload = {
            "task_kind": "campaign",
            "idempotency_key": "",  # empty -> validator rejects
            "items": [{"name": "x"}],
        }
        resp = auth_client.post(BATCH_PATH, json=payload)
        assert resp.status_code == 400, (
            f"empty idempotency_key must yield 400, got {resp.status_code}: {resp.text}"
        )

    def test_batch_rejects_unknown_task_kind(self, auth_client):
        """An unknown task_kind must be rejected with 400."""
        payload = {
            "task_kind": "not_a_real_kind",
            "idempotency_key": _key(),
            "items": [{"name": "x"}],
        }
        resp = auth_client.post(BATCH_PATH, json=payload)
        assert resp.status_code == 400, (
            f"unknown task_kind must yield 400, got {resp.status_code}: {resp.text}"
        )

    def test_batch_rejects_multiple_items(self, auth_client):
        """More than one item must be rejected with 400 MultiItemNotSupported."""
        payload = {
            "task_kind": "campaign",
            "idempotency_key": _key(),
            "items": [{"name": "x"}, {"name": "y"}],
        }
        resp = auth_client.post(BATCH_PATH, json=payload)
        assert resp.status_code == 400, (
            f"multi-item batch must yield 400, got {resp.status_code}: {resp.text}"
        )
        body = resp.json()
        assert body.get("code") == CODE_MULTI_ITEM_NOT_SUPPORTED, (
            f"multi-item batch must use code {CODE_MULTI_ITEM_NOT_SUPPORTED} "
            f"(MultiItemNotSupported), got {body.get('code')}: {resp.text}"
        )


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
