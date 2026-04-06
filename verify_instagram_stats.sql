-- Instagram AI回复统计数据验证和测试脚本
-- 用于验证修复是否生效

-- ============================================
-- 1. 检查各平台评论表的数据量
-- ============================================

-- TikTok 评论总数
SELECT 'TikTok' as platform, COUNT(*) as total_comments
FROM gm_agent_comments;

-- Instagram 评论总数
SELECT 'Instagram' as platform, COUNT(*) as total_comments
FROM gm_agent_instagram_comments;

-- Facebook 评论总数
SELECT 'Facebook' as platform, COUNT(*) as total_comments
FROM gm_agent_facebook_comments;

-- Twitter 评论总数
SELECT 'Twitter' as platform, COUNT(*) as total_comments
FROM gm_agent_twitter_comments;

-- Reddit 评论总数
SELECT 'Reddit' as platform, COUNT(*) as total_comments
FROM gm_agent_reddit_comments;

-- ============================================
-- 2. 按活动统计各平台的评论数
-- ============================================

-- 查看所有活动及其平台信息
SELECT 
    c.id as campaign_id,
    c.name as campaign_name,
    c.status,
    p.name as platform_name,
    c.created_at
FROM gm_campaigns c
LEFT JOIN gm_platforms p ON c.platform_id = p.id
ORDER BY c.created_at DESC
LIMIT 20;

-- ============================================
-- 3. 针对特定活动的详细统计（修改 campaign_id 值）
-- ============================================

-- 设置要查询的活动 ID（请根据实际情况修改）
DO $$
DECLARE
    target_campaign_id INT := 1;  -- 👈 修改这里的活动ID
BEGIN
    -- TikTok 评论数
    RAISE NOTICE 'Campaign ID: %', target_campaign_id;
    
    RAISE NOTICE 'TikTok comments: %', (
        SELECT COUNT(*) 
        FROM gm_agent_comments ac
        JOIN gm_agent_videos av ON ac.video_db_id = av.id
        JOIN gm_crawler_tasks ct ON av.task_id = ct.id
        WHERE ct.campaign_id = target_campaign_id
    );
    
    -- Instagram 评论数
    RAISE NOTICE 'Instagram comments: %', (
        SELECT COUNT(*) 
        FROM gm_agent_instagram_comments 
        WHERE campaign_id = target_campaign_id
    );
    
    -- Facebook 评论数
    RAISE NOTICE 'Facebook comments: %', (
        SELECT COUNT(*) 
        FROM gm_agent_facebook_comments 
        WHERE campaign_id = target_campaign_id
    );
    
    -- Twitter 评论数
    RAISE NOTICE 'Twitter comments: %', (
        SELECT COUNT(*) 
        FROM gm_agent_twitter_comments 
        WHERE campaign_id = target_campaign_id
    );
    
    -- Reddit 评论数
    RAISE NOTICE 'Reddit comments: %', (
        SELECT COUNT(*) 
        FROM gm_agent_reddit_comments 
        WHERE campaign_id = target_campaign_id
    );
    
    -- 总计
    RAISE NOTICE 'Total comments: %', (
        SELECT 
            COALESCE((SELECT COUNT(*) FROM gm_agent_comments ac
                     JOIN gm_agent_videos av ON ac.video_db_id = av.id
                     JOIN gm_crawler_tasks ct ON av.task_id = ct.id
                     WHERE ct.campaign_id = target_campaign_id), 0) +
            COALESCE((SELECT COUNT(*) FROM gm_agent_instagram_comments WHERE campaign_id = target_campaign_id), 0) +
            COALESCE((SELECT COUNT(*) FROM gm_agent_facebook_comments WHERE campaign_id = target_campaign_id), 0) +
            COALESCE((SELECT COUNT(*) FROM gm_agent_twitter_comments WHERE campaign_id = target_campaign_id), 0) +
            COALESCE((SELECT COUNT(*) FROM gm_agent_reddit_comments WHERE campaign_id = target_campaign_id), 0)
    );
END $$;

-- ============================================
-- 4. 查询所有活动的跨平台统计（完整版本）
-- ============================================

WITH campaign_stats AS (
    -- TikTok 评论统计
    SELECT 
        ct.campaign_id,
        COUNT(*) as comment_count,
        'TikTok' as platform
    FROM gm_agent_comments ac
    JOIN gm_agent_videos av ON ac.video_db_id = av.id
    JOIN gm_crawler_tasks ct ON av.task_id = ct.id
    GROUP BY ct.campaign_id
    
    UNION ALL
    
    -- Instagram 评论统计
    SELECT 
        campaign_id,
        COUNT(*) as comment_count,
        'Instagram' as platform
    FROM gm_agent_instagram_comments
    WHERE campaign_id IS NOT NULL
    GROUP BY campaign_id
    
    UNION ALL
    
    -- Facebook 评论统计
    SELECT 
        campaign_id,
        COUNT(*) as comment_count,
        'Facebook' as platform
    FROM gm_agent_facebook_comments
    WHERE campaign_id IS NOT NULL
    GROUP BY campaign_id
    
    UNION ALL
    
    -- Twitter 评论统计
    SELECT 
        campaign_id,
        COUNT(*) as comment_count,
        'Twitter' as platform
    FROM gm_agent_twitter_comments
    WHERE campaign_id IS NOT NULL
    GROUP BY campaign_id
    
    UNION ALL
    
    -- Reddit 评论统计
    SELECT 
        campaign_id,
        COUNT(*) as comment_count,
        'Reddit' as platform
    FROM gm_agent_reddit_comments
    WHERE campaign_id IS NOT NULL
    GROUP BY campaign_id
)
SELECT 
    c.id as campaign_id,
    c.name as campaign_name,
    p.name as platform_name,
    c.status,
    COALESCE(SUM(cs.comment_count), 0) as total_comments,
    STRING_AGG(DISTINCT cs.platform || ': ' || cs.comment_count, ', ') as platform_breakdown
FROM gm_campaigns c
LEFT JOIN gm_platforms p ON c.platform_id = p.id
LEFT JOIN campaign_stats cs ON c.id = cs.campaign_id
GROUP BY c.id, c.name, p.name, c.status
ORDER BY total_comments DESC, c.created_at DESC;

-- ============================================
-- 5. Instagram 活动详细诊断
-- ============================================

-- 查找所有 Instagram 相关的活动
SELECT 
    c.id as campaign_id,
    c.name as campaign_name,
    c.status,
    c.platform_id,
    p.name as platform_name,
    (SELECT COUNT(*) 
     FROM gm_agent_instagram_comments ic 
     WHERE ic.campaign_id = c.id) as instagram_comments_count,
    c.created_at
FROM gm_campaigns c
LEFT JOIN gm_platforms p ON c.platform_id = p.id
WHERE p.name ILIKE '%instagram%' OR p.name ILIKE '%ins%'
ORDER BY c.created_at DESC;

-- ============================================
-- 6. 检查评论表的 campaign_id 关联情况
-- ============================================

-- Instagram 评论中有多少关联了 campaign_id
SELECT 
    'Instagram' as platform,
    COUNT(*) as total_comments,
    COUNT(campaign_id) as with_campaign_id,
    COUNT(*) - COUNT(campaign_id) as without_campaign_id,
    ROUND(COUNT(campaign_id)::numeric / NULLIF(COUNT(*), 0) * 100, 2) as campaign_id_fill_rate
FROM gm_agent_instagram_comments

UNION ALL

-- Facebook 评论中有多少关联了 campaign_id
SELECT 
    'Facebook' as platform,
    COUNT(*) as total_comments,
    COUNT(campaign_id) as with_campaign_id,
    COUNT(*) - COUNT(campaign_id) as without_campaign_id,
    ROUND(COUNT(campaign_id)::numeric / NULLIF(COUNT(*), 0) * 100, 2) as campaign_id_fill_rate
FROM gm_agent_facebook_comments

UNION ALL

-- Twitter 评论中有多少关联了 campaign_id
SELECT 
    'Twitter' as platform,
    COUNT(*) as total_comments,
    COUNT(campaign_id) as with_campaign_id,
    COUNT(*) - COUNT(campaign_id) as without_campaign_id,
    ROUND(COUNT(campaign_id)::numeric / NULLIF(COUNT(*), 0) * 100, 2) as campaign_id_fill_rate
FROM gm_agent_twitter_comments

UNION ALL

-- Reddit 评论中有多少关联了 campaign_id
SELECT 
    'Reddit' as platform,
    COUNT(*) as total_comments,
    COUNT(campaign_id) as with_campaign_id,
    COUNT(*) - COUNT(campaign_id) as without_campaign_id,
    ROUND(COUNT(campaign_id)::numeric / NULLIF(COUNT(*), 0) * 100, 2) as campaign_id_fill_rate
FROM gm_agent_reddit_comments;

-- ============================================
-- 7. 示例：创建测试数据（仅用于开发环境）
-- ============================================

-- 注意：只在开发/测试环境执行！
-- 以下代码会创建测试数据

/*
-- 假设有一个 Instagram 活动，ID 为 999
DO $$
DECLARE
    test_campaign_id INT := 999;
    test_post_id INT;
BEGIN
    -- 检查活动是否存在
    IF NOT EXISTS (SELECT 1 FROM gm_campaigns WHERE id = test_campaign_id) THEN
        RAISE NOTICE 'Campaign % does not exist. Please create it first.', test_campaign_id;
        RETURN;
    END IF;
    
    -- 创建测试 Instagram 帖子
    INSERT INTO gm_agent_instagram_posts (
        campaign_id, task_id, instagram_post_id, 
        post_url, caption, author_username
    )
    VALUES (
        test_campaign_id, 1, 'test_post_123',
        'https://instagram.com/p/test_post_123/', 
        'Test post for statistics', 'test_user'
    )
    RETURNING id INTO test_post_id;
    
    -- 插入测试评论
    INSERT INTO gm_agent_instagram_comments (
        post_db_id, campaign_id, instagram_comment_id,
        comment_text, reason, suggested_reply, status
    )
    SELECT 
        test_post_id,
        test_campaign_id,
        'test_comment_' || i,
        'This is test comment #' || i,
        'Test reason',
        'Test suggested reply',
        2  -- status = 2 表示已回复
    FROM generate_series(1, 50) i;
    
    RAISE NOTICE 'Created 50 test Instagram comments for campaign %', test_campaign_id;
END $$;
*/

-- ============================================
-- 8. 性能测试：比较修复前后的查询性能
-- ============================================

-- 修复前的查询（仅 TikTok）
EXPLAIN ANALYZE
SELECT COUNT(*) 
FROM gm_agent_comments ac
JOIN gm_agent_videos av ON ac.video_db_id = av.id
JOIN gm_crawler_tasks ct ON av.task_id = ct.id
WHERE ct.campaign_id = 1;

-- 修复后的查询（所有平台）
EXPLAIN ANALYZE
SELECT 
    COALESCE((SELECT COUNT(*) FROM gm_agent_comments ac
             JOIN gm_agent_videos av ON ac.video_db_id = av.id
             JOIN gm_crawler_tasks ct ON av.task_id = ct.id
             WHERE ct.campaign_id = 1), 0) +
    COALESCE((SELECT COUNT(*) FROM gm_agent_instagram_comments WHERE campaign_id = 1), 0) +
    COALESCE((SELECT COUNT(*) FROM gm_agent_facebook_comments WHERE campaign_id = 1), 0) +
    COALESCE((SELECT COUNT(*) FROM gm_agent_twitter_comments WHERE campaign_id = 1), 0) +
    COALESCE((SELECT COUNT(*) FROM gm_agent_reddit_comments WHERE campaign_id = 1), 0)
AS total_count;

-- ============================================
-- 9. 数据完整性检查
-- ============================================

-- 检查是否有孤立的评论（没有关联到活动）
SELECT 
    'Instagram' as platform,
    COUNT(*) as orphaned_comments
FROM gm_agent_instagram_comments
WHERE campaign_id IS NULL

UNION ALL

SELECT 
    'Facebook' as platform,
    COUNT(*) as orphaned_comments
FROM gm_agent_facebook_comments
WHERE campaign_id IS NULL

UNION ALL

SELECT 
    'Twitter' as platform,
    COUNT(*) as orphaned_comments
FROM gm_agent_twitter_comments
WHERE campaign_id IS NULL

UNION ALL

SELECT 
    'Reddit' as platform,
    COUNT(*) as orphaned_comments
FROM gm_agent_reddit_comments
WHERE campaign_id IS NULL;
