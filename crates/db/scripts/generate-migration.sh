#!/bin/bash
#
# Generate a new migration
#
# Usage: ./scripts/generate-migration.sh <migration_name>
#
# Example: ./scripts/generate-migration.sh add_user_preferences
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

if [ -z "$1" ]; then
    echo -e "${RED}Error: Migration name required!${NC}"
    echo "Usage: $0 <migration_name>"
    echo "Example: $0 add_user_preferences"
    exit 1
fi

MIGRATION_NAME="$1"

echo "==================================="
echo "  Generate New Migration"
echo "==================================="
echo

cd "$PROJECT_DIR"

# Generate migration
echo "Generating migration: $MIGRATION_NAME"
diesel migration generate "$MIGRATION_NAME"

# Find the new migration directory
NEW_MIGRATION=$(ls -td migrations/*"$MIGRATION_NAME"* 2>/dev/null | head -1)

if [ -n "$NEW_MIGRATION" ]; then
    echo -e "${GREEN}Migration created: $NEW_MIGRATION${NC}"
    echo
    echo "Next steps:"
    echo "  1. Edit $NEW_MIGRATION/up.sql"
    echo "  2. Edit $NEW_MIGRATION/down.sql"
    echo "  3. Run: diesel migration run"
    echo "  4. Run: ./scripts/export-schema.sh --sync"
else
    echo -e "${RED}Error: Migration not found after creation${NC}"
    exit 1
fi
