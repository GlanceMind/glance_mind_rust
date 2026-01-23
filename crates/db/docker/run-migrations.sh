#!/bin/bash
# =============================================================================
# Run Diesel Migrations in Order
# =============================================================================
# This script runs all migration up.sql files in the correct order.
# It's designed to work with PostgreSQL's docker-entrypoint-initdb.d mechanism.
# =============================================================================

set -e

echo "=== Running GlanceMind Database Migrations ==="

# Get the migrations directory
MIGRATIONS_DIR="/docker-entrypoint-initdb.d/migrations"

# Find all up.sql files and sort them by directory name (which includes timestamp)
find "$MIGRATIONS_DIR" -name "up.sql" -type f | sort | while read migration; do
    migration_name=$(dirname "$migration" | xargs basename)
    echo "Applying migration: $migration_name"
    psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" -f "$migration"
done

echo "=== All migrations applied successfully ==="
