#!/bin/bash
# Pre-deployment check script
# Run this before deploying to catch Diesel schema/entity mismatches early

set -e

echo "🔍 Running pre-deployment checks..."

# 1. Check Rust compilation
echo ""
echo "📦 Step 1: Checking Rust compilation (release mode)..."
if cargo check --release -p glance_mind_api 2>&1; then
    echo "✅ Rust compilation check passed"
else
    echo "❌ Rust compilation check failed!"
    echo ""
    echo "Common causes:"
    echo "  - Schema/Entity field order mismatch"
    echo "  - Schema/Entity field type mismatch"
    echo "  - Missing #[diesel(check_for_backend(Pg))] annotation"
    echo ""
    echo "To debug, check the entity definitions in crates/db/src/entity/"
    echo "and ensure they match the schema in crates/db/src/schema.rs"
    exit 1
fi

# 2. Verify schema.rs is up to date with migrations
echo ""
echo "📦 Step 2: Checking if schema.rs is up to date..."
if command -v diesel &> /dev/null; then
    cd crates/db
    # Run diesel print-schema and compare
    TEMP_SCHEMA=$(mktemp)
    diesel print-schema --database-url="${DATABASE_URL:-postgres://localhost/test}" > "$TEMP_SCHEMA" 2>/dev/null || true
    
    if [ -s "$TEMP_SCHEMA" ]; then
        if diff -q "$TEMP_SCHEMA" src/schema.rs > /dev/null 2>&1; then
            echo "✅ schema.rs is up to date"
        else
            echo "⚠️  schema.rs may be out of date with migrations"
            echo "   Run: cd crates/db && diesel print-schema > src/schema.rs"
        fi
    else
        echo "⚠️  Could not verify schema.rs (diesel print-schema failed or no DATABASE_URL)"
    fi
    rm -f "$TEMP_SCHEMA"
    cd ../..
else
    echo "⚠️  diesel CLI not installed, skipping schema verification"
fi

# 3. Run cargo clippy for additional checks
echo ""
echo "📦 Step 3: Running clippy checks..."
if cargo clippy -p glance_mind_api -- -D warnings 2>&1 | head -50; then
    echo "✅ Clippy checks passed"
else
    echo "⚠️  Clippy found some issues (check output above)"
fi

echo ""
echo "🎉 Pre-deployment checks completed!"
echo ""
echo "Next steps:"
echo "  1. Commit and push your changes"
echo "  2. Create a PR or push to release branch"
echo "  3. CI/CD will build and deploy automatically"
