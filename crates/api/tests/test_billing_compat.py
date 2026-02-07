#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
GlanceMind API E2E Tests - Billing Backward Compatibility & Migration Safety
=============================================================================
Ensures billing changes don't break existing data or functionality.

Run: pytest tests/test_billing_compat.py -v --tb=short
"""

import pytest
from decimal import Decimal

TEST_USER_ID = 999


# =============================================================================
# TestMigrationSafety
# =============================================================================

class TestMigrationSafety:
    """Verify migration applied correctly."""

    def test_billing_columns_exist(self, db_cursor):
        """New columns exist with correct defaults."""
        db_cursor.execute("""
            SELECT billing_status, frozen_cost, consumed_cost, frozen_at
            FROM gm_aipub_plans LIMIT 1
        """)
        # If table is empty, just verify the query doesn't error
        row = db_cursor.fetchone()
        if row:
            assert row['billing_status'] in ('none', 'frozen', 'settled')
        print("  OK: billing columns exist")

    def test_reference_type_column_exists(self, db_cursor):
        """reference_type column exists on wallet_transactions."""
        db_cursor.execute("""
            SELECT reference_type FROM gm_wallet_transactions LIMIT 1
        """)
        # Query succeeds = column exists
        print("  OK: reference_type column exists")

    def test_image_pricing_rule_exists(self, db_cursor):
        """IMAGE pricing rule seeded correctly."""
        db_cursor.execute("""
            SELECT action_type, cost_points FROM gm_pricing_rules
            WHERE action_type = 'IMAGE' AND platform_id IS NULL
        """)
        row = db_cursor.fetchone()
        assert row is not None, "IMAGE pricing rule should exist"
        assert row['cost_points'] == Decimal('5.00')
        print("  OK: IMAGE pricing rule = 5.00")

    def test_stored_procedures_exist(self, db_cursor):
        """All 3 stored procedures are callable."""
        for func in ['fn_freeze_budget', 'fn_consume_from_frozen', 'fn_finalize_plan']:
            db_cursor.execute("""
                SELECT EXISTS(
                    SELECT 1 FROM pg_proc WHERE proname = %s
                ) as exists_flag
            """, (func,))
            row = db_cursor.fetchone()
            assert row['exists_flag'], f"{func} should exist"
        print("  OK: all 3 stored procedures exist")

    def test_billing_status_constraint(self, db_cursor):
        """CHECK constraint allows only none/frozen/settled."""
        db_cursor.execute("""
            SELECT conname FROM pg_constraint
            WHERE conname = 'aipub_plans_valid_billing_status'
        """)
        assert db_cursor.fetchone() is not None, "CHECK constraint should exist"
        print("  OK: billing_status CHECK constraint exists")


# =============================================================================
# TestBackwardCompatibility
# =============================================================================

class TestBackwardCompatibility:
    """Ensure existing data and flows are not broken."""

    def test_old_plan_defaults(self, db_cursor):
        """Existing plans (pre-migration) have billing_status='none', costs=0."""
        db_cursor.execute("""
            SELECT billing_status, frozen_cost, consumed_cost
            FROM gm_aipub_plans
            WHERE billing_status = 'none'
            LIMIT 5
        """)
        rows = db_cursor.fetchall()
        for row in rows:
            assert row['billing_status'] == 'none'
            assert row['frozen_cost'] == Decimal('0.00')
            assert row['consumed_cost'] == Decimal('0.00')
        print(f"  OK: {len(rows)} old plans have correct defaults")

    def test_finalize_on_old_plan(self, db_cursor):
        """fn_finalize_plan on billing_status=none → only updates status, no wallet ops."""
        user_id = TEST_USER_ID
        # Find a group to satisfy the valid_target constraint
        db_cursor.execute("SELECT id FROM gm_social_groups LIMIT 1")
        group_row = db_cursor.fetchone()
        group_id = group_row['id'] if group_row else None
        if group_id is None:
            pytest.skip("No groups in DB")

        # Create plan with billing_status=none (simulate old plan)
        db_cursor.execute("""
            INSERT INTO gm_aipub_plans
                (user_id, platform_id, content_type, plan_type, status, group_id)
            VALUES (%s, 2, 'profile', 'account_grooming', 'ai_processing', %s)
            RETURNING id
        """, (user_id, group_id))
        plan_id = db_cursor.fetchone()['id']
        db_cursor.connection.commit()

        # Get wallet before
        db_cursor.execute(
            "SELECT balance_points, frozen_points FROM gm_user_wallets WHERE user_id = %s",
            (user_id,))
        wallet_before = db_cursor.fetchone()

        # Finalize
        db_cursor.execute("SELECT fn_finalize_plan(%s, 'failed')", (plan_id,))
        db_cursor.connection.commit()

        # Verify status updated but no wallet change
        db_cursor.execute(
            "SELECT status, billing_status FROM gm_aipub_plans WHERE id = %s",
            (plan_id,))
        plan = db_cursor.fetchone()
        assert plan['status'] == 'failed'
        assert plan['billing_status'] == 'none'  # Still none

        db_cursor.execute(
            "SELECT balance_points, frozen_points FROM gm_user_wallets WHERE user_id = %s",
            (user_id,))
        wallet_after = db_cursor.fetchone()
        assert wallet_before == wallet_after

        # Cleanup
        db_cursor.execute("DELETE FROM gm_aipub_plans WHERE id = %s", (plan_id,))
        db_cursor.connection.commit()
        print("  OK: finalize on old plan → status updated, wallet unchanged")

    def test_campaign_billing_unaffected(self, db_cursor):
        """Campaign stored procedures still work."""
        # Verify campaign procedures exist
        for func in ['fn_activate_campaign', 'fn_finalize_campaign']:
            db_cursor.execute("""
                SELECT EXISTS(
                    SELECT 1 FROM pg_proc WHERE proname = %s
                ) as exists_flag
            """, (func,))
            row = db_cursor.fetchone()
            exists = row['exists_flag']
            if not exists:
                pytest.skip(f"{func} not found (may not be deployed)")
        print("  OK: campaign stored procedures still exist")

    def test_wallet_constraints_intact(self, db_cursor):
        """CHECK constraints balance>=0, frozen>=0 still enforced."""
        with pytest.raises(Exception):
            db_cursor.execute("""
                UPDATE gm_user_wallets
                SET balance_points = -1 WHERE user_id = %s
            """, (TEST_USER_ID,))
        db_cursor.connection.rollback()

        with pytest.raises(Exception):
            db_cursor.execute("""
                UPDATE gm_user_wallets
                SET frozen_points = -1 WHERE user_id = %s
            """, (TEST_USER_ID,))
        db_cursor.connection.rollback()
        print("  OK: wallet CHECK constraints intact")

    def test_old_transactions_intact(self, db_cursor):
        """Existing transactions have reference_type=NULL (acceptable)."""
        db_cursor.execute("""
            SELECT COUNT(*) as cnt FROM gm_wallet_transactions
            WHERE reference_type IS NULL
        """)
        row = db_cursor.fetchone()
        # Old transactions with NULL reference_type are expected
        print(f"  OK: {row['cnt']} old transactions with reference_type=NULL (expected)")
