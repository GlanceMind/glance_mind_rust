#!/bin/bash
#
# Sync schema.rs to all dependent services
#
# Usage: ./scripts/sync-schema.sh
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
SCHEMA_FILE="$PROJECT_DIR/src/schema.rs"

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "==================================="
echo "  GlanceMind Schema Sync Tool"
echo "==================================="
echo

# Check if schema.rs exists
if [ ! -f "$SCHEMA_FILE" ]; then
    echo -e "${RED}Error: src/schema.rs not found!${NC}"
    echo "Run 'diesel print-schema > src/schema.rs' first."
    exit 1
fi

# Target services (relative to parent directory)
declare -a TARGETS=(
    "../glance_mind_api/src/schema.rs"
    "../glance_mind_worker/glance_mind_scheduler/src/schema.rs"
)

echo "Source: $SCHEMA_FILE"
echo "Schema file size: $(wc -l < "$SCHEMA_FILE") lines"
echo

# Sync to each target
for target in "${TARGETS[@]}"; do
    target_path="$PROJECT_DIR/$target"
    target_dir="$(dirname "$target_path")"
    
    if [ -d "$target_dir" ]; then
        echo -n "Syncing to $target... "
        cp "$SCHEMA_FILE" "$target_path"
        echo -e "${GREEN}Done${NC}"
    else
        echo -e "${YELLOW}Skipping $target (directory not found)${NC}"
    fi
done

echo
echo -e "${GREEN}Schema sync completed!${NC}"
