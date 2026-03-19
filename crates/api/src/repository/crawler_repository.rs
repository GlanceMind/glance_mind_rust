use crate::config::database::DBPool;
use crate::dto::crawler_dto::UnifiedContentDto;
use crate::platform_routing::SupportedPlatform;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::SelectableHelper;
use glance_mind_db::entity::agent::{
    AgentVideo, FacebookPost, InstagramPost, RedditPost, TwitterTweet,
};
use glance_mind_db::entity::crawler::{CrawlerResult, CrawlerTask};
use glance_mind_db::entity::platform::Platform;
use glance_mind_db::schema::{
    gm_agent_comments, gm_agent_facebook_comments, gm_agent_facebook_posts,
    gm_agent_instagram_comments, gm_agent_instagram_posts, gm_agent_reddit_comments,
    gm_agent_reddit_posts, gm_agent_twitter_comments, gm_agent_twitter_tweets,
    gm_agent_videos, gm_campaigns, gm_crawler_results as crawler_results,
    gm_crawler_tasks as crawler_tasks, gm_platforms,
};
use std::collections::HashMap;

#[derive(Clone)]
pub struct CrawlerRepository {
    pool: DBPool,
}

impl CrawlerRepository {
    pub fn new(pool: DBPool) -> Self {
        Self { pool }
    }

    pub async fn find_tasks_by_campaign_id(
        &self,
        c_id: i32,
        page: i64,
        page_size: i64,
    ) -> Result<(Vec<CrawlerTask>, i64), diesel::result::Error> {
        let mut conn = self.pool.get().unwrap();

        let total = crawler_tasks::table
            .filter(crawler_tasks::campaign_id.eq(c_id))
            .count()
            .get_result(&mut conn)?;

        let items = crawler_tasks::table
            .filter(crawler_tasks::campaign_id.eq(c_id))
            .order(crawler_tasks::created_at.desc())
            .limit(page_size)
            .offset((page - 1) * page_size)
            .select(CrawlerTask::as_select())
            .load(&mut conn)?;

        Ok((items, total))
    }

    pub async fn find_results_by_task_id(
        &self,
        t_id: i32,
        page: i64,
        page_size: i64,
    ) -> Result<(Vec<CrawlerResult>, i64), diesel::result::Error> {
        let mut conn = self.pool.get().unwrap();

        let total = crawler_results::table
            .filter(crawler_results::task_id.eq(t_id))
            .count()
            .get_result(&mut conn)?;

        let items = crawler_results::table
            .filter(crawler_results::task_id.eq(t_id))
            .order(crawler_results::created_at.desc())
            .limit(page_size)
            .offset((page - 1) * page_size)
            .select(CrawlerResult::as_select())
            .load(&mut conn)?;

        Ok((items, total))
    }

    /// Legacy method for backward compatibility - only queries TikTok videos
    pub async fn find_results_by_campaign_id(
        &self,
        c_id: i32,
        page: i64,
        page_size: i64,
    ) -> Result<(Vec<CrawlerResult>, i64), diesel::result::Error> {
        let mut conn = self.pool.get().unwrap();

        // Count total agent videos for this campaign
        let total: i64 = gm_agent_videos::table
            .inner_join(crawler_tasks::table)
            .filter(crawler_tasks::campaign_id.eq(c_id))
            .count()
            .get_result(&mut conn)?;

        // Fetch agent videos with proper join
        let agent_videos_list: Vec<AgentVideo> = gm_agent_videos::table
            .inner_join(crawler_tasks::table)
            .filter(crawler_tasks::campaign_id.eq(c_id))
            .order(gm_agent_videos::created_at.desc())
            .limit(page_size)
            .offset((page - 1) * page_size)
            .select(AgentVideo::as_select())
            .load(&mut conn)?;

        // Convert AgentVideo to CrawlerResult
        let items: Vec<CrawlerResult> = agent_videos_list
            .into_iter()
            .map(|av| CrawlerResult {
                id: av.id,
                task_id: av.task_id,
                video_id: av.video_id.unwrap_or_default(),
                video_title: av.description.clone(),
                comment_count: av.comment_count,
                view_count: av.play_count,
                like_count: av.like_count,
                author_name: av.author,
                processed: true,
                replied: false,
                created_at: av.created_at,
                updated_at: Some(av.created_at),
            })
            .collect();

        Ok((items, total))
    }

    /// Resolve a supported platform from gm_platforms.name.
    fn get_supported_platform(
        &self,
        platform_id: i32,
    ) -> Result<SupportedPlatform, diesel::result::Error> {
        let mut conn = self.pool.get().unwrap();
        let platform: Platform = gm_platforms::table
            .find(platform_id)
            .select(Platform::as_select())
            .first(&mut conn)?;
        SupportedPlatform::from_name(&platform.name).ok_or(diesel::result::Error::NotFound)
    }

    fn get_task_supported_platform(
        &self,
        task_id: i32,
    ) -> Result<SupportedPlatform, diesel::result::Error> {
        let mut conn = self.pool.get().unwrap();
        let platform_id = crawler_tasks::table
            .inner_join(gm_campaigns::table)
            .filter(crawler_tasks::id.eq(task_id))
            .select(gm_campaigns::platform_id)
            .first::<i32>(&mut conn)?;

        self.get_supported_platform(platform_id)
    }

    fn valid_comment_count_for(
        content_db_id: i32,
        valid_comment_counts: &HashMap<i32, i64>,
    ) -> i64 {
        *valid_comment_counts.get(&content_db_id).unwrap_or(&0)
    }

    /// Unified method to query content by campaign and platform
    /// Uses database to resolve platform_id -> platform_name, then routes to correct table
    pub async fn find_unified_contents_by_campaign(
        &self,
        campaign_id: i32,
        platform_id: i32,
        page: i64,
        page_size: i64,
    ) -> Result<(Vec<UnifiedContentDto>, i64), diesel::result::Error> {
        // Get platform name from database (no hardcoded platform_id mapping)
        let platform = self.get_supported_platform(platform_id)?;

        tracing::info!(
            "find_unified_contents_by_campaign: campaign_id={}, platform_id={}, platform_name={}, page={}, page_size={}",
            campaign_id,
            platform_id,
            platform.as_db_name(),
            page,
            page_size
        );

        // Route to correct table based on platform name (hardcoded: platform_name -> data_table)
        let result = match platform {
            SupportedPlatform::Tiktok => {
                tracing::info!("Querying TikTok (gm_agent_videos)");
                self.find_tiktok_contents(campaign_id, page, page_size)
                    .await
            }
            SupportedPlatform::Facebook => {
                tracing::info!("Querying Facebook (gm_agent_facebook_posts)");
                self.find_facebook_contents(campaign_id, page, page_size)
                    .await
            }
            SupportedPlatform::Instagram => {
                tracing::info!("Querying Instagram (gm_agent_instagram_posts)");
                self.find_instagram_contents(campaign_id, page, page_size)
                    .await
            }
            SupportedPlatform::Reddit => {
                tracing::info!("Querying Reddit (gm_agent_reddit_posts)");
                self.find_reddit_contents(campaign_id, page, page_size)
                    .await
            }
            SupportedPlatform::Twitter => {
                tracing::info!("Querying Twitter (gm_agent_twitter_tweets)");
                self.find_twitter_contents(campaign_id, page, page_size)
                    .await
            }
        };

        if let Ok((ref contents, total)) = result {
            tracing::info!("Query result: {} contents, total={}", contents.len(), total);
        }

        result
    }

    pub async fn find_unified_contents_by_task(
        &self,
        task_id: i32,
        page: i64,
        page_size: i64,
    ) -> Result<(Vec<UnifiedContentDto>, i64), diesel::result::Error> {
        let platform = self.get_task_supported_platform(task_id)?;

        tracing::info!(
            "find_unified_contents_by_task: task_id={}, platform_name={}, page={}, page_size={}",
            task_id,
            platform.as_db_name(),
            page,
            page_size
        );

        let result = match platform {
            SupportedPlatform::Tiktok => {
                tracing::info!("Querying TikTok task results (gm_agent_videos)");
                self.find_tiktok_contents_by_task(task_id, page, page_size)
                    .await
            }
            SupportedPlatform::Facebook => {
                tracing::info!("Querying Facebook task results (gm_agent_facebook_posts)");
                self.find_facebook_contents_by_task(task_id, page, page_size)
                    .await
            }
            SupportedPlatform::Instagram => {
                tracing::info!("Querying Instagram task results (gm_agent_instagram_posts)");
                self.find_instagram_contents_by_task(task_id, page, page_size)
                    .await
            }
            SupportedPlatform::Reddit => {
                tracing::info!("Querying Reddit task results (gm_agent_reddit_posts)");
                self.find_reddit_contents_by_task(task_id, page, page_size)
                    .await
            }
            SupportedPlatform::Twitter => {
                tracing::info!("Querying Twitter task results (gm_agent_twitter_tweets)");
                self.find_twitter_contents_by_task(task_id, page, page_size)
                    .await
            }
        };

        if let Ok((ref contents, total)) = result {
            tracing::info!(
                "Task query result: task_id={}, contents={}, total={}",
                task_id,
                contents.len(),
                total
            );
        }

        result
    }

    async fn find_tiktok_contents(
        &self,
        campaign_id: i32,
        page: i64,
        page_size: i64,
    ) -> Result<(Vec<UnifiedContentDto>, i64), diesel::result::Error> {
        let mut conn = self.pool.get().unwrap();

        let total: i64 = gm_agent_videos::table
            .filter(gm_agent_videos::campaign_id.eq(campaign_id))
            .count()
            .get_result(&mut conn)?;

        tracing::info!(
            "find_tiktok_contents: campaign_id={}, total={}",
            campaign_id,
            total
        );

        let items: Vec<AgentVideo> = gm_agent_videos::table
            .filter(gm_agent_videos::campaign_id.eq(campaign_id))
            .order(gm_agent_videos::created_at.desc())
            .limit(page_size)
            .offset((page - 1) * page_size)
            .select(AgentVideo::as_select())
            .load(&mut conn)?;

        let content_db_ids: Vec<i32> = items.iter().map(|video| video.id).collect();
        let valid_comment_counts =
            Self::load_tiktok_valid_comment_counts(&mut conn, &content_db_ids)?;
        let dtos = Self::map_tiktok_contents(items, &valid_comment_counts);

        Ok((dtos, total))
    }

    async fn find_tiktok_contents_by_task(
        &self,
        task_id: i32,
        page: i64,
        page_size: i64,
    ) -> Result<(Vec<UnifiedContentDto>, i64), diesel::result::Error> {
        let mut conn = self.pool.get().unwrap();

        let total: i64 = gm_agent_videos::table
            .filter(gm_agent_videos::task_id.eq(task_id))
            .count()
            .get_result(&mut conn)?;

        let items: Vec<AgentVideo> = gm_agent_videos::table
            .filter(gm_agent_videos::task_id.eq(task_id))
            .order(gm_agent_videos::created_at.desc())
            .limit(page_size)
            .offset((page - 1) * page_size)
            .select(AgentVideo::as_select())
            .load(&mut conn)?;

        let content_db_ids: Vec<i32> = items.iter().map(|video| video.id).collect();
        let valid_comment_counts =
            Self::load_tiktok_valid_comment_counts(&mut conn, &content_db_ids)?;
        let dtos = Self::map_tiktok_contents(items, &valid_comment_counts);

        Ok((dtos, total))
    }

    fn load_tiktok_valid_comment_counts(
        conn: &mut PgConnection,
        video_db_ids: &[i32],
    ) -> Result<HashMap<i32, i64>, diesel::result::Error> {
        if video_db_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let counts: Vec<(i32, i64)> = gm_agent_comments::table
            .filter(gm_agent_comments::video_db_id.eq_any(video_db_ids))
            .group_by(gm_agent_comments::video_db_id)
            .select((
                gm_agent_comments::video_db_id,
                diesel::dsl::count_star(),
            ))
            .load(conn)?;

        Ok(counts.into_iter().collect())
    }

    fn map_tiktok_contents(
        items: Vec<AgentVideo>,
        valid_comment_counts: &HashMap<i32, i64>,
    ) -> Vec<UnifiedContentDto> {
        items
            .into_iter()
            .map(|v| UnifiedContentDto {
                id: v.id,
                task_id: v.task_id,
                content_id: v.video_id.unwrap_or_default(),
                content_type: "video".to_string(),
                platform: "tiktok".to_string(),
                title: v.description,
                author_name: v.author,
                author_id: v.author_unique_id,
                url: v.url,
                thumbnail_url: None,
                like_count: v.like_count,
                comment_count: v.comment_count,
                valid_comment_count: Self::valid_comment_count_for(v.id, valid_comment_counts),
                share_count: v.share_count,
                view_count: v.play_count,
                processed: true,
                replied: false,
                posted_at: v.publish_time.map(|ts| {
                    chrono::DateTime::from_timestamp(ts, 0)
                        .unwrap_or_default()
                        .with_timezone(&chrono::Utc)
                }),
                created_at: v.created_at,
                campaign_id: v.campaign_id,
            })
            .collect()
    }

    async fn find_facebook_contents(
        &self,
        campaign_id: i32,
        page: i64,
        page_size: i64,
    ) -> Result<(Vec<UnifiedContentDto>, i64), diesel::result::Error> {
        let mut conn = self.pool.get().unwrap();

        let total: i64 = gm_agent_facebook_posts::table
            .filter(gm_agent_facebook_posts::campaign_id.eq(campaign_id))
            .count()
            .get_result(&mut conn)?;

        tracing::info!(
            "find_facebook_contents: campaign_id={}, total={}",
            campaign_id,
            total
        );

        let items: Vec<FacebookPost> = gm_agent_facebook_posts::table
            .filter(gm_agent_facebook_posts::campaign_id.eq(campaign_id))
            .order(gm_agent_facebook_posts::created_at.desc())
            .limit(page_size)
            .offset((page - 1) * page_size)
            .select(FacebookPost::as_select())
            .load(&mut conn)?;

        let content_db_ids: Vec<i32> = items.iter().map(|post| post.id).collect();
        let valid_comment_counts =
            Self::load_facebook_valid_comment_counts(&mut conn, &content_db_ids)?;
        let dtos = Self::map_facebook_contents(items, &valid_comment_counts);

        Ok((dtos, total))
    }

    async fn find_facebook_contents_by_task(
        &self,
        task_id: i32,
        page: i64,
        page_size: i64,
    ) -> Result<(Vec<UnifiedContentDto>, i64), diesel::result::Error> {
        let mut conn = self.pool.get().unwrap();

        let total: i64 = gm_agent_facebook_posts::table
            .filter(gm_agent_facebook_posts::task_id.eq(task_id))
            .count()
            .get_result(&mut conn)?;

        let items: Vec<FacebookPost> = gm_agent_facebook_posts::table
            .filter(gm_agent_facebook_posts::task_id.eq(task_id))
            .order(gm_agent_facebook_posts::created_at.desc())
            .limit(page_size)
            .offset((page - 1) * page_size)
            .select(FacebookPost::as_select())
            .load(&mut conn)?;

        let content_db_ids: Vec<i32> = items.iter().map(|post| post.id).collect();
        let valid_comment_counts =
            Self::load_facebook_valid_comment_counts(&mut conn, &content_db_ids)?;
        let dtos = Self::map_facebook_contents(items, &valid_comment_counts);

        Ok((dtos, total))
    }

    fn load_facebook_valid_comment_counts(
        conn: &mut PgConnection,
        post_db_ids: &[i32],
    ) -> Result<HashMap<i32, i64>, diesel::result::Error> {
        if post_db_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let counts: Vec<(i32, i64)> = gm_agent_facebook_comments::table
            .filter(gm_agent_facebook_comments::post_db_id.eq_any(post_db_ids))
            .group_by(gm_agent_facebook_comments::post_db_id)
            .select((
                gm_agent_facebook_comments::post_db_id,
                diesel::dsl::count_star(),
            ))
            .load(conn)?;

        Ok(counts.into_iter().collect())
    }

    fn map_facebook_contents(
        items: Vec<FacebookPost>,
        valid_comment_counts: &HashMap<i32, i64>,
    ) -> Vec<UnifiedContentDto> {
        items
            .into_iter()
            .map(|p| UnifiedContentDto {
                id: p.id,
                task_id: p.task_id,
                content_id: p.facebook_post_id,
                content_type: p.post_type.unwrap_or_else(|| "post".to_string()),
                platform: "facebook".to_string(),
                title: p.message,
                author_name: p.author_name,
                author_id: p.author_id,
                url: p.url,
                thumbnail_url: p.image_url.or(p.video_thumbnail),
                like_count: p.reactions_count,
                comment_count: p.comments_count,
                valid_comment_count: Self::valid_comment_count_for(p.id, valid_comment_counts),
                share_count: p.reshare_count,
                view_count: None,
                processed: true,
                replied: false,
                posted_at: p.posted_at,
                created_at: p.created_at,
                campaign_id: p.campaign_id,
            })
            .collect()
    }

    async fn find_instagram_contents(
        &self,
        campaign_id: i32,
        page: i64,
        page_size: i64,
    ) -> Result<(Vec<UnifiedContentDto>, i64), diesel::result::Error> {
        let mut conn = self.pool.get().unwrap();

        let total: i64 = gm_agent_instagram_posts::table
            .filter(gm_agent_instagram_posts::campaign_id.eq(campaign_id))
            .count()
            .get_result(&mut conn)?;

        tracing::info!(
            "find_instagram_contents: campaign_id={}, total={}",
            campaign_id,
            total
        );

        let items: Vec<InstagramPost> = gm_agent_instagram_posts::table
            .filter(gm_agent_instagram_posts::campaign_id.eq(campaign_id))
            .order(gm_agent_instagram_posts::created_at.desc())
            .limit(page_size)
            .offset((page - 1) * page_size)
            .select(InstagramPost::as_select())
            .load(&mut conn)?;

        let content_db_ids: Vec<i32> = items.iter().map(|post| post.id).collect();
        let valid_comment_counts =
            Self::load_instagram_valid_comment_counts(&mut conn, &content_db_ids)?;
        let dtos = Self::map_instagram_contents(items, &valid_comment_counts);

        Ok((dtos, total))
    }

    async fn find_instagram_contents_by_task(
        &self,
        task_id: i32,
        page: i64,
        page_size: i64,
    ) -> Result<(Vec<UnifiedContentDto>, i64), diesel::result::Error> {
        let mut conn = self.pool.get().unwrap();

        let total: i64 = gm_agent_instagram_posts::table
            .filter(gm_agent_instagram_posts::task_id.eq(task_id))
            .count()
            .get_result(&mut conn)?;

        let items: Vec<InstagramPost> = gm_agent_instagram_posts::table
            .filter(gm_agent_instagram_posts::task_id.eq(task_id))
            .order(gm_agent_instagram_posts::created_at.desc())
            .limit(page_size)
            .offset((page - 1) * page_size)
            .select(InstagramPost::as_select())
            .load(&mut conn)?;

        let content_db_ids: Vec<i32> = items.iter().map(|post| post.id).collect();
        let valid_comment_counts =
            Self::load_instagram_valid_comment_counts(&mut conn, &content_db_ids)?;
        let dtos = Self::map_instagram_contents(items, &valid_comment_counts);

        Ok((dtos, total))
    }

    fn load_instagram_valid_comment_counts(
        conn: &mut PgConnection,
        post_db_ids: &[i32],
    ) -> Result<HashMap<i32, i64>, diesel::result::Error> {
        if post_db_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let counts: Vec<(i32, i64)> = gm_agent_instagram_comments::table
            .filter(gm_agent_instagram_comments::post_db_id.eq_any(post_db_ids))
            .group_by(gm_agent_instagram_comments::post_db_id)
            .select((
                gm_agent_instagram_comments::post_db_id,
                diesel::dsl::count_star(),
            ))
            .load(conn)?;

        Ok(counts.into_iter().collect())
    }

    fn map_instagram_contents(
        items: Vec<InstagramPost>,
        valid_comment_counts: &HashMap<i32, i64>,
    ) -> Vec<UnifiedContentDto> {
        items
            .into_iter()
            .map(|p| {
                let content_type = match p.product_type.as_deref() {
                    Some("clips") => "reel",
                    Some("feed") => "post",
                    _ => "post",
                };
                UnifiedContentDto {
                    id: p.id,
                    task_id: p.task_id,
                    content_id: p.code,
                    content_type: content_type.to_string(),
                    platform: "instagram".to_string(),
                    title: p.caption_text,
                    author_name: p.owner_username,
                    author_id: p.owner_id,
                    url: p.media_url,
                    thumbnail_url: p.thumbnail_url,
                    like_count: p.like_count,
                    comment_count: p.comment_count,
                    valid_comment_count: Self::valid_comment_count_for(p.id, valid_comment_counts),
                    share_count: None,
                    view_count: p.play_count,
                    processed: true,
                    replied: false,
                    posted_at: p.posted_at,
                    created_at: p.created_at,
                    campaign_id: p.campaign_id,
                }
            })
            .collect()
    }

    async fn find_reddit_contents(
        &self,
        campaign_id: i32,
        page: i64,
        page_size: i64,
    ) -> Result<(Vec<UnifiedContentDto>, i64), diesel::result::Error> {
        let mut conn = self.pool.get().unwrap();

        let total: i64 = gm_agent_reddit_posts::table
            .filter(gm_agent_reddit_posts::campaign_id.eq(campaign_id))
            .count()
            .get_result(&mut conn)?;

        tracing::info!(
            "find_reddit_contents: campaign_id={}, total={}",
            campaign_id,
            total
        );

        let items: Vec<RedditPost> = gm_agent_reddit_posts::table
            .filter(gm_agent_reddit_posts::campaign_id.eq(campaign_id))
            .order(gm_agent_reddit_posts::created_at.desc())
            .limit(page_size)
            .offset((page - 1) * page_size)
            .select(RedditPost::as_select())
            .load(&mut conn)?;

        let content_db_ids: Vec<i32> = items.iter().map(|post| post.id).collect();
        let valid_comment_counts =
            Self::load_reddit_valid_comment_counts(&mut conn, &content_db_ids)?;
        let dtos = Self::map_reddit_contents(items, &valid_comment_counts);

        Ok((dtos, total))
    }

    async fn find_reddit_contents_by_task(
        &self,
        task_id: i32,
        page: i64,
        page_size: i64,
    ) -> Result<(Vec<UnifiedContentDto>, i64), diesel::result::Error> {
        let mut conn = self.pool.get().unwrap();

        let total: i64 = gm_agent_reddit_posts::table
            .filter(gm_agent_reddit_posts::task_id.eq(task_id))
            .count()
            .get_result(&mut conn)?;

        let items: Vec<RedditPost> = gm_agent_reddit_posts::table
            .filter(gm_agent_reddit_posts::task_id.eq(task_id))
            .order(gm_agent_reddit_posts::created_at.desc())
            .limit(page_size)
            .offset((page - 1) * page_size)
            .select(RedditPost::as_select())
            .load(&mut conn)?;

        let content_db_ids: Vec<i32> = items.iter().map(|post| post.id).collect();
        let valid_comment_counts =
            Self::load_reddit_valid_comment_counts(&mut conn, &content_db_ids)?;
        let dtos = Self::map_reddit_contents(items, &valid_comment_counts);

        Ok((dtos, total))
    }

    fn load_reddit_valid_comment_counts(
        conn: &mut PgConnection,
        post_db_ids: &[i32],
    ) -> Result<HashMap<i32, i64>, diesel::result::Error> {
        if post_db_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let counts: Vec<(i32, i64)> = gm_agent_reddit_comments::table
            .filter(gm_agent_reddit_comments::post_db_id.eq_any(post_db_ids))
            .group_by(gm_agent_reddit_comments::post_db_id)
            .select((gm_agent_reddit_comments::post_db_id, diesel::dsl::count_star()))
            .load(conn)?;

        Ok(counts.into_iter().collect())
    }

    fn map_reddit_contents(
        items: Vec<RedditPost>,
        valid_comment_counts: &HashMap<i32, i64>,
    ) -> Vec<UnifiedContentDto> {
        items
            .into_iter()
            .map(|p| UnifiedContentDto {
                id: p.id,
                task_id: p.task_id,
                content_id: p.post_id,
                content_type: if p.is_video.unwrap_or(false) {
                    "video".to_string()
                } else {
                    "post".to_string()
                },
                platform: "reddit".to_string(),
                title: Some(p.title),
                author_name: p.author,
                author_id: None,
                url: p.url,
                thumbnail_url: p.thumbnail,
                like_count: p.score, // Reddit uses score instead of likes
                comment_count: p.num_comments,
                valid_comment_count: Self::valid_comment_count_for(p.id, valid_comment_counts),
                share_count: None,
                view_count: None,
                processed: true,
                replied: false,
                posted_at: p.post_created_at,
                created_at: p.created_at,
                campaign_id: p.campaign_id,
            })
            .collect()
    }

    async fn find_twitter_contents(
        &self,
        campaign_id: i32,
        page: i64,
        page_size: i64,
    ) -> Result<(Vec<UnifiedContentDto>, i64), diesel::result::Error> {
        let mut conn = self.pool.get().unwrap();

        let total: i64 = gm_agent_twitter_tweets::table
            .filter(gm_agent_twitter_tweets::campaign_id.eq(campaign_id))
            .count()
            .get_result(&mut conn)?;

        tracing::info!(
            "find_twitter_contents: campaign_id={}, total={}",
            campaign_id,
            total
        );

        let items: Vec<TwitterTweet> = gm_agent_twitter_tweets::table
            .filter(gm_agent_twitter_tweets::campaign_id.eq(campaign_id))
            .order(gm_agent_twitter_tweets::created_at.desc())
            .limit(page_size)
            .offset((page - 1) * page_size)
            .select(TwitterTweet::as_select())
            .load(&mut conn)?;

        let content_db_ids: Vec<i32> = items.iter().map(|tweet| tweet.id).collect();
        let valid_comment_counts =
            Self::load_twitter_valid_comment_counts(&mut conn, &content_db_ids)?;
        let dtos = Self::map_twitter_contents(items, &valid_comment_counts);

        Ok((dtos, total))
    }

    async fn find_twitter_contents_by_task(
        &self,
        task_id: i32,
        page: i64,
        page_size: i64,
    ) -> Result<(Vec<UnifiedContentDto>, i64), diesel::result::Error> {
        let mut conn = self.pool.get().unwrap();

        let total: i64 = gm_agent_twitter_tweets::table
            .filter(gm_agent_twitter_tweets::task_id.eq(task_id))
            .count()
            .get_result(&mut conn)?;

        let items: Vec<TwitterTweet> = gm_agent_twitter_tweets::table
            .filter(gm_agent_twitter_tweets::task_id.eq(task_id))
            .order(gm_agent_twitter_tweets::created_at.desc())
            .limit(page_size)
            .offset((page - 1) * page_size)
            .select(TwitterTweet::as_select())
            .load(&mut conn)?;

        let content_db_ids: Vec<i32> = items.iter().map(|tweet| tweet.id).collect();
        let valid_comment_counts =
            Self::load_twitter_valid_comment_counts(&mut conn, &content_db_ids)?;
        let dtos = Self::map_twitter_contents(items, &valid_comment_counts);

        Ok((dtos, total))
    }

    fn load_twitter_valid_comment_counts(
        conn: &mut PgConnection,
        tweet_db_ids: &[i32],
    ) -> Result<HashMap<i32, i64>, diesel::result::Error> {
        if tweet_db_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let counts: Vec<(i32, i64)> = gm_agent_twitter_comments::table
            .filter(gm_agent_twitter_comments::tweet_db_id.eq_any(tweet_db_ids))
            .group_by(gm_agent_twitter_comments::tweet_db_id)
            .select((
                gm_agent_twitter_comments::tweet_db_id,
                diesel::dsl::count_star(),
            ))
            .load(conn)?;

        Ok(counts.into_iter().collect())
    }

    fn map_twitter_contents(
        items: Vec<TwitterTweet>,
        valid_comment_counts: &HashMap<i32, i64>,
    ) -> Vec<UnifiedContentDto> {
        items
            .into_iter()
            .map(|t| UnifiedContentDto {
                id: t.id,
                task_id: t.task_id,
                content_id: t.twitter_tweet_id,
                content_type: "tweet".to_string(),
                platform: "twitter".to_string(),
                title: Some(t.full_text),
                author_name: t.screen_name,
                author_id: t.user_id,
                url: None,
                thumbnail_url: t.user_avatar,
                like_count: t.favorite_count,
                comment_count: t.reply_count,
                valid_comment_count: Self::valid_comment_count_for(t.id, valid_comment_counts),
                share_count: t.retweet_count,
                view_count: t.view_count,
                processed: true,
                replied: false,
                posted_at: t.tweet_created_at,
                created_at: t.created_at,
                campaign_id: t.campaign_id,
            })
            .collect()
    }
}
