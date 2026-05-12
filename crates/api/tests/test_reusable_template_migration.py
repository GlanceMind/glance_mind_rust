"""
Rollback round-trip coverage for reusable reply template migration.

The test runs migration SQL inside a disposable PostgreSQL schema by rewriting
unqualified table names to that schema. It verifies down.sql preserves the
latest reusable-library prompt values back into campaign snapshots.
"""

import re
import uuid
from pathlib import Path

import psycopg2
import pytest

from conftest import DATABASE_URL


MIGRATION_DIR = (
    Path(__file__).resolve().parents[2]
    / "db"
    / "migrations"
    / "2026-05-06-000001_add_reusable_reply_templates"
)
CAMPAIGN_REPLY_TEMPLATE_IDS_MIGRATION_DIR = (
    Path(__file__).resolve().parents[2]
    / "db"
    / "migrations"
    / "2026-05-07-000001_add_campaign_reply_template_ids"
)


def _schema_sql(schema: str, sql: str) -> str:
    replacements = {
        "gm_users": f"{schema}.gm_users",
        "gm_campaigns": f"{schema}.gm_campaigns",
        "gm_campaign_templates": f"{schema}.gm_campaign_templates",
        "gm_reply_template_library": f"{schema}.gm_reply_template_library",
        "gm_resolved_campaign_templates": f"{schema}.gm_resolved_campaign_templates",
    }

    rewritten = sql
    for name, replacement in replacements.items():
        rewritten = re.sub(rf"\b{name}\b", replacement, rewritten)
    return rewritten


@pytest.fixture
def migration_schema():
    schema = f"gm_reusable_template_migration_{uuid.uuid4().hex[:12]}"
    try:
        conn = psycopg2.connect(DATABASE_URL)
    except psycopg2.OperationalError as exc:
        pytest.skip(f"PostgreSQL migration test database unavailable: {exc}")

    conn.autocommit = True
    cur = conn.cursor()
    cur.execute(f'CREATE SCHEMA "{schema}"')
    try:
        yield schema, conn
    finally:
        cur.execute(f'DROP SCHEMA IF EXISTS "{schema}" CASCADE')
        cur.close()
        conn.close()


def _init_pre_migration_schema(cur, schema: str):
    cur.execute(
        f"""
        CREATE TABLE {schema}.gm_users (
            id INTEGER PRIMARY KEY,
            email TEXT,
            username TEXT
        );

        CREATE TABLE {schema}.gm_campaigns (
            id INTEGER PRIMARY KEY,
            user_id INTEGER NOT NULL REFERENCES {schema}.gm_users(id),
            name TEXT NOT NULL
        );

        CREATE TABLE {schema}.gm_campaign_templates (
            id SERIAL PRIMARY KEY,
            campaign_id INTEGER NOT NULL REFERENCES {schema}.gm_campaigns(id),
            name VARCHAR(255),
            weight INTEGER NOT NULL DEFAULT 1,
            dm_prompt TEXT,
            reply_prompt TEXT,
            reply_post_prompt TEXT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ
        );

        INSERT INTO {schema}.gm_users (id, email, username)
        VALUES (1, 'migration@example.com', 'migration-user');

        INSERT INTO {schema}.gm_campaigns (id, user_id, name)
        VALUES (10, 1, 'Migration Campaign');

        INSERT INTO {schema}.gm_campaign_templates (
            id,
            campaign_id,
            name,
            weight,
            dm_prompt,
            reply_prompt,
            reply_post_prompt,
            created_at
        )
        VALUES (
            20,
            10,
            'Original Campaign Snapshot',
            42,
            'original dm',
            'original reply',
            'original post',
            NOW()
        );
        """
    )


def test_reusable_template_migration_down_preserves_latest_library_values(migration_schema):
    schema, conn = migration_schema
    cur = conn.cursor()
    _init_pre_migration_schema(cur, schema)
    cur.execute(f'SET search_path TO "{schema}", public')

    up_sql = _schema_sql(schema, (MIGRATION_DIR / "up.sql").read_text())
    down_sql = _schema_sql(schema, (MIGRATION_DIR / "down.sql").read_text())

    cur.execute(up_sql)
    cur.execute(
        f"""
        SELECT library_template_id
        FROM {schema}.gm_campaign_templates
        WHERE id = 20
        """
    )
    library_id = cur.fetchone()[0]
    assert library_id is not None

    cur.execute(
        f"""
        UPDATE {schema}.gm_reply_template_library
        SET
            name = 'Updated Library Name',
            dm_prompt = NULL,
            reply_prompt = 'updated reply',
            reply_post_prompt = NULL,
            updated_at = NOW()
        WHERE id = %s
        """,
        (library_id,),
    )

    cur.execute(
        f"""
        SELECT name, dm_prompt, reply_prompt, reply_post_prompt
        FROM {schema}.gm_resolved_campaign_templates
        WHERE id = 20
        """
    )
    resolved_before_down = cur.fetchone()
    assert resolved_before_down == (
        "Updated Library Name",
        None,
        "updated reply",
        None,
    )

    cur.execute(down_sql)

    cur.execute(
        f"""
        SELECT
            name,
            dm_prompt,
            reply_prompt,
            reply_post_prompt,
            EXISTS (
                SELECT 1
                FROM information_schema.columns
                WHERE table_schema = %s
                  AND table_name = 'gm_campaign_templates'
                  AND column_name = 'library_template_id'
            ) AS has_library_template_id
        FROM {schema}.gm_campaign_templates
        WHERE id = 20
        """,
        (schema,),
    )
    snapshot_after_down = cur.fetchone()
    assert snapshot_after_down == (
        "Updated Library Name",
        None,
        "updated reply",
        None,
        False,
    )

    cur.execute(
        """
        SELECT to_regclass(%s) IS NULL
        """,
        (f"{schema}.gm_reply_template_library",),
    )
    assert cur.fetchone()[0] is True
    cur.close()


def test_campaign_reply_template_ids_migration_backfills_and_reverts(migration_schema):
    schema, conn = migration_schema
    cur = conn.cursor()
    _init_pre_migration_schema(cur, schema)
    cur.execute(f'SET search_path TO "{schema}", public')

    reusable_up_sql = _schema_sql(schema, (MIGRATION_DIR / "up.sql").read_text())
    reply_ids_up_sql = _schema_sql(
        schema, (CAMPAIGN_REPLY_TEMPLATE_IDS_MIGRATION_DIR / "up.sql").read_text()
    )
    reply_ids_down_sql = _schema_sql(
        schema, (CAMPAIGN_REPLY_TEMPLATE_IDS_MIGRATION_DIR / "down.sql").read_text()
    )

    cur.execute(reusable_up_sql)
    cur.execute(f'DROP INDEX IF EXISTS "{schema}".idx_campaign_templates_campaign_library_unique')
    cur.execute(
        f"""
        SELECT library_template_id
        FROM {schema}.gm_campaign_templates
        WHERE id = 20
        """
    )
    migrated_library_id = cur.fetchone()[0]
    assert migrated_library_id is not None

    cur.execute(
        f"""
        INSERT INTO {schema}.gm_reply_template_library (
            id,
            user_id,
            name,
            description,
            weight,
            dm_prompt,
            reply_prompt,
            reply_post_prompt,
            usage_count,
            created_at,
            updated_at
        )
        VALUES
            (101, 1, 'Reusable A', NULL, 1, NULL, 'reply a', NULL, 0, NOW(), NULL),
            (102, 1, 'Reusable B', NULL, 1, NULL, 'reply b', NULL, 0, NOW(), NULL),
            (103, 1, 'Reusable C', NULL, 1, NULL, 'reply c', NULL, 0, NOW(), NULL);

        INSERT INTO {schema}.gm_campaign_templates (
            id,
            campaign_id,
            library_template_id,
            name,
            weight,
            dm_prompt,
            reply_prompt,
            reply_post_prompt,
            created_at
        )
        VALUES
            (21, 10, 101, 'Assignment A low duplicate', 10, NULL, 'reply a low', NULL, NOW()),
            (22, 10, 102, 'Assignment B tie first', 30, NULL, 'reply b', NULL, NOW()),
            (23, 10, 101, 'Assignment A winning duplicate', 40, NULL, 'reply a high', NULL, NOW()),
            (24, 10, 103, 'Assignment C tie second', 30, NULL, 'reply c', NULL, NOW());
        """
    )

    cur.execute(reply_ids_up_sql)
    cur.execute(f"SELECT reply_template_ids FROM {schema}.gm_campaigns WHERE id = 10")
    assert cur.fetchone()[0] == [migrated_library_id, 101, 102, 103]

    cur.execute(reply_ids_down_sql)
    cur.execute(
        """
        SELECT EXISTS (
            SELECT 1
            FROM information_schema.columns
            WHERE table_schema = %s
              AND table_name = 'gm_campaigns'
              AND column_name = 'reply_template_ids'
        )
        """,
        (schema,),
    )
    assert cur.fetchone()[0] is False
    cur.close()
