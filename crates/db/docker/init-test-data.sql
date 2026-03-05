-- =============================================================================
-- GlanceMind Test Database Mock Data
-- =============================================================================
-- This script initializes base data required for integration testing.
-- It mirrors production data structure to ensure tests are realistic.
-- =============================================================================

-- ============================================================================
-- 1. Platforms
-- IMPORTANT: IDs MUST match glance_mind_protocol/proto/common.proto Platform enum
-- Source of truth: common.proto - DO NOT change IDs here without updating proto first!
-- ============================================================================
INSERT INTO gm_platforms (id, name, display_name, base_url, page_size, is_active) VALUES
(1, 'reddit', 'Reddit', 'https://www.reddit.com', 25, true),       -- PLATFORM_REDDIT = 1
(2, 'tiktok', 'TikTok', 'https://www.tiktok.com', 20, true),       -- PLATFORM_TIKTOK = 2
(3, 'facebook', 'Facebook', 'https://www.facebook.com', 20, true), -- PLATFORM_FACEBOOK = 3
(4, 'instagram', 'Instagram', 'https://www.instagram.com', 12, true), -- PLATFORM_INSTAGRAM = 4
(5, 'twitter', 'Twitter/X', 'https://twitter.com', 20, true),      -- PLATFORM_TWITTER = 5
(6, 'youtube', 'YouTube', 'https://www.youtube.com', 20, true)     -- PLATFORM_YOUTUBE = 6
ON CONFLICT (id) DO UPDATE SET
    display_name = EXCLUDED.display_name,
    base_url = EXCLUDED.base_url,
    page_size = EXCLUDED.page_size;

-- Reset sequence
SELECT setval('platforms_id_seq', (SELECT MAX(id) FROM gm_platforms));

-- ============================================================================
-- 2. Regions (common regions for all platforms)
-- ============================================================================
INSERT INTO gm_regions (id, platform_id, code, name, display_name, is_active) VALUES
-- TikTok regions
(1, 2, 'US', 'United States', 'United States', true),
(2, 2, 'GB', 'United Kingdom', 'United Kingdom', true),
(3, 2, 'JP', 'Japan', 'Japan', true),
(4, 2, 'TW', 'Taiwan', 'Taiwan', true),
-- Reddit regions (Reddit doesn't have regions, use global)
(5, 1, 'GLOBAL', 'Global', 'Global', true),
-- Facebook regions
(6, 3, 'US', 'United States', 'United States', true),
(7, 3, 'GB', 'United Kingdom', 'United Kingdom', true),
-- Instagram regions
(8, 4, 'US', 'United States', 'United States', true),
(9, 4, 'GB', 'United Kingdom', 'United Kingdom', true),
-- Twitter regions
(10, 5, 'US', 'United States', 'United States', true),
(11, 5, 'GLOBAL', 'Global', 'Global', true)
ON CONFLICT (platform_id, code) DO UPDATE SET
    display_name = EXCLUDED.display_name;

SELECT setval('regions_id_seq', (SELECT MAX(id) FROM gm_regions));

-- ============================================================================
-- 3. Pricing Rules (cost per action per platform)
-- ============================================================================
INSERT INTO gm_pricing_rules (id, platform_id, action_type, cost_points) VALUES
-- Reddit pricing
(1, 1, 'SCAN_POST', 0.50),
(2, 1, 'AI_ANALYZE', 1.00),
(3, 1, 'REPLY_COMMENT', 2.00),
-- TikTok pricing
(4, 2, 'SCAN_POST', 0.50),
(5, 2, 'AI_ANALYZE', 1.00),
(6, 2, 'REPLY_COMMENT', 2.00),
-- Facebook pricing
(7, 3, 'SCAN_POST', 0.50),
(8, 3, 'AI_ANALYZE', 1.00),
(9, 3, 'REPLY_COMMENT', 2.00),
-- Instagram pricing
(10, 4, 'SCAN_POST', 0.50),
(11, 4, 'AI_ANALYZE', 1.00),
(12, 4, 'REPLY_COMMENT', 2.00),
-- Twitter pricing
(13, 5, 'SCAN_POST', 0.50),
(14, 5, 'AI_ANALYZE', 1.00),
(15, 5, 'REPLY_COMMENT', 2.00),
-- Global pricing (platform_id IS NULL) for billing system
(16, NULL, 'AI_ANALYZE', 1.00),
(17, NULL, 'IMAGE', 5.00),
(18, NULL, 'VIDEO_GENERATE', 200.00)
ON CONFLICT (action_type, platform_id) DO UPDATE SET
    cost_points = EXCLUDED.cost_points;

SELECT setval('pricing_rules_id_seq', (SELECT MAX(id) FROM gm_pricing_rules));

-- ============================================================================
-- 4. AI Models (for campaign configuration)
-- ============================================================================
INSERT INTO gm_ai_models (id, name, provider, model_key, model_type, cost_multiplier, is_active) VALUES
(1, 'Gemini 3.1 Pro', 'google', 'gemini-3.1-pro-preview', 'chat', 0.5, true),
(2, 'GPT-5.2', 'openai', 'gpt-5.2', 'chat', 1.0, true),
(3, 'Claude Haiku Thinking', 'anthropic', 'claude-haiku-4-5-20251001-thinking', 'chat', 0.8, true),
(4, 'Grok 4', 'xai', 'grok-4', 'chat', 1.5, true),
(5, 'Veo-2', 'google', 'veo-2', 'video', 5.0, true),
(6, 'Jimeng 3.0 720P', 'jimeng', 'jimeng-video-3.0-720p', 'video', 2.0, true),
(7, 'Jimeng 3.0 1080P', 'jimeng', 'jimeng-video-3.0-1080p', 'video', 3.0, true)
ON CONFLICT (id) DO UPDATE SET
    name = EXCLUDED.name,
    provider = EXCLUDED.provider,
    model_key = EXCLUDED.model_key,
    cost_multiplier = EXCLUDED.cost_multiplier;

SELECT setval('ai_models_id_seq', (SELECT MAX(id) FROM gm_ai_models));

-- ============================================================================
-- 5. Test Users
-- ============================================================================
-- Password hash for 'TestPassword123!' using bcrypt
-- To regenerate: python3 -c "import bcrypt; print(bcrypt.hashpw(b'TestPassword123!', bcrypt.gensalt(12)).decode())"
INSERT INTO gm_users (id, email, username, password_hash, full_name, role, status, is_active, invite_code) VALUES
(1, 'admin@glancemind.test', 'admin', '$2b$12$Ikc.R4FMMGahbGhfHlLl4.PciMiV37qXfHpPNCjGGQg/yOEgk7k/e', 'Test Admin', 'admin', 'ACTIVE', true, 'ADMIN001'),
(2, 'user@glancemind.test', 'testuser', '$2b$12$Ikc.R4FMMGahbGhfHlLl4.PciMiV37qXfHpPNCjGGQg/yOEgk7k/e', 'Test User', 'user', 'ACTIVE', true, 'USER0001'),
(3, 'premium@glancemind.test', 'premiumuser', '$2b$12$Ikc.R4FMMGahbGhfHlLl4.PciMiV37qXfHpPNCjGGQg/yOEgk7k/e', 'Premium User', 'user', 'ACTIVE', true, 'PREM0001')
ON CONFLICT (id) DO UPDATE SET
    email = EXCLUDED.email,
    username = EXCLUDED.username,
    password_hash = EXCLUDED.password_hash;

SELECT setval('users_id_seq', (SELECT MAX(id) FROM gm_users));

-- ============================================================================
-- 6. User Wallets (with test balances)
-- ============================================================================
INSERT INTO gm_user_wallets (user_id, balance_points, frozen_points, deposit_cny, deposit_usd) VALUES
(1, 100000.00, 0.00, 10000.00, 1500.00),  -- Admin: large balance
(2, 10000.00, 0.00, 1000.00, 150.00),     -- Regular user: moderate balance
(3, 50000.00, 5000.00, 5000.00, 750.00)   -- Premium user: has frozen points
ON CONFLICT (user_id) DO UPDATE SET
    balance_points = EXCLUDED.balance_points,
    frozen_points = EXCLUDED.frozen_points;

-- ============================================================================
-- 7. Admin Users
-- ============================================================================
INSERT INTO gm_admin_users (id, username, email, password_hash, full_name, role, is_active) VALUES
(1, 'superadmin', 'superadmin@glancemind.test', '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewKkxoCyJwcF6hCa', 'Super Admin', 'super_admin', true)
ON CONFLICT (id) DO UPDATE SET
    email = EXCLUDED.email;

SELECT setval('gm_admin_users_id_seq', (SELECT MAX(id) FROM gm_admin_users));

-- ============================================================================
-- 8. Social Groups (for account management)
-- ============================================================================
-- Groups must exist for all platforms with campaigns that need device-based queries
INSERT INTO gm_social_groups (id, user_id, platform_id, group_name) VALUES
(1, 2, 2, 'TikTok Main Group'),       -- For TikTok campaigns
(2, 2, 1, 'Reddit Marketing'),        -- For Reddit campaigns
(3, 3, 4, 'Instagram Business'),      -- For Instagram campaigns
(4, 2, 3, 'Facebook Business'),       -- For Facebook campaigns
(5, 2, 5, 'Twitter Engagement')       -- For Twitter campaigns
ON CONFLICT (id) DO NOTHING;

SELECT setval('social_groups_id_seq', (SELECT MAX(id) FROM gm_social_groups));

-- ============================================================================
-- 9. Social Accounts (mock accounts for testing)
-- ============================================================================
-- NOTE: device_id is critical for get_comments_by_device API
-- Test device IDs: test_device_001 (TikTok/Facebook), test_device_002 (Instagram), test_device_003 (Reddit/Twitter)
INSERT INTO gm_social_accounts (id, user_id, platform_id, username, group_id, status, health_score, cookie, daily_max_replies, device_id, profile_name) VALUES
(1, 2, 2, 'test_tiktok_1', 1, 'ACTIVE', 100, '{}', 50, 'test_device_001', 'TikTok Profile 1'),
(2, 2, 2, 'test_tiktok_2', 1, 'ACTIVE', 95, '{}', 50, 'test_device_001', 'TikTok Profile 2'),
(3, 2, 1, 'test_reddit_1', 2, 'ACTIVE', 100, '{}', 30, 'test_device_003', 'Reddit Profile'),
(4, 3, 4, 'test_instagram_1', 3, 'ACTIVE', 100, '{}', 40, 'test_device_002', 'Instagram Profile'),
-- Additional accounts for Facebook and Twitter
(5, 2, 3, 'test_facebook_1', 4, 'ACTIVE', 100, '{}', 40, 'test_device_001', 'Facebook Profile'),
(6, 2, 5, 'test_twitter_1', 5, 'ACTIVE', 100, '{}', 40, 'test_device_003', 'Twitter Profile')
ON CONFLICT (id) DO NOTHING;

SELECT setval('social_accounts_id_seq', (SELECT MAX(id) FROM gm_social_accounts));

-- ============================================================================
-- 10. Test Campaigns (various states for testing)
-- ============================================================================
-- IMPORTANT: Create campaigns for ALL platforms to ensure comprehensive testing
-- CRITICAL: social_group_id must be set for get_comments_by_device API to work!

-- Active campaign for scheduler testing
INSERT INTO gm_campaigns (
    id, user_id, name, status, platform_id, region_id, ai_model_id,
    product_prompt, schedule_type, schedule_config, keyword, max_scan_count,
    budget_cap, auto_like, auto_follow, auto_dm, is_frozen,
    pending_consumption, actual_consumption, total_scanned, social_group_id,
    auto_reply_comments, auto_reply_post
) VALUES
-- Platform 1: Reddit - Active (social_group_id = 2)
(1, 2, 'Reddit Marketing Campaign', 'ACTIVE', 1, 5, 1,
 'AI-powered marketing assistant for tech products.', 'INTERVAL', '{"interval_seconds": 7200}',
 'marketing automation, AI tools', 30, 500.00, false, false, false, true, 0.00, 0.00, 0, 2,
 true, false),

-- Platform 2: TikTok - Active (social_group_id = 1)
(2, 2, 'TikTok Travel Promo', 'ACTIVE', 2, 1, 1,
 'We sell premium travel packages to China.', 'INTERVAL', '{"interval_seconds": 3600}',
 'travel china, china tour', 50, 1000.00, true, true, false, true, 0.00, 0.00, 0, 1,
 true, true),

-- Platform 3: Facebook - Active (social_group_id = 4)
(3, 2, 'Facebook Brand Campaign', 'ACTIVE', 3, 6, 1,
 'Premium fashion brand marketing.', 'INTERVAL', '{"interval_seconds": 3600}',
 'fashion trends, style tips', 40, 800.00, true, true, true, true, 0.00, 0.00, 0, 4,
 true, true),

-- Platform 4: Instagram - Active (social_group_id = 3)
(4, 3, 'Instagram Influencer Campaign', 'ACTIVE', 4, 8, 2,
 'Luxury lifestyle brand promotion.', 'INTERVAL', '{"interval_seconds": 1800}',
 'luxury lifestyle, premium products', 60, 1200.00, true, true, true, true, 0.00, 0.00, 0, 3,
 true, true),

-- Platform 5: Twitter - Active (social_group_id = 5)
(5, 2, 'Twitter Tech Engagement', 'ACTIVE', 5, 10, 1,
 'SaaS product marketing on Twitter/X.', 'INTERVAL', '{"interval_seconds": 3600}',
 'saas tools, productivity', 35, 600.00, true, false, false, true, 0.00, 0.00, 0, 5,
 true, false),

-- Draft campaign (should not be processed by scheduler)
(6, 2, 'Draft TikTok Campaign', 'DRAFT', 2, 1, 1,
 'Draft product description.', 'IMMEDIATE', NULL,
 'test keyword', 20, 200.00, true, true, true, false, 0.00, 0.00, 0, 1,
 false, false),

-- Completed campaign (for history testing)
(7, 3, 'Completed Instagram Campaign', 'COMPLETED', 4, 8, 2,
 'Fashion brand promotion - completed.', 'INTERVAL', '{"interval_seconds": 1800}',
 'fashion style', 100, 2000.00, true, true, true, true, 0.00, 1500.00, 500, 3,
 true, true),

-- Reddit with advanced search_options (social_group_id = 2)
(8, 2, 'Reddit Rust Community', 'ACTIVE', 1, 5, 1,
 'Rust programming tools marketing.', 'INTERVAL', '{"interval_seconds": 3600}',
 'programming rust, rust lang', 40, 800.00, false, false, false, true, 0.00, 0.00, 0, 2,
 true, false)
ON CONFLICT (id) DO NOTHING;

-- Update campaign 8 with search_options separately
UPDATE gm_campaigns SET search_options = '{"subreddit": "rust", "sort": "hot", "time_filter": "week"}'::jsonb WHERE id = 8;

SELECT setval('campaigns_id_seq', (SELECT MAX(id) FROM gm_campaigns));

-- ============================================================================
-- 11. Campaign Templates (for reply generation)
-- ============================================================================
INSERT INTO gm_campaign_templates (id, campaign_id, weight, reply_prompt, dm_prompt) VALUES
(1, 1, 100, 'Provide helpful marketing tips and mention our AI tools when relevant.', NULL),
(2, 2, 100, 'Reply to travel-related comments with helpful information about China tourism.', 'Send a friendly DM about our travel services.'),
(3, 3, 100, 'Engage with fashion enthusiasts and share style tips.', 'Share exclusive fashion updates.'),
(4, 4, 100, 'Share luxury lifestyle content and engage authentically.', 'Welcome to our premium community!'),
(5, 5, 100, 'Share tech insights and productivity tips.', NULL),
(6, 8, 100, 'Share programming insights and engage with the Rust community.', NULL)
ON CONFLICT (id) DO NOTHING;

SELECT setval('gm_campaign_templates_id_seq', (SELECT MAX(id) FROM gm_campaign_templates));

-- ============================================================================
-- 12. Freeze budget for active campaigns (simulate activation)
-- ============================================================================
-- User 2: campaigns 1(500) + 2(1000) + 3(800) + 5(600) + 8(800) = 3700
UPDATE gm_user_wallets 
SET frozen_points = frozen_points + 3700.00,
    balance_points = balance_points - 3700.00
WHERE user_id = 2;

-- User 3: campaign 4(1200) = 1200
UPDATE gm_user_wallets 
SET frozen_points = frozen_points + 1200.00,
    balance_points = balance_points - 1200.00
WHERE user_id = 3;

-- ============================================================================
-- 13. Crawler Tasks (for video/content association)
-- ============================================================================
INSERT INTO gm_crawler_tasks (id, campaign_id, keywords, max_count, process_count, status, search_offset, search_limit) VALUES
(1, 2, ARRAY['travel china'], 50, 10, 'completed', 0, 20),
(2, 3, ARRAY['fashion trends'], 40, 8, 'completed', 0, 20),
(3, 4, ARRAY['luxury lifestyle'], 60, 15, 'completed', 0, 12),
(4, 1, ARRAY['rust programming'], 30, 5, 'completed', 0, 25),
(5, 5, ARRAY['saas tools'], 35, 7, 'completed', 0, 20)
ON CONFLICT (id) DO NOTHING;

SELECT setval('crawler_tasks_id_seq', (SELECT MAX(id) FROM gm_crawler_tasks));

-- ============================================================================
-- 14. TikTok Videos & Comments (Platform 2)
-- ============================================================================
INSERT INTO gm_agent_videos (id, video_id, author, description, task_id, campaign_id, like_count, comment_count, share_count, play_count, publish_time, author_unique_id, url) VALUES
(1, 'tiktok_video_001', 'travel_influencer', 'Amazing trip to China! #travel #china', 1, 2, 15000, 500, 200, 100000, 1705000000, 'travel_inf_123', 'https://tiktok.com/@travel_inf/video/001'),
(2, 'tiktok_video_002', 'food_blogger', 'Best Chinese food in Beijing! #food #beijing', 1, 2, 8000, 300, 100, 50000, 1705100000, 'food_blog_456', 'https://tiktok.com/@food_blog/video/002'),
(3, 'tiktok_video_003', 'wanderlust_jane', 'Shanghai skyline views #shanghai #travel', 1, 2, 12000, 400, 150, 80000, 1705200000, 'wanderlust_j', 'https://tiktok.com/@wanderlust/video/003')
ON CONFLICT (id) DO NOTHING;

SELECT setval('agent_videos_id_seq', (SELECT MAX(id) FROM gm_agent_videos));

INSERT INTO gm_agent_comments (id, video_db_id, comment_id, user_nickname, user_unique_id, content, reason, suggested_reply, create_time, campaign_id, status, suggested_dm) VALUES
(1, 1, 'tiktok_cmt_001', 'curious_traveler', 'cur_trav_789', 'Where did you stay in China?', 'Asking for travel recommendations', 'We stayed at the amazing Marriott in Shanghai! Check out our website for exclusive deals.', '2025-01-10 10:00:00', 2, 0, 'Hey! Thanks for asking. We have some great travel packages available.'),
(2, 1, 'tiktok_cmt_002', 'asia_lover', 'asia_lov_012', 'I want to visit too! Any tips?', 'Seeking travel advice', 'Book early for the best rates! Our team can help you plan the perfect trip.', '2025-01-10 11:00:00', 2, 0, NULL),
(3, 1, 'tiktok_cmt_003', 'budget_backpacker', 'budget_bp', 'Is it expensive?', 'Price inquiry', 'China can be budget-friendly! We have options starting from $500.', '2025-01-10 12:00:00', 2, 1, 'Hi there! Let me share some budget options with you.'),
(4, 2, 'tiktok_cmt_004', 'foodie_kim', 'foodie_k_345', 'That looks delicious!', 'Food appreciation', 'Thank you! The food scene in Beijing is incredible.', '2025-01-11 09:00:00', 2, 0, NULL),
(5, 2, 'tiktok_cmt_005', 'chef_mike', 'chef_m_678', 'What restaurant is this?', 'Restaurant inquiry', 'This is Dadong, famous for their Peking duck!', '2025-01-11 10:30:00', 2, 2, NULL),
(6, 3, 'tiktok_cmt_006', 'night_owl', 'night_o_901', 'The view is breathtaking!', 'Positive feedback', 'It truly is! Shanghai never sleeps.', '2025-01-12 20:00:00', 2, 0, NULL)
ON CONFLICT (id) DO NOTHING;

SELECT setval('agent_comments_id_seq', (SELECT MAX(id) FROM gm_agent_comments));

-- ============================================================================
-- 15. Facebook Posts & Comments (Platform 3)
-- ============================================================================
INSERT INTO gm_agent_facebook_posts (id, task_id, campaign_id, facebook_post_id, post_type, url, message, timestamp, posted_at, reactions_count, comments_count, author_name) VALUES
(1, 2, 3, 'fb_post_001', 'photo', 'https://facebook.com/posts/001', 'Latest fashion trends for 2025! #fashion #style', 1705300000, '2025-01-15 14:00:00+00', 2500, 150, 'Fashion Daily'),
(2, 2, 3, 'fb_post_002', 'video', 'https://facebook.com/posts/002', 'Style tips for the new season', 1705400000, '2025-01-16 10:00:00+00', 3000, 200, 'Style Guide')
ON CONFLICT (id) DO NOTHING;

SELECT setval('gm_agent_facebook_posts_id_seq', (SELECT MAX(id) FROM gm_agent_facebook_posts));

INSERT INTO gm_agent_facebook_comments (id, post_db_id, campaign_id, facebook_comment_id, comment_text, reason, suggested_reply, status, comment_username, like_count, reply_count) VALUES
(1, 1, 3, 'fb_cmt_001', 'Love this outfit! Where can I buy it?', 'Purchase inquiry', 'Thank you! You can find this at our online store: fashiondaily.com', 'pending', 'sarah_style', 50, 3),
(2, 1, 3, 'fb_cmt_002', 'The colors are amazing!', 'Positive feedback', 'We love bold colors this season!', 'pending', 'color_queen', 30, 1),
(3, 1, 3, 'fb_cmt_003', 'Not my style tbh', 'Neutral feedback', 'Fashion is personal - find what works for you!', 'replied', 'honest_bob', 5, 0),
(4, 2, 3, 'fb_cmt_004', 'Great tips! More please!', 'Content request', 'Stay tuned for more style tips every week!', 'pending', 'tips_lover', 45, 2),
(5, 2, 3, 'fb_cmt_005', 'Can you do a video on accessories?', 'Content suggestion', 'Great idea! We will cover accessories next week.', 'pending', 'accessory_fan', 25, 0)
ON CONFLICT (id) DO NOTHING;

SELECT setval('gm_agent_facebook_comments_id_seq', (SELECT MAX(id) FROM gm_agent_facebook_comments));

-- ============================================================================
-- 16. Instagram Posts & Comments (Platform 4)
-- ============================================================================
INSERT INTO gm_agent_instagram_posts (id, task_id, campaign_id, code, instagram_id, media_type, caption_text, owner_username, like_count, comment_count, posted_at) VALUES
(1, 3, 4, 'IG_CODE_001', 'ig_post_001', 1, 'Living the luxury life! #luxury #lifestyle', 'luxury_living', 8000, 120, '2025-01-17 18:00:00+00'),
(2, 3, 4, 'IG_CODE_002', 'ig_post_002', 2, 'Premium watch collection reveal #watches #luxury', 'watch_collector', 12000, 180, '2025-01-18 12:00:00+00'),
(3, 3, 4, 'IG_CODE_003', 'ig_post_003', 1, 'Luxury resort experience #travel #premium', 'resort_reviewer', 6500, 90, '2025-01-19 15:00:00+00')
ON CONFLICT (id) DO NOTHING;

SELECT setval('gm_agent_instagram_posts_id_seq', (SELECT MAX(id) FROM gm_agent_instagram_posts));

INSERT INTO gm_agent_instagram_comments (id, post_db_id, campaign_id, instagram_comment_id, comment_text, reason, suggested_reply, status, comment_username, like_count, child_comment_count) VALUES
(1, 1, 4, 'ig_cmt_001', 'Goals! 💎', 'Aspirational comment', 'Keep dreaming big! 🌟', 'pending', 'dreamer_99', 120, 5),
(2, 1, 4, 'ig_cmt_002', 'What brand is that bag?', 'Product inquiry', 'Its Hermès Birkin - timeless elegance!', 'pending', 'bag_lover', 85, 3),
(3, 1, 4, 'ig_cmt_003', 'So jealous right now 😍', 'Emotional response', 'Everyone deserves some luxury moments!', 'replied', 'envious_emma', 45, 1),
(4, 2, 4, 'ig_cmt_004', 'That Rolex is stunning!', 'Product appreciation', 'A classic choice that never goes out of style.', 'pending', 'watch_enthusiast', 200, 8),
(5, 2, 4, 'ig_cmt_005', 'Price?', 'Price inquiry', 'DM us for exclusive pricing!', 'pending', 'curious_buyer', 30, 0),
(6, 2, 4, 'ig_cmt_006', 'Can you show the Patek Philippe?', 'Product request', 'Coming soon in our next post!', 'pending', 'patek_fan', 75, 2),
(7, 3, 4, 'ig_cmt_007', 'Which resort is this?', 'Location inquiry', 'This is the Amanpuri in Phuket!', 'pending', 'travel_planner', 65, 4)
ON CONFLICT (id) DO NOTHING;

SELECT setval('gm_agent_instagram_comments_id_seq', (SELECT MAX(id) FROM gm_agent_instagram_comments));

-- ============================================================================
-- 17. Reddit Posts & Comments (Platform 1)
-- ============================================================================
INSERT INTO gm_agent_reddit_posts (id, task_id, campaign_id, post_id, post_name, title, selftext, author, subreddit, url, score, num_comments, post_created_at) VALUES
(1, 4, 1, 'reddit_001', 't3_reddit001', 'Best AI tools for marketing automation?', 'Looking for recommendations on AI marketing tools. What do you use?', 'marketing_pro', 'marketing', 'https://reddit.com/r/marketing/001', 450, 85, '2025-01-14 09:00:00+00'),
(2, 4, 1, 'reddit_002', 't3_reddit002', 'My experience with AI content generation', 'Sharing my journey using AI for content...', 'ai_enthusiast', 'artificial', 'https://reddit.com/r/artificial/002', 320, 62, '2025-01-15 11:00:00+00')
ON CONFLICT (id) DO NOTHING;

SELECT setval('gm_agent_reddit_posts_id_seq', (SELECT MAX(id) FROM gm_agent_reddit_posts));

INSERT INTO gm_agent_reddit_comments (id, post_db_id, campaign_id, comment_id, comment_name, author, body, reason, suggested_reply, status, score, depth, comment_created_at) VALUES
(1, 1, 1, 'rc_001', 't1_rc001', 'helpful_marketer', 'I highly recommend checking out HubSpot AI features!', 'Tool recommendation', 'Great suggestion! Our tool integrates seamlessly with HubSpot.', 'pending', 85, 0, '2025-01-14 10:00:00+00'),
(2, 1, 1, 'rc_002', 't1_rc002', 'skeptic_sam', 'Most AI tools are overhyped imo', 'Skeptical comment', 'We understand the skepticism! Try our free trial to see the results.', 'pending', 42, 0, '2025-01-14 11:30:00+00'),
(3, 1, 1, 'rc_003', 't1_rc003', 'startup_founder', 'Budget is a concern for us', 'Budget constraint', 'We have startup-friendly pricing! Check out our website.', 'replied', 28, 0, '2025-01-14 14:00:00+00'),
(4, 2, 1, 'rc_004', 't1_rc004', 'curious_dev', 'What models do you use?', 'Technical inquiry', 'We use a combination of GPT-4 and custom fine-tuned models.', 'pending', 65, 0, '2025-01-15 12:00:00+00'),
(5, 2, 1, 'rc_005', 't1_rc005', 'content_creator', 'This changed my workflow completely!', 'Success story', 'Love hearing success stories! Would you like to share more?', 'pending', 110, 0, '2025-01-15 15:00:00+00')
ON CONFLICT (id) DO NOTHING;

SELECT setval('gm_agent_reddit_comments_id_seq', (SELECT MAX(id) FROM gm_agent_reddit_comments));

-- ============================================================================
-- 18. Twitter Tweets & Comments (Platform 5)
-- ============================================================================
INSERT INTO gm_agent_twitter_tweets (id, task_id, campaign_id, twitter_tweet_id, conversation_id, full_text, screen_name, user_name, favorite_count, retweet_count, reply_count, tweet_created_at) VALUES
(1, 5, 5, 'tw_001', 'conv_001', 'Just discovered the best SaaS tool for productivity! Thread 🧵', 'saas_reviewer', 'SaaS Reviewer', 2500, 450, 180, '2025-01-16 08:00:00+00'),
(2, 5, 5, 'tw_002', 'conv_002', 'Top 5 productivity tools you NEED in 2025:', 'tech_tips', 'Tech Tips Daily', 5000, 800, 250, '2025-01-17 10:00:00+00')
ON CONFLICT (id) DO NOTHING;

SELECT setval('gm_agent_twitter_tweets_id_seq', (SELECT MAX(id) FROM gm_agent_twitter_tweets));

INSERT INTO gm_agent_twitter_comments (id, tweet_db_id, campaign_id, twitter_comment_id, conversation_id, comment_screen_name, comment_user_name, comment_text, reason, suggested_reply, status, favorite_count, reply_count) VALUES
(1, 1, 5, 'tc_001', 'conv_001', 'curious_user', 'Curious User', 'What tool is it?', 'Product inquiry', 'Its called ProductivityPro! Check it out.', 'pending', 45, 5),
(2, 1, 5, 'tc_002', 'conv_001', 'remote_worker', 'Remote Worker', 'Does it work for remote teams?', 'Feature inquiry', 'Absolutely! Built specifically for remote collaboration.', 'pending', 32, 3),
(3, 1, 5, 'tc_003', 'conv_001', 'enterprise_pm', 'Enterprise PM', 'Any enterprise features?', 'Enterprise inquiry', 'Yes! We have SSO, advanced analytics, and dedicated support.', 'replied', 28, 2),
(4, 2, 5, 'tc_004', 'conv_002', 'startup_ceo', 'Startup CEO', 'Adding this to my stack!', 'Positive engagement', 'Great choice! Let us know if you need any help getting started.', 'pending', 150, 8),
(5, 2, 5, 'tc_005', 'conv_002', 'productivity_nerd', 'Productivity Nerd', 'Finally a good list!', 'Appreciation', 'Thanks! We curate only the best tools.', 'pending', 85, 4)
ON CONFLICT (id) DO NOTHING;

SELECT setval('gm_agent_twitter_comments_id_seq', (SELECT MAX(id) FROM gm_agent_twitter_comments));

-- ============================================================================
-- Verification Queries (for testing)
-- ============================================================================
DO $$
BEGIN
    RAISE NOTICE '=== Test Data Verification ===';
    RAISE NOTICE 'Platforms: %', (SELECT COUNT(*) FROM gm_platforms);
    RAISE NOTICE 'Regions: %', (SELECT COUNT(*) FROM gm_regions);
    RAISE NOTICE 'Pricing Rules: %', (SELECT COUNT(*) FROM gm_pricing_rules);
    RAISE NOTICE 'AI Models: %', (SELECT COUNT(*) FROM gm_ai_models);
    RAISE NOTICE 'Users: %', (SELECT COUNT(*) FROM gm_users);
    RAISE NOTICE 'User Wallets: %', (SELECT COUNT(*) FROM gm_user_wallets);
    RAISE NOTICE 'Social Groups: %', (SELECT COUNT(*) FROM gm_social_groups);
    RAISE NOTICE 'Social Accounts: %', (SELECT COUNT(*) FROM gm_social_accounts);
    RAISE NOTICE 'Campaigns: %', (SELECT COUNT(*) FROM gm_campaigns);
    RAISE NOTICE 'Active Campaigns: %', (SELECT COUNT(*) FROM gm_campaigns WHERE status = 'ACTIVE');
    RAISE NOTICE '--- Agent Data ---';
    RAISE NOTICE 'TikTok Videos: %', (SELECT COUNT(*) FROM gm_agent_videos);
    RAISE NOTICE 'TikTok Comments: %', (SELECT COUNT(*) FROM gm_agent_comments);
    RAISE NOTICE 'Facebook Posts: %', (SELECT COUNT(*) FROM gm_agent_facebook_posts);
    RAISE NOTICE 'Facebook Comments: %', (SELECT COUNT(*) FROM gm_agent_facebook_comments);
    RAISE NOTICE 'Instagram Posts: %', (SELECT COUNT(*) FROM gm_agent_instagram_posts);
    RAISE NOTICE 'Instagram Comments: %', (SELECT COUNT(*) FROM gm_agent_instagram_comments);
    RAISE NOTICE 'Reddit Posts: %', (SELECT COUNT(*) FROM gm_agent_reddit_posts);
    RAISE NOTICE 'Reddit Comments: %', (SELECT COUNT(*) FROM gm_agent_reddit_comments);
    RAISE NOTICE 'Twitter Tweets: %', (SELECT COUNT(*) FROM gm_agent_twitter_tweets);
    RAISE NOTICE 'Twitter Comments: %', (SELECT COUNT(*) FROM gm_agent_twitter_comments);
    RAISE NOTICE '=== Test Data Ready ===';
END $$;
