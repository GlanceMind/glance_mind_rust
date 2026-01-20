#!/bin/bash
#
# Migrate existing database to the new baseline migration system
#
# Purpose: Run on existing production/staging databases
#
# Usage:
#   ./scripts/migrate-existing-db.sh                    # Use DATABASE_URL from .env
#   DATABASE_URL=xxx ./scripts/migrate-existing-db.sh   # Specify database URL
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "=============================================="
echo "  GlanceMind - Existing Database Migration"
echo "=============================================="
echo

# Load environment
if [ -z "$DATABASE_URL" ]; then
    if [ -f "$PROJECT_DIR/.env" ]; then
        export $(grep -v '^#' "$PROJECT_DIR/.env" | xargs)
    fi
fi

if [ -z "$DATABASE_URL" ]; then
    echo -e "${RED}Error: DATABASE_URL is not set!${NC}"
    echo "Please set DATABASE_URL environment variable or configure it in .env file"
    exit 1
fi

# Display target database (hide password)
DB_DISPLAY=$(echo "$DATABASE_URL" | sed 's/:\/\/[^:]*:[^@]*@/:\/\/***:***@/')
echo "Target database: $DB_DISPLAY"
echo

# Confirm operation
echo -e "${YELLOW}Warning: This operation will:${NC}"
echo "  1. Delete all old records in __diesel_schema_migrations table"
echo "  2. Insert new baseline migration record"
echo "  3. NOT affect any business data"
echo
read -p "Continue? (y/N): " confirm

if [ "$confirm" != "y" ] && [ "$confirm" != "Y" ]; then
    echo "Operation cancelled"
    exit 0
fi

echo
echo "Running migration..."
echo

# Execute SQL script
psql "$DATABASE_URL" -f "$SCRIPT_DIR/migrate_existing_db.sql"

echo
echo -e "${GREEN}Migration completed!${NC}"
echo
echo "Next steps:"
echo "  1. Run 'diesel migration run' to apply any new migrations"
echo "  2. Run './scripts/export-schema.sh' to ensure schema is synced"
