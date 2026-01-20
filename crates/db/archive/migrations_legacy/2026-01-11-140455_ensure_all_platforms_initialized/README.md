# Platform Initialization Migration

## 📋 Overview

This migration ensures all supported social media platforms are properly initialized in the database with their corresponding regions. It is designed to be **idempotent** and safe to run on both fresh and existing databases.

## 🎯 Purpose

- Ensure REDDIT, TIKTOK, INSTAGRAM, and TWITTER platforms exist
- Initialize standard regions for each platform: GLOBAL, US, UK, JP, CN, EU
- Safe for production deployment (uses `ON CONFLICT DO NOTHING`)
- Can be run multiple times without errors

## 📊 What This Migration Does

### Step 1: Platform Initialization

Ensures the following platforms exist:

| Platform | Display Name | Status |
|----------|-------------|--------|
| REDDIT | Reddit | Active |
| TIKTOK | TikTok | Active |
| INSTAGRAM | Instagram | Active |
| TWITTER | Twitter / X | Active |

### Step 2: Region Initialization

For each platform, ensures these regions exist:

| Code | Display Name | Description |
|------|--------------|-------------|
| GLOBAL | Global | Worldwide content |
| US | United States | US-specific content |
| UK | United Kingdom | UK-specific content |
| JP | Japan | Japan-specific content |
| CN | China | China-specific content |
| EU | European Union | EU-specific content |
| KR | South Korea | South Korea-specific content |
| MY | Malaysia | Malaysia-specific content |
| SG | Singapore | Singapore-specific content |
| AE | United Arab Emirates | UAE/Dubai-specific content |
| DE | Germany | Germany-specific content |

## 🔒 Safety Features

### Idempotent Operations
- Uses `ON CONFLICT (name) DO NOTHING` for platforms
- Uses `NOT EXISTS` checks for regions
- Safe to run multiple times

### No Data Loss
- Will not overwrite existing platforms
- Will not duplicate regions
- Will not affect existing campaigns or social accounts

### Production Ready
- Can be deployed directly to production
- Works on both empty and populated databases
- No manual intervention required

## 🚀 Deployment

### Prerequisites
```bash
# Ensure diesel CLI is installed
cargo install diesel_cli --no-default-features --features postgres
```

### Run Migration
```bash
cd /path/to/glance_mind_api
diesel migration run
```

### Verify Migration
```bash
# Check migration status
diesel migration list

# Verify platforms were created
psql -d your_database -c "SELECT * FROM gm_platforms WHERE name IN ('REDDIT', 'TIKTOK', 'INSTAGRAM', 'TWITTER');"

# Verify regions were created
psql -d your_database -c "SELECT p.name, COUNT(r.id) as region_count FROM gm_platforms p LEFT JOIN gm_regions r ON r.platform_id = p.id WHERE p.name IN ('REDDIT', 'TIKTOK', 'INSTAGRAM', 'TWITTER') GROUP BY p.name;"
```

Expected output:
```
    name    | region_count 
------------+-------------
 REDDIT     |          11
 TIKTOK     |          11
 INSTAGRAM  |          11
 TWITTER    |          11
```

## 🔄 Rollback

**⚠️ WARNING**: Rolling back this migration will:
- Delete all 4 platforms (REDDIT, TIKTOK, INSTAGRAM, TWITTER)
- Delete all associated regions
- Cascade delete all related data:
  - Campaigns using these platforms
  - Social accounts on these platforms
  - All collected posts, comments, videos, etc.

**Only rollback in development environments!**

```bash
diesel migration revert
```

## 📝 Notes

### Relationship to Other Migrations

This migration supersedes:
- `2026-01-11-121351_add_instagram_twitter_platforms` - Instagram and Twitter initialization

The older migration can remain in place as this migration is idempotent and will not cause conflicts.

### Database Constraints

Relies on these constraints:
- `gm_platforms.name` - UNIQUE constraint (for ON CONFLICT)
- `gm_regions` - No unique constraint on (platform_id, code), uses NOT EXISTS check

### Platform Name Format

Platform names in database:
- Stored as: UPPERCASE (e.g., 'REDDIT', 'TIKTOK')
- Sent from frontend: lowercase (e.g., 'reddit', 'tiktok')
- Agent expects: lowercase (e.g., 'reddit', 'tiktok')

The conversion happens in the frontend:
```typescript
{ [selectedPlatform.name.toLowerCase()]: formData.searchOptions }
```

## 🧪 Testing

### Test on Local Database
```bash
# Create test database
createdb glance_mind_test

# Run migration
DATABASE_URL=postgres://user:pass@localhost/glance_mind_test diesel migration run

# Verify
psql glance_mind_test -c "SELECT name, display_name FROM gm_platforms ORDER BY name;"

# Test idempotency - run again
DATABASE_URL=postgres://user:pass@localhost/glance_mind_test diesel migration run

# Should show no errors and same results
```

### Expected Results

After migration:
- 4 platforms created (or already exist)
- 44 total regions created (11 regions × 4 platforms)
- All platforms active and ready for use

## ✅ Production Deployment Checklist

- [ ] Review migration SQL files
- [ ] Test migration on staging environment
- [ ] Verify no conflicts with existing data
- [ ] Backup production database
- [ ] Run migration during maintenance window (recommended)
- [ ] Verify all platforms and regions created
- [ ] Test frontend platform selection
- [ ] Test campaign creation with new platforms
- [ ] Monitor logs for any issues

## 🔗 Related Files

- Frontend: `glance_mind_front/src/utils/platformSearchConfig.ts`
- Frontend: `glance_mind_front/src/utils/advancedSearchOptions.ts`
- Agent: `glance_mind_worker/glance_mind_agent/src/platforms/*/workflow.py`
- Scheduler: `glance_mind_worker/glance_mind_scheduler/src/main.rs`

## 📞 Support

If you encounter issues:
1. Check migration status: `diesel migration list`
2. Check database logs for errors
3. Verify database user has INSERT permissions
4. Ensure unique constraint exists on `gm_platforms.name`

## 📅 Created

- **Date**: 2026-01-11
- **Version**: v1.0
- **Status**: Ready for Production ✅
