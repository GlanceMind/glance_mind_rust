#!/bin/bash
# =============================================================================
# Run Diesel Migrations with dependency-aware retries
# =============================================================================
# Some historical migrations are not strictly timestamp-ordered. To keep the
# test database bootstrapping resilient, apply each migration in its own
# transaction and retry deferred files in a later pass once their dependencies
# exist.
# =============================================================================

set -uo pipefail

echo "=== Running GlanceMind Database Migrations ==="

MIGRATIONS_DIR="/docker-entrypoint-initdb.d/migrations"

mapfile -t pending_migrations < <(find "$MIGRATIONS_DIR" -name "up.sql" -type f | sort)

pass=1
while [ ${#pending_migrations[@]} -gt 0 ]; do
    echo "--- Migration pass ${pass} (${#pending_migrations[@]} files pending) ---"
    progress=0
    next_pending=()

    for migration in "${pending_migrations[@]}"; do
        migration_name=$(basename "$(dirname "$migration")")
        echo "Applying migration: $migration_name"

        if psql -v ON_ERROR_STOP=1 -1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" -f "$migration"; then
            progress=$((progress + 1))
        else
            echo "Deferring migration for later pass: $migration_name"
            next_pending+=("$migration")
        fi
    done

    if [ ${#next_pending[@]} -eq 0 ]; then
        break
    fi

    if [ "$progress" -eq 0 ]; then
        echo "ERROR: No migration progress made in pass ${pass}."
        echo "Remaining migrations:"
        for migration in "${next_pending[@]}"; do
            echo "  - $(basename "$(dirname "$migration")")"
        done
        exit 1
    fi

    pending_migrations=("${next_pending[@]}")
    pass=$((pass + 1))
done

echo "=== All migrations applied successfully ==="
