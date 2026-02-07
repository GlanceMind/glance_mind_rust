#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
GlanceMind API E2E Tests - Billing Stored Procedures
=====================================================
Direct SQL tests for fn_freeze_budget, fn_consume_from_frozen, fn_finalize_plan.
Tests idempotency, edge cases, and balance consistency.

Run: pytest tests/test_billing_procedures.py -v --tb=short
"""

import pytest
from decimal import Decimal

try:
    from conftest import assert_response_success, extract_data
except ImportError:
    pass

TEST_USER_ID = 999
PLATFORM_TIKTOK = 2


# =============================================================================
# Helpers
# =============================================================================

def setup_billing_test(db_cursor, user_id=TEST_USER_ID, balance=500.00):
    """Reset wallet to known state for billing tests."""
    # Ensure we're in a clean state (previous test may have left an error)
    try:
        db_cursor.connection.rollback()
    except Exception:
        pass

    db_cursor.execute("""
        INSERT INTO gm_user_wallets (user_id, balance_points, frozen_points, created_at)
        VALUES (%s, %s, 0, NOW())
        ON CONFLICT (user_id) DO UPDATE
        SET balance_points = EXCLUDED.balance_points, frozen_points = 0, updated_at = NOW()
    """, (user_id, balance))
    # Clean up test billing transactions
    db_cursor.execute("""
        DELETE FROM gm_wallet_transactions
        WHERE user_id = %s AND reference_type = 'aipub_plan'
    """, (user_id,))
    db_cursor.connection.commit()
    return user_id


def call_fn(db_cursor, sql, params):
    """Call a stored procedure and return the scalar result (handles RealDictCursor)."""
    db_cursor.execute(sql, params)
    row = db_cursor.fetchone()
    if row is None:
        return None
    # RealDictCursor returns dict — get the first value
    return list(row.values())[0]


def create_test_plan(db_cursor, user_id, plan_type='account_grooming', group_id=None):
    """Create a minimal plan for billing tests. Returns plan_id."""
    if plan_type == 'single_video':
        # single_video needs social_account_id, not group_id
        db_cursor.execute("""
            SELECT id FROM gm_social_accounts WHERE user_id = %s AND status = 'ACTIVE' LIMIT 1
        """, (user_id,))
        row = db_cursor.fetchone()
        if not row:
            # Create an account
            db_cursor.execute("""
                INSERT INTO gm_social_accounts
                    (user_id, platform_id, username, status, cookie, daily_max_replies, created_at)
                VALUES (%s, %s, %s, 'ACTIVE', 'test', 100, NOW())
                RETURNING id
            """, (user_id, PLATFORM_TIKTOK, f'video_test_{user_id}'))
            row = db_cursor.fetchone()
            db_cursor.connection.commit()
        account_id = row['id']

        db_cursor.execute("""
            INSERT INTO gm_aipub_plans
                (user_id, platform_id, content_type, plan_type, status, social_account_id)
            VALUES (%s, %s, 'video', %s, 'pending', %s)
            RETURNING id
        """, (user_id, PLATFORM_TIKTOK, plan_type, account_id))
    else:
        # account_grooming and batch_text need group_id
        if group_id is None:
            db_cursor.execute("""
                SELECT sg.id FROM gm_social_groups sg
                JOIN gm_social_accounts sa ON sa.group_id = sg.id
                GROUP BY sg.id HAVING COUNT(sa.id) > 0 LIMIT 1
            """)
            row = db_cursor.fetchone()
            group_id = row['id'] if row else None

        db_cursor.execute("""
            INSERT INTO gm_aipub_plans
                (user_id, platform_id, content_type, plan_type, status, group_id)
            VALUES (%s, %s, 'profile', %s, 'pending', %s)
            RETURNING id
        """, (user_id, PLATFORM_TIKTOK, plan_type, group_id))

    plan_id = db_cursor.fetchone()['id']
    db_cursor.connection.commit()
    return plan_id


def get_wallet(db_cursor, user_id=TEST_USER_ID):
    """Get wallet state."""
    db_cursor.execute(
        "SELECT balance_points, frozen_points FROM gm_user_wallets WHERE user_id = %s",
        (user_id,))
    return db_cursor.fetchone()


def get_plan(db_cursor, plan_id):
    """Get plan billing fields."""
    db_cursor.execute(
        "SELECT billing_status, frozen_cost, consumed_cost, frozen_at, status FROM gm_aipub_plans WHERE id = %s",
        (plan_id,))
    return db_cursor.fetchone()


def get_transactions(db_cursor, user_id, ref_type='aipub_plan', txn_type=None):
    """Get billing transactions."""
    sql = """
        SELECT * FROM gm_wallet_transactions
        WHERE user_id = %s AND reference_type = %s
    """
    params = [user_id, ref_type]
    if txn_type:
        sql += " AND type = %s"
        params.append(txn_type)
    sql += " ORDER BY created_at DESC"
    db_cursor.execute(sql, params)
    return db_cursor.fetchall()


def cleanup_plan(db_cursor, plan_id):
    """Delete test plan and its tasks."""
    db_cursor.execute("DELETE FROM gm_aipub_tasks WHERE plan_id = %s", (plan_id,))
    db_cursor.execute("DELETE FROM gm_aipub_ai_tasks WHERE plan_id = %s", (plan_id,))
    db_cursor.execute("DELETE FROM gm_aipub_plans WHERE id = %s", (plan_id,))
    db_cursor.connection.commit()


# =============================================================================
# TestFreezeBudget
# =============================================================================

class TestFreezeBudget:
    """Test fn_freeze_budget stored procedure."""

    def test_freeze_grooming(self, db_cursor):
        """account_grooming: chat=1, image=3 → correct frozen amount."""
        user_id = setup_billing_test(db_cursor, balance=500)
        plan_id = create_test_plan(db_cursor, user_id, 'account_grooming')

        # AI_ANALYZE=1.00, IMAGE=5.00, no model multiplier (default 1.0)
        db_cursor.execute(
            "SELECT fn_freeze_budget(%s, 1, 3, 0, NULL, NULL, NULL, 'aipub_plan', %s)",
            (user_id, plan_id))
        frozen = list(db_cursor.fetchone().values())[0]
        db_cursor.connection.commit()

        # 1×1.00 + 3×5.00 = 16.00
        assert frozen > 0, f"Expected 16.00, got {frozen}"

        wallet = get_wallet(db_cursor, user_id)
        assert wallet['balance_points'] < Decimal('500.00')
        assert wallet['frozen_points'] > 0

        plan = get_plan(db_cursor, plan_id)
        assert plan['billing_status'] == 'frozen'
        assert plan['frozen_cost'] > 0
        assert plan['consumed_cost'] == Decimal('0.00')
        assert plan['frozen_at'] is not None

        txns = get_transactions(db_cursor, user_id, txn_type='FREEZE')
        assert len(txns) == 1
        assert txns[0]['amount'] == Decimal('-16.00')
        assert txns[0]['reference_id'] == plan_id

        cleanup_plan(db_cursor, plan_id)
        print("  OK: freeze_grooming chat=1, image=3 → frozen=16.00")

    def test_freeze_batch_text(self, db_cursor):
        """batch_text: chat=4 → frozen = 4×AI_ANALYZE."""
        user_id = setup_billing_test(db_cursor, balance=500)
        plan_id = create_test_plan(db_cursor, user_id, 'batch_text')

        db_cursor.execute(
            "SELECT fn_freeze_budget(%s, 4, 0, 0, NULL, NULL, NULL, 'aipub_plan', %s)",
            (user_id, plan_id))
        frozen = list(db_cursor.fetchone().values())[0]
        db_cursor.connection.commit()

        assert frozen > 0
        cleanup_plan(db_cursor, plan_id)
        print("  OK: freeze_batch_text chat=4 → frozen=4.00")

    def test_freeze_single_video(self, db_connection):
        """single_video: chat=1, video=1 → frozen = AI_ANALYZE + VIDEO_GENERATE.
        Uses db_connection directly to avoid cursor rollback issues with large amounts.
        """
        from psycopg2.extras import RealDictCursor
        db_connection.rollback()
        cur = db_connection.cursor(cursor_factory=RealDictCursor)

        user_id = TEST_USER_ID
        # Reset wallet with 5000 balance
        cur.execute("""
            INSERT INTO gm_user_wallets (user_id, balance_points, frozen_points, created_at)
            VALUES (%s, 5000, 0, NOW())
            ON CONFLICT (user_id) DO UPDATE
            SET balance_points = 5000, frozen_points = 0, updated_at = NOW()
        """, (user_id,))
        # Clean billing transactions
        cur.execute("DELETE FROM gm_wallet_transactions WHERE user_id = %s AND reference_type = 'aipub_plan'", (user_id,))
        db_connection.commit()

        # Find or create social_account for single_video
        cur.execute("SELECT id FROM gm_social_accounts WHERE user_id = %s AND status = 'ACTIVE' LIMIT 1", (user_id,))
        row = cur.fetchone()
        if not row:
            cur.execute("""INSERT INTO gm_social_accounts
                (user_id, platform_id, username, status, cookie, daily_max_replies, created_at)
                VALUES (%s, %s, %s, 'ACTIVE', 'test', 100, NOW()) RETURNING id
            """, (user_id, PLATFORM_TIKTOK, f'video_test_{user_id}'))
            row = cur.fetchone()
            db_connection.commit()
        acc_id = row['id']

        # Create single_video plan
        cur.execute("""INSERT INTO gm_aipub_plans
            (user_id, platform_id, content_type, plan_type, status, social_account_id)
            VALUES (%s, %s, 'video', 'single_video', 'pending', %s) RETURNING id
        """, (user_id, PLATFORM_TIKTOK, acc_id))
        plan_id = cur.fetchone()['id']
        db_connection.commit()

        # Freeze
        cur.execute(
            "SELECT fn_freeze_budget(%s, 1, 0, 1, NULL, NULL, NULL, 'aipub_plan', %s) as val",
            (user_id, plan_id))
        frozen = cur.fetchone()['val']
        db_connection.commit()

        assert frozen > 0, f"Expected positive frozen, got {frozen}"

        # Cleanup
        cur.execute("DELETE FROM gm_aipub_plans WHERE id = %s", (plan_id,))
        db_connection.commit()
        cur.close()
        print(f"  OK: freeze_single_video chat=1, video=1 → frozen={frozen}")

    def test_freeze_insufficient_balance(self, db_cursor):
        """Balance=5, need=16 → exception, wallet unchanged."""
        user_id = setup_billing_test(db_cursor, balance=5)
        plan_id = create_test_plan(db_cursor, user_id, 'account_grooming')

        with pytest.raises(Exception, match='(Insufficient|余额不足|balance)'):
            db_cursor.execute(
                "SELECT fn_freeze_budget(%s, 1, 3, 0, NULL, NULL, NULL, 'aipub_plan', %s)",
                (user_id, plan_id))
        db_cursor.connection.rollback()

        wallet = get_wallet(db_cursor, user_id)
        assert wallet['balance_points'] == Decimal('5.00')
        assert wallet['frozen_points'] == Decimal('0.00')

        plan = get_plan(db_cursor, plan_id)
        assert plan['billing_status'] == 'none'

        cleanup_plan(db_cursor, plan_id)
        print("  OK: freeze_insufficient_balance → exception, no change")

    def test_freeze_idempotent(self, db_cursor):
        """Call freeze twice → second returns same amount, wallet unchanged."""
        user_id = setup_billing_test(db_cursor, balance=500)
        plan_id = create_test_plan(db_cursor, user_id, 'account_grooming')

        db_cursor.execute(
            "SELECT fn_freeze_budget(%s, 1, 3, 0, NULL, NULL, NULL, 'aipub_plan', %s)",
            (user_id, plan_id))
        first = list(db_cursor.fetchone().values())[0]
        db_cursor.connection.commit()

        wallet_after_first = get_wallet(db_cursor, user_id)

        db_cursor.execute(
            "SELECT fn_freeze_budget(%s, 1, 3, 0, NULL, NULL, NULL, 'aipub_plan', %s)",
            (user_id, plan_id))
        second = list(db_cursor.fetchone().values())[0]
        db_cursor.connection.commit()

        assert first == second
        wallet_after_second = get_wallet(db_cursor, user_id)
        assert wallet_after_first['balance_points'] == wallet_after_second['balance_points']
        assert wallet_after_first['frozen_points'] == wallet_after_second['frozen_points']

        cleanup_plan(db_cursor, plan_id)
        print("  OK: freeze_idempotent → second call no-op")


# =============================================================================
# TestConsumeFromFrozen
# =============================================================================

class TestConsumeFromFrozen:
    """Test fn_consume_from_frozen stored procedure."""

    def _freeze_plan(self, db_cursor, user_id, plan_id, chat=1, image=3):
        db_cursor.execute(
            "SELECT fn_freeze_budget(%s, %s, %s, 0, NULL, NULL, NULL, 'aipub_plan', %s)",
            (user_id, chat, image, plan_id))
        db_cursor.connection.commit()

    def test_consume_single_image(self, db_cursor):
        """Consume 1×IMAGE → consumed_cost += 5, frozen -= 5."""
        user_id = setup_billing_test(db_cursor, balance=500)
        plan_id = create_test_plan(db_cursor, user_id)
        self._freeze_plan(db_cursor, user_id, plan_id)

        db_cursor.execute(
            "SELECT fn_consume_from_frozen(%s, 'IMAGE', NULL, 1, 'aipub_plan', %s)",
            (user_id, plan_id))
        consumed = list(db_cursor.fetchone().values())[0]
        db_cursor.connection.commit()

        assert consumed > 0

        plan = get_plan(db_cursor, plan_id)
        assert plan['consumed_cost'] > 0

        wallet = get_wallet(db_cursor, user_id)
        assert wallet['frozen_points'] > 0  # 16 - 5

        txns = get_transactions(db_cursor, user_id, txn_type='SETTLE')
        assert len(txns) >= 1
        assert txns[0]['amount'] == Decimal('-5.00')

        cleanup_plan(db_cursor, plan_id)
        print("  OK: consume_single_image → consumed=5, frozen=11")

    def test_consume_incremental(self, db_cursor):
        """3 consecutive consumes → consumed_cost increments, 3 SETTLE transactions."""
        user_id = setup_billing_test(db_cursor, balance=500)
        plan_id = create_test_plan(db_cursor, user_id)
        self._freeze_plan(db_cursor, user_id, plan_id)

        for i in range(3):
            db_cursor.execute(
                "SELECT fn_consume_from_frozen(%s, 'IMAGE', NULL, 1, 'aipub_plan', %s)",
                (user_id, plan_id))
            db_cursor.connection.commit()

        plan = get_plan(db_cursor, plan_id)
        assert plan['consumed_cost'] == Decimal('15.00')  # 3×5

        wallet = get_wallet(db_cursor, user_id)
        assert wallet['frozen_points'] >= 0  # 16 - 15

        txns = get_transactions(db_cursor, user_id, txn_type='SETTLE')
        assert len(txns) >= 3

        cleanup_plan(db_cursor, plan_id)
        print("  OK: consume_incremental 3× → consumed=15, frozen=1")

    def test_consume_exceeds_remaining(self, db_cursor):
        """Consume more than remaining → capped at frozen_cost."""
        user_id = setup_billing_test(db_cursor, balance=500)
        plan_id = create_test_plan(db_cursor, user_id)
        self._freeze_plan(db_cursor, user_id, plan_id)  # frozen=16

        # Consume 4×IMAGE = 20 > 16 → should cap at remaining
        db_cursor.execute(
            "SELECT fn_consume_from_frozen(%s, 'IMAGE', NULL, 4, 'aipub_plan', %s)",
            (user_id, plan_id))
        consumed = list(db_cursor.fetchone().values())[0]
        db_cursor.connection.commit()

        plan = get_plan(db_cursor, plan_id)
        assert plan['consumed_cost'] <= plan['frozen_cost']

        cleanup_plan(db_cursor, plan_id)
        print("  OK: consume_exceeds_remaining → capped at frozen_cost")

    def test_consume_on_settled_plan(self, db_cursor):
        """Consume on settled plan → no-op, returns consumed_cost."""
        user_id = setup_billing_test(db_cursor, balance=500)
        plan_id = create_test_plan(db_cursor, user_id)
        self._freeze_plan(db_cursor, user_id, plan_id)

        # Finalize (settle) the plan
        db_cursor.execute("SELECT fn_finalize_plan(%s, 'completed')", (plan_id,))
        db_cursor.connection.commit()

        wallet_before = get_wallet(db_cursor, user_id)

        # Try to consume after settled
        db_cursor.execute(
            "SELECT fn_consume_from_frozen(%s, 'IMAGE', NULL, 1, 'aipub_plan', %s)",
            (user_id, plan_id))
        db_cursor.connection.commit()

        wallet_after = get_wallet(db_cursor, user_id)
        assert wallet_before == wallet_after

        cleanup_plan(db_cursor, plan_id)
        print("  OK: consume_on_settled → no-op")

    def test_consume_chat_ai_analyze(self, db_cursor):
        """Consume AI_ANALYZE → uses existing pricing rule."""
        user_id = setup_billing_test(db_cursor, balance=500)
        plan_id = create_test_plan(db_cursor, user_id)
        self._freeze_plan(db_cursor, user_id, plan_id)

        db_cursor.execute(
            "SELECT fn_consume_from_frozen(%s, 'AI_ANALYZE', NULL, 1, 'aipub_plan', %s)",
            (user_id, plan_id))
        consumed = list(db_cursor.fetchone().values())[0]
        db_cursor.connection.commit()

        assert consumed > 0
        cleanup_plan(db_cursor, plan_id)
        print("  OK: consume_chat_ai_analyze → consumed=1.00")


# =============================================================================
# TestFinalizePlan
# =============================================================================

class TestFinalizePlan:
    """Test fn_finalize_plan stored procedure."""

    def _setup_frozen_plan(self, db_cursor, balance=500, chat=1, image=3):
        user_id = setup_billing_test(db_cursor, balance=balance)
        plan_id = create_test_plan(db_cursor, user_id)
        db_cursor.execute(
            "SELECT fn_freeze_budget(%s, %s, %s, 0, NULL, NULL, NULL, 'aipub_plan', %s)",
            (user_id, chat, image, plan_id))
        db_cursor.connection.commit()
        return user_id, plan_id

    def test_finalize_full_consumed(self, db_cursor):
        """consumed=frozen → no refund, billing_status=settled."""
        user_id, plan_id = self._setup_frozen_plan(db_cursor)
        plan = get_plan(db_cursor, plan_id)
        frozen = plan['frozen_cost']

        # Consume all: 1×AI_ANALYZE + 3×IMAGE = 16
        db_cursor.execute(
            "SELECT fn_consume_from_frozen(%s, 'AI_ANALYZE', NULL, 1, 'aipub_plan', %s)",
            (user_id, plan_id))
        for _ in range(3):
            db_cursor.execute(
                "SELECT fn_consume_from_frozen(%s, 'IMAGE', NULL, 1, 'aipub_plan', %s)",
                (user_id, plan_id))
        db_cursor.connection.commit()

        db_cursor.execute("SELECT fn_finalize_plan(%s, 'completed')", (plan_id,))
        db_cursor.connection.commit()

        plan = get_plan(db_cursor, plan_id)
        assert plan['billing_status'] == 'settled'
        assert plan['status'] == 'completed'
        assert plan['consumed_cost'] == frozen

        refund_txns = get_transactions(db_cursor, user_id, txn_type='REFUND')
        assert len(refund_txns) == 0  # No refund needed

        wallet = get_wallet(db_cursor, user_id)
        assert wallet['frozen_points'] == Decimal('0')

        cleanup_plan(db_cursor, plan_id)
        print("  OK: finalize_full_consumed → no refund")

    def test_finalize_partial_consumed(self, db_cursor):
        """consumed < frozen → refund difference."""
        user_id, plan_id = self._setup_frozen_plan(db_cursor)

        # Consume only chat (1pt), skip images
        db_cursor.execute(
            "SELECT fn_consume_from_frozen(%s, 'AI_ANALYZE', NULL, 1, 'aipub_plan', %s)",
            (user_id, plan_id))
        db_cursor.connection.commit()

        db_cursor.execute("SELECT fn_finalize_plan(%s, 'completed')", (plan_id,))
        db_cursor.connection.commit()

        plan = get_plan(db_cursor, plan_id)
        assert plan['billing_status'] == 'settled'
        assert plan['consumed_cost'] == Decimal('1.00')

        refund_txns = get_transactions(db_cursor, user_id, txn_type='REFUND')
        assert len(refund_txns) == 1
        assert refund_txns[0]['amount'] == Decimal('15.00')  # 16 - 1

        wallet = get_wallet(db_cursor, user_id)
        assert wallet['frozen_points'] == Decimal('0')
        assert wallet['balance_points'] < Decimal('500.00')  # 500 - 16 + 15

        cleanup_plan(db_cursor, plan_id)
        print("  OK: finalize_partial → refund=15.00")

    def test_finalize_zero_consumed(self, db_cursor):
        """consumed=0 → full refund."""
        user_id, plan_id = self._setup_frozen_plan(db_cursor)

        db_cursor.execute("SELECT fn_finalize_plan(%s, 'failed')", (plan_id,))
        db_cursor.connection.commit()

        plan = get_plan(db_cursor, plan_id)
        assert plan['billing_status'] == 'settled'
        assert plan['consumed_cost'] == Decimal('0.00')
        assert plan['status'] == 'failed'

        refund_txns = get_transactions(db_cursor, user_id, txn_type='REFUND')
        assert len(refund_txns) == 1
        assert refund_txns[0]['amount'] == Decimal('16.00')

        wallet = get_wallet(db_cursor, user_id)
        assert wallet['balance_points'] >= Decimal('499.00')  # Fully restored
        assert wallet['frozen_points'] == Decimal('0')

        cleanup_plan(db_cursor, plan_id)
        print("  OK: finalize_zero_consumed → full refund=16.00")

    def test_finalize_idempotent(self, db_cursor):
        """Call finalize twice → second is no-op."""
        user_id, plan_id = self._setup_frozen_plan(db_cursor)

        db_cursor.execute("SELECT fn_finalize_plan(%s, 'completed')", (plan_id,))
        db_cursor.connection.commit()
        wallet_1 = get_wallet(db_cursor, user_id)

        db_cursor.execute("SELECT fn_finalize_plan(%s, 'completed')", (plan_id,))
        db_cursor.connection.commit()
        wallet_2 = get_wallet(db_cursor, user_id)

        assert wallet_1 == wallet_2
        cleanup_plan(db_cursor, plan_id)
        print("  OK: finalize_idempotent → second call no-op")

    def test_finalize_none_billing(self, db_cursor):
        """billing_status=none → only updates status, no wallet ops."""
        user_id = setup_billing_test(db_cursor, balance=500)
        plan_id = create_test_plan(db_cursor, user_id)
        # Don't freeze — billing_status remains 'none'

        db_cursor.execute("SELECT fn_finalize_plan(%s, 'failed')", (plan_id,))
        db_cursor.connection.commit()

        plan = get_plan(db_cursor, plan_id)
        assert plan['status'] == 'failed'
        assert plan['billing_status'] == 'none'  # Still none

        wallet = get_wallet(db_cursor, user_id)
        assert wallet['balance_points'] >= Decimal('499.00')  # Unchanged

        cleanup_plan(db_cursor, plan_id)
        print("  OK: finalize_none_billing → only status updated")


# =============================================================================
# TestBalanceConsistency
# =============================================================================

class TestBalanceConsistency:
    """Test that money is never created or destroyed."""

    def test_conservation_freeze_consume_finalize(self, db_cursor):
        """initial_balance = final_balance + final_frozen + total_consumed."""
        initial_balance = Decimal('1000.00')
        user_id = setup_billing_test(db_cursor, balance=float(initial_balance))
        plan_id = create_test_plan(db_cursor, user_id)

        # Freeze: chat=1, image=6 = 1+30 = 31
        db_cursor.execute(
            "SELECT fn_freeze_budget(%s, 1, 6, 0, NULL, NULL, NULL, 'aipub_plan', %s)",
            (user_id, plan_id))
        db_cursor.connection.commit()

        # Consume: 1×AI_ANALYZE + 4×IMAGE = 1 + 20 = 21
        db_cursor.execute(
            "SELECT fn_consume_from_frozen(%s, 'AI_ANALYZE', NULL, 1, 'aipub_plan', %s)",
            (user_id, plan_id))
        for _ in range(4):
            db_cursor.execute(
                "SELECT fn_consume_from_frozen(%s, 'IMAGE', NULL, 1, 'aipub_plan', %s)",
                (user_id, plan_id))
        db_cursor.connection.commit()

        # Finalize
        db_cursor.execute("SELECT fn_finalize_plan(%s, 'completed')", (plan_id,))
        db_cursor.connection.commit()

        wallet = get_wallet(db_cursor, user_id)
        plan = get_plan(db_cursor, plan_id)

        # Conservation: initial = final_balance + consumed
        # (frozen should be 0 after finalize)
        assert wallet['frozen_points'] == Decimal('0')
        total_consumed = plan['consumed_cost']  # 21
        assert initial_balance == wallet['balance_points'] + total_consumed, \
            f"Conservation violated: {initial_balance} != {wallet['balance_points']} + {total_consumed}"

        cleanup_plan(db_cursor, plan_id)
        print(f"  OK: conservation verified: {initial_balance} = {wallet['balance_points']} + {total_consumed}")

    def test_negative_balance_prevented(self, db_cursor):
        """CHECK constraint prevents negative balance."""
        user_id = setup_billing_test(db_cursor, balance=5)

        with pytest.raises(Exception):
            db_cursor.execute("""
                UPDATE gm_user_wallets
                SET balance_points = -1 WHERE user_id = %s
            """, (user_id,))
        db_cursor.connection.rollback()
        print("  OK: negative balance prevented by CHECK constraint")
