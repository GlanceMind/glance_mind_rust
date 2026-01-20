#!/bin/bash
#
# Export schema from database and optionally sync to services
#
# Usage: 
#   ./scripts/export-schema.sh           # Export only
#   ./scripts/export-schema.sh --sync    # Export and sync to services
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

echo "==================================="
echo "  GlanceMind Schema Export Tool"
echo "==================================="
echo

# Load environment
if [ -f "$PROJECT_DIR/.env" ]; then
    export $(grep -v '^#' "$PROJECT_DIR/.env" | xargs)
fi

if [ -z "$DATABASE_URL" ]; then
    echo -e "${RED}Error: DATABASE_URL is not set!${NC}"
    echo "Create a .env file with DATABASE_URL or export it."
    exit 1
fi

echo "Exporting schema from database..."
cd "$PROJECT_DIR"
diesel print-schema > src/schema.rs

LINES=$(wc -l < src/schema.rs)
echo -e "${GREEN}Schema exported: $LINES lines${NC}"

# Sync if requested
if [ "$1" == "--sync" ]; then
    echo
    "$SCRIPT_DIR/sync-schema.sh"
fi
