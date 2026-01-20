# Baseline Migration

## Overview

This is the **baseline migration** for GlanceMind database, containing the complete database schema as of 2026-01-20.

## Purpose

### New Environment (Empty Database)

Simply run `diesel migration run` to automatically create all table structures.

### Existing Environment (Production/Staging)

Before running new migrations, you need to mark the baseline as applied:

```bash
# Method 1: Use the script
./scripts/migrate-existing-db.sh

# Method 2: Execute SQL directly
psql $DATABASE_URL -f scripts/migrate_existing_db.sql
```

This will:

1. Delete old records in `__diesel_schema_migrations` table (41 records)
2. Insert the baseline migration record
3. Not affect any business data

## Technical Details

- All `CREATE TABLE` statements use `IF NOT EXISTS` for safe re-runs
- All `CREATE INDEX` statements use `IF NOT EXISTS`
- All `CREATE FUNCTION` statements use `OR REPLACE`

## Included Tables

### User Related

- `gm_users` - User accounts
- `gm_admin_users` - Admin users
- `gm_user_wallets` - User wallets
- `gm_wallet_transactions` - Wallet transaction records
- `gm_login_logs` - Login logs
- `gm_email_verifications` - Email verifications

### Campaign Related

- `gm_campaigns` - Campaigns/Tasks
- `gm_campaign_accounts` - Campaign linked accounts
- `gm_campaign_templates` - Campaign templates

### Social Accounts

- `gm_social_groups` - Social account groups
- `gm_social_accounts` - Social accounts

### Crawler Related

- `gm_crawler_tasks` - Crawler tasks
- `gm_crawler_results` - Crawler results

### Agent Related

- `gm_agent_videos` - Agent videos
- `gm_agent_comments` - Agent comments
- `gm_agent_instagram_*` - Instagram related tables
- `gm_agent_twitter_*` - Twitter related tables
- `gm_agent_reddit_*` - Reddit related tables

### Configuration

- `gm_platforms` - Platform configuration
- `gm_regions` - Region configuration
- `gm_ai_models` - AI model configuration
- `gm_pricing_rules` - Pricing rules

### Others

- `gm_referrals` - Referral relationships
- `gm_referral_earnings` - Referral earnings
- `gm_promo_codes` - Promotional codes
- `gm_video_generation_tasks` - Video generation tasks
- `gm_upload_tasks` - Upload tasks

## Archived Migrations

The original 41 migrations are archived in `archive/migrations_legacy/` for reference only and are no longer in use.
