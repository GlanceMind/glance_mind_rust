use crate::dto::agent_dto::{AgentCommentDto, UnifiedCommentDto, UnifiedCommentWithConfigDto};
use crate::dto::common::PageResponse;
use crate::repository::crawler_repository::{
    PLATFORM_NAME_FACEBOOK, PLATFORM_NAME_INSTAGRAM, PLATFORM_NAME_REDDIT, PLATFORM_NAME_TIKTOK,
    PLATFORM_NAME_TWITTER,
};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use glance_mind_db::entity::agent::{
    AgentComment, FacebookComment, InstagramComment, RedditComment, TwitterComment,
};
use glance_mind_db::entity::platform::Platform;
use glance_mind_db::schema::{
    gm_agent_comments, gm_agent_facebook_comments, gm_agent_instagram_comments,
    gm_agent_reddit_comments, gm_agent_twitter_comments, gm_platforms,
};

#[derive(Clone)]
pub struct AgentRepository {
    pool: Pool<ConnectionManager<PgConnection>>,
}

impl AgentRepository {
    pub fn new(pool: Pool<ConnectionManager<PgConnection>>) -> Self {
        Self { pool }
    }

    /// Legacy method - only queries TikTok comments
    pub fn get_video_comments(
        &self,
        video_id: i32,
        page: i64,
        per_page: i64,
    ) -> Result<PageResponse<AgentCommentDto>, diesel::result::Error> {
        let mut conn = self
            .pool
            .get()
            .map_err(|_| diesel::result::Error::NotFound)?;

        let offset = (page - 1) * per_page;

        // Get total count
        let total: i64 = gm_agent_comments::table
            .filter(gm_agent_comments::video_db_id.eq(video_id))
            .count()
            .get_result(&mut conn)?;

        // Get paginated results
        let comments: Vec<AgentComment> = gm_agent_comments::table
            .filter(gm_agent_comments::video_db_id.eq(video_id))
            .order(gm_agent_comments::id.desc())
            .limit(per_page)
            .offset(offset)
            .select(AgentComment::as_select())
            .load(&mut conn)?;

        let list = comments
            .into_iter()
            .map(AgentCommentDto::from_entity)
            .collect();

        Ok(PageResponse::new(list, total, page, per_page))
    }

    /// Get platform name by platform_id from database
    fn get_platform_name(&self, platform_id: i32) -> Result<String, diesel::result::Error> {
        let mut conn = self
            .pool
            .get()
            .map_err(|_| diesel::result::Error::NotFound)?;
        let platform: Platform = gm_platforms::table
            .find(platform_id)
            .select(Platform::as_select())
            .first(&mut conn)?;
        Ok(platform.name.to_uppercase())
    }

    /// Unified method to query comments by content_db_id and platform
    /// Uses database to resolve platform_id -> platform_name, then routes to correct table
    pub fn get_unified_comments(
        &self,
        content_db_id: i32,
        platform_id: i32,
        page: i64,
        per_page: i64,
    ) -> Result<PageResponse<UnifiedCommentDto>, diesel::result::Error> {
        // Get platform name from database (no hardcoded platform_id mapping)
        let platform_name = self.get_platform_name(platform_id)?;

        // Route to correct table based on platform name
        match platform_name.as_str() {
            PLATFORM_NAME_TIKTOK => self.get_tiktok_comments(content_db_id, page, per_page),
            PLATFORM_NAME_FACEBOOK => self.get_facebook_comments(content_db_id, page, per_page),
            PLATFORM_NAME_INSTAGRAM => self.get_instagram_comments(content_db_id, page, per_page),
            PLATFORM_NAME_REDDIT => self.get_reddit_comments(content_db_id, page, per_page),
            PLATFORM_NAME_TWITTER => self.get_twitter_comments(content_db_id, page, per_page),
            _ => self.get_tiktok_comments(content_db_id, page, per_page),
        }
    }

    fn get_tiktok_comments(
        &self,
        video_db_id: i32,
        page: i64,
        per_page: i64,
    ) -> Result<PageResponse<UnifiedCommentDto>, diesel::result::Error> {
        let mut conn = self
            .pool
            .get()
            .map_err(|_| diesel::result::Error::NotFound)?;

        let offset = (page - 1) * per_page;

        let total: i64 = gm_agent_comments::table
            .filter(gm_agent_comments::video_db_id.eq(video_db_id))
            .count()
            .get_result(&mut conn)?;

        let comments: Vec<AgentComment> = gm_agent_comments::table
            .filter(gm_agent_comments::video_db_id.eq(video_db_id))
            .order(gm_agent_comments::id.desc())
            .limit(per_page)
            .offset(offset)
            .select(AgentComment::as_select())
            .load(&mut conn)?;

        let list: Vec<UnifiedCommentDto> = comments
            .into_iter()
            .map(|c| UnifiedCommentDto {
                id: c.id,
                content_db_id: c.video_db_id,
                comment_id: c.comment_id,
                platform: "tiktok".to_string(),
                user_name: c.user_nickname,
                user_id: c.user_unique_id,
                content: c.content,
                parent_comment_id: None,
                like_count: None,
                reply_count: None,
                reason: c.reason,
                suggested_reply: c.suggested_reply,
                suggested_dm: c.suggested_dm,
                suggested_reply_post: c.suggested_reply_post,
                status: match c.status {
                    0 => "pending".to_string(),
                    1 => "processing".to_string(),
                    2 => "completed".to_string(),
                    _ => "pending".to_string(),
                },
                comment_created_at: c.create_time.map(|t| t.and_utc()),
                created_at: c.created_at,
                campaign_id: c.campaign_id,
            })
            .collect();

        Ok(PageResponse::new(list, total, page, per_page))
    }

    fn get_facebook_comments(
        &self,
        post_db_id: i32,
        page: i64,
        per_page: i64,
    ) -> Result<PageResponse<UnifiedCommentDto>, diesel::result::Error> {
        let mut conn = self
            .pool
            .get()
            .map_err(|_| diesel::result::Error::NotFound)?;

        let offset = (page - 1) * per_page;

        let total: i64 = gm_agent_facebook_comments::table
            .filter(gm_agent_facebook_comments::post_db_id.eq(post_db_id))
            .count()
            .get_result(&mut conn)?;

        let comments: Vec<FacebookComment> = gm_agent_facebook_comments::table
            .filter(gm_agent_facebook_comments::post_db_id.eq(post_db_id))
            .order(gm_agent_facebook_comments::id.desc())
            .limit(per_page)
            .offset(offset)
            .select(FacebookComment::as_select())
            .load(&mut conn)?;

        let list: Vec<UnifiedCommentDto> = comments
            .into_iter()
            .map(|c| UnifiedCommentDto {
                id: c.id,
                content_db_id: c.post_db_id,
                comment_id: c.facebook_comment_id,
                platform: "facebook".to_string(),
                user_name: c.comment_username,
                user_id: c.comment_user_id,
                content: Some(c.comment_text),
                parent_comment_id: c.parent_comment_id,
                like_count: c.like_count,
                reply_count: c.reply_count,
                reason: c.reason,
                suggested_reply: c.suggested_reply,
                suggested_dm: c.suggested_dm,
                suggested_reply_post: c.suggested_reply_post,
                status: Self::status_option_i16_to_string(c.status),
                comment_created_at: c.comment_created_at,
                created_at: c.created_at,
                campaign_id: c.campaign_id,
            })
            .collect();

        Ok(PageResponse::new(list, total, page, per_page))
    }

    fn get_instagram_comments(
        &self,
        post_db_id: i32,
        page: i64,
        per_page: i64,
    ) -> Result<PageResponse<UnifiedCommentDto>, diesel::result::Error> {
        let mut conn = self
            .pool
            .get()
            .map_err(|_| diesel::result::Error::NotFound)?;

        let offset = (page - 1) * per_page;

        let total: i64 = gm_agent_instagram_comments::table
            .filter(gm_agent_instagram_comments::post_db_id.eq(post_db_id))
            .count()
            .get_result(&mut conn)?;

        let comments: Vec<InstagramComment> = gm_agent_instagram_comments::table
            .filter(gm_agent_instagram_comments::post_db_id.eq(post_db_id))
            .order(gm_agent_instagram_comments::id.desc())
            .limit(per_page)
            .offset(offset)
            .select(InstagramComment::as_select())
            .load(&mut conn)?;

        let list: Vec<UnifiedCommentDto> = comments
            .into_iter()
            .map(|c| UnifiedCommentDto {
                id: c.id,
                content_db_id: c.post_db_id,
                comment_id: c.instagram_comment_id,
                platform: "instagram".to_string(),
                user_name: c.comment_username,
                user_id: c.comment_user_id,
                content: Some(c.comment_text),
                parent_comment_id: c.parent_comment_id,
                like_count: c.like_count.or(c.comment_like_count),
                reply_count: c.child_comment_count,
                reason: c.reason,
                suggested_reply: c.suggested_reply,
                suggested_dm: c.suggested_dm,
                suggested_reply_post: c.suggested_reply_post,
                status: Self::status_option_i16_to_string(c.status),
                comment_created_at: c.comment_created_at,
                created_at: c.created_at,
                campaign_id: c.campaign_id,
            })
            .collect();

        Ok(PageResponse::new(list, total, page, per_page))
    }

    fn get_reddit_comments(
        &self,
        post_db_id: i32,
        page: i64,
        per_page: i64,
    ) -> Result<PageResponse<UnifiedCommentDto>, diesel::result::Error> {
        let mut conn = self
            .pool
            .get()
            .map_err(|_| diesel::result::Error::NotFound)?;

        let offset = (page - 1) * per_page;

        let total: i64 = gm_agent_reddit_comments::table
            .filter(gm_agent_reddit_comments::post_db_id.eq(post_db_id))
            .count()
            .get_result(&mut conn)?;

        let comments: Vec<RedditComment> = gm_agent_reddit_comments::table
            .filter(gm_agent_reddit_comments::post_db_id.eq(post_db_id))
            .order(gm_agent_reddit_comments::id.desc())
            .limit(per_page)
            .offset(offset)
            .select(RedditComment::as_select())
            .load(&mut conn)?;

        let list: Vec<UnifiedCommentDto> = comments
            .into_iter()
            .map(|c| UnifiedCommentDto {
                id: c.id,
                content_db_id: c.post_db_id,
                comment_id: c.comment_id,
                platform: "reddit".to_string(),
                user_name: c.author,
                user_id: None,
                content: c.body,
                parent_comment_id: c.parent_id,
                like_count: c.score, // Reddit uses score
                reply_count: None,
                reason: c.reason,
                suggested_reply: c.suggested_reply,
                suggested_dm: c.suggested_dm,
                suggested_reply_post: c.suggested_reply_post,
                status: Self::status_option_i16_to_string(c.status),
                comment_created_at: c.comment_created_at,
                created_at: c.created_at,
                campaign_id: c.campaign_id,
            })
            .collect();

        Ok(PageResponse::new(list, total, page, per_page))
    }

    fn get_twitter_comments(
        &self,
        tweet_db_id: i32,
        page: i64,
        per_page: i64,
    ) -> Result<PageResponse<UnifiedCommentDto>, diesel::result::Error> {
        let mut conn = self
            .pool
            .get()
            .map_err(|_| diesel::result::Error::NotFound)?;

        let offset = (page - 1) * per_page;

        let total: i64 = gm_agent_twitter_comments::table
            .filter(gm_agent_twitter_comments::tweet_db_id.eq(tweet_db_id))
            .count()
            .get_result(&mut conn)?;

        let comments: Vec<TwitterComment> = gm_agent_twitter_comments::table
            .filter(gm_agent_twitter_comments::tweet_db_id.eq(tweet_db_id))
            .order(gm_agent_twitter_comments::id.desc())
            .limit(per_page)
            .offset(offset)
            .select(TwitterComment::as_select())
            .load(&mut conn)?;

        let list: Vec<UnifiedCommentDto> = comments
            .into_iter()
            .map(|c| UnifiedCommentDto {
                id: c.id,
                content_db_id: c.tweet_db_id,
                comment_id: c.twitter_comment_id,
                platform: "twitter".to_string(),
                user_name: c.comment_screen_name,
                user_id: c.comment_user_id,
                content: Some(c.comment_text),
                parent_comment_id: c.in_reply_to_status_id,
                like_count: c.favorite_count,
                reply_count: c.reply_count,
                reason: c.reason,
                suggested_reply: c.suggested_reply,
                suggested_dm: c.suggested_dm,
                suggested_reply_post: c.suggested_reply_post,
                status: Self::status_option_i16_to_string(c.status),
                comment_created_at: c.comment_created_at,
                created_at: c.created_at,
                campaign_id: c.campaign_id,
            })
            .collect();

        Ok(PageResponse::new(list, total, page, per_page))
    }

    pub fn get_comments_by_device(
        &self,
        device_id: &str,
        status_filter: Option<i16>,
        page: i64,
        per_page: i64,
    ) -> Result<PageResponse<crate::dto::agent_dto::CommentWithVideoDto>, diesel::result::Error>
    {
        use glance_mind_db::schema::{
            gm_agent_comments, gm_agent_videos, gm_campaigns, gm_crawler_tasks, gm_social_accounts,
            gm_social_groups,
        };

        let mut conn = self
            .pool
            .get()
            .map_err(|_| diesel::result::Error::NotFound)?;

        let offset = (page - 1) * per_page;
        let status_val = status_filter.unwrap_or(0);

        // Correct data flow:
        // device_id → social_accounts(group_id) → social_groups(id) →
        // campaigns(social_group_id) → crawler_tasks(campaign_id) →
        // agent_videos(task_id) → agent_comments(video_db_id)

        let query = gm_agent_comments::table
            .inner_join(
                gm_agent_videos::table.on(gm_agent_comments::video_db_id.eq(gm_agent_videos::id)),
            )
            .inner_join(
                gm_crawler_tasks::table.on(gm_agent_videos::task_id.eq(gm_crawler_tasks::id)),
            )
            .inner_join(gm_campaigns::table.on(gm_crawler_tasks::campaign_id.eq(gm_campaigns::id)))
            .inner_join(
                gm_social_groups::table
                    .on(gm_campaigns::social_group_id.eq(gm_social_groups::id.nullable())),
            )
            .inner_join(
                gm_social_accounts::table.on(gm_social_groups::id
                    .nullable()
                    .eq(gm_social_accounts::group_id)),
            )
            .filter(gm_social_accounts::device_id.eq(device_id))
            .filter(gm_agent_comments::status.eq(status_val));

        // Get total count
        let total: i64 = query.count().get_result(&mut conn)?;

        // Get paginated results - now selecting Campaign as well
        let results: Vec<(
            AgentComment,
            glance_mind_db::entity::agent::AgentVideo,
            glance_mind_db::entity::campaign::Campaign,
        )> = query
            .select((
                AgentComment::as_select(),
                glance_mind_db::entity::agent::AgentVideo::as_select(),
                glance_mind_db::entity::campaign::Campaign::as_select(),
            ))
            .order(gm_agent_comments::id.desc())
            .limit(per_page)
            .offset(offset)
            .load(&mut conn)?;

        // For each comment, randomly select a profile_name from the campaign's group
        let mut list = Vec::new();
        for (comment, video, campaign) in results {
            // Get a random profile_name from the campaign's social group
            let profile_name = if let Some(group_id) = campaign.social_group_id {
                use diesel::dsl::sql;
                use diesel::sql_types::Text;

                // Query to get a random profile_name from the group's accounts
                let random_profile_result: Result<Option<String>, _> = gm_social_accounts::table
                    .filter(gm_social_accounts::group_id.eq(group_id))
                    .filter(gm_social_accounts::profile_name.is_not_null())
                    .select(gm_social_accounts::profile_name)
                    .order(sql::<Text>("RANDOM()"))
                    .first(&mut conn);

                random_profile_result.unwrap_or_default()
            } else {
                None
            };

            list.push(crate::dto::agent_dto::CommentWithVideoDto {
                id: comment.id,
                comment_id: comment.comment_id,
                video_id: video.video_id.unwrap_or_default(),
                content: comment.content,
                status: comment.status,
                user_nickname: comment.user_nickname,
                user_unique_id: comment.user_unique_id,
                suggested_reply: comment.suggested_reply,
                suggested_dm: comment.suggested_dm,
                suggested_reply_post: comment.suggested_reply_post,
                reason: comment.reason,
                create_time: comment.create_time,
                created_at: comment.created_at,
                campaign_id: comment.campaign_id,
                auto_like: campaign.auto_like,
                auto_follow: campaign.auto_follow,
                auto_dm: campaign.auto_dm,
                auto_reply_comments: campaign.auto_reply_comments,
                auto_reply_post: campaign.auto_reply_post,
                profile_name,
            });
        }

        Ok(PageResponse::new(list, total, page, per_page))
    }

    pub fn update_comment_status(
        &self,
        comment_id: &str,
        new_status: i16,
    ) -> Result<usize, diesel::result::Error> {
        use glance_mind_db::schema::gm_agent_comments;

        let mut conn = self
            .pool
            .get()
            .map_err(|_| diesel::result::Error::NotFound)?;

        diesel::update(
            gm_agent_comments::table.filter(gm_agent_comments::comment_id.eq(comment_id)),
        )
        .set(gm_agent_comments::status.eq(new_status))
        .execute(&mut conn)
    }

    /// Get all videos for a campaign (for export)
    pub fn get_campaign_videos(
        &self,
        campaign_id: i32,
    ) -> Result<Vec<glance_mind_db::entity::agent::AgentVideo>, diesel::result::Error> {
        use glance_mind_db::schema::gm_agent_videos;

        let mut conn = self
            .pool
            .get()
            .map_err(|_| diesel::result::Error::NotFound)?;

        gm_agent_videos::table
            .filter(gm_agent_videos::campaign_id.eq(campaign_id))
            .order(gm_agent_videos::id.desc())
            .select(glance_mind_db::entity::agent::AgentVideo::as_select())
            .load(&mut conn)
    }

    /// Get all comments for a campaign (for export)
    pub fn get_campaign_comments(
        &self,
        campaign_id: i32,
    ) -> Result<Vec<AgentComment>, diesel::result::Error> {
        use glance_mind_db::schema::gm_agent_comments;

        let mut conn = self
            .pool
            .get()
            .map_err(|_| diesel::result::Error::NotFound)?;

        gm_agent_comments::table
            .filter(gm_agent_comments::campaign_id.eq(campaign_id))
            .order(gm_agent_comments::id.desc())
            .select(AgentComment::as_select())
            .load(&mut conn)
    }

    /// Unified method to get comments by device_id with platform support
    /// Routes to correct table based on platform name
    pub fn get_comments_by_device_unified(
        &self,
        device_id: &str,
        platform: &str,
        status_filter: Option<i16>,
        page: i64,
        per_page: i64,
    ) -> Result<PageResponse<UnifiedCommentWithConfigDto>, diesel::result::Error> {
        let platform_upper = platform.to_uppercase();
        match platform_upper.as_str() {
            PLATFORM_NAME_TIKTOK => {
                self.get_tiktok_comments_by_device(device_id, status_filter, page, per_page)
            }
            PLATFORM_NAME_FACEBOOK => {
                self.get_facebook_comments_by_device(device_id, status_filter, page, per_page)
            }
            PLATFORM_NAME_INSTAGRAM => {
                self.get_instagram_comments_by_device(device_id, status_filter, page, per_page)
            }
            PLATFORM_NAME_REDDIT => {
                self.get_reddit_comments_by_device(device_id, status_filter, page, per_page)
            }
            PLATFORM_NAME_TWITTER => {
                self.get_twitter_comments_by_device(device_id, status_filter, page, per_page)
            }
            _ => {
                // Default to TikTok for unknown platforms
                self.get_tiktok_comments_by_device(device_id, status_filter, page, per_page)
            }
        }
    }

    /// Get TikTok comments by device_id
    fn get_tiktok_comments_by_device(
        &self,
        device_id: &str,
        status_filter: Option<i16>,
        page: i64,
        per_page: i64,
    ) -> Result<PageResponse<UnifiedCommentWithConfigDto>, diesel::result::Error> {
        use glance_mind_db::schema::{
            gm_agent_comments, gm_agent_videos, gm_campaigns, gm_crawler_tasks, gm_social_accounts,
            gm_social_groups,
        };

        let mut conn = self
            .pool
            .get()
            .map_err(|_| diesel::result::Error::NotFound)?;

        let offset = (page - 1) * per_page;
        let status_val = status_filter.unwrap_or(0);

        let query = gm_agent_comments::table
            .inner_join(
                gm_agent_videos::table.on(gm_agent_comments::video_db_id.eq(gm_agent_videos::id)),
            )
            .inner_join(
                gm_crawler_tasks::table.on(gm_agent_videos::task_id.eq(gm_crawler_tasks::id)),
            )
            .inner_join(gm_campaigns::table.on(gm_crawler_tasks::campaign_id.eq(gm_campaigns::id)))
            .inner_join(
                gm_social_groups::table
                    .on(gm_campaigns::social_group_id.eq(gm_social_groups::id.nullable())),
            )
            .inner_join(
                gm_social_accounts::table.on(gm_social_groups::id
                    .nullable()
                    .eq(gm_social_accounts::group_id)),
            )
            .filter(gm_social_accounts::device_id.eq(device_id))
            .filter(gm_agent_comments::status.eq(status_val));

        let total: i64 = query.count().get_result(&mut conn)?;

        let results: Vec<(
            AgentComment,
            glance_mind_db::entity::agent::AgentVideo,
            glance_mind_db::entity::campaign::Campaign,
        )> = query
            .select((
                AgentComment::as_select(),
                glance_mind_db::entity::agent::AgentVideo::as_select(),
                glance_mind_db::entity::campaign::Campaign::as_select(),
            ))
            .order(gm_agent_comments::id.desc())
            .limit(per_page)
            .offset(offset)
            .load(&mut conn)?;

        let mut list = Vec::new();
        for (comment, video, campaign) in results {
            let profile_name = self.get_random_profile_name(&mut conn, campaign.social_group_id);

            // Build TikTok video URL: https://www.tiktok.com/@{author}/video/{video_id}
            let author_unique_id = video.author_unique_id.clone();
            let video_id = video.video_id.clone().unwrap_or_default();
            let content_url = video.url.clone().or_else(|| {
                author_unique_id
                    .as_ref()
                    .map(|author| format!("https://www.tiktok.com/@{}/video/{}", author, video_id))
            });

            list.push(UnifiedCommentWithConfigDto {
                id: comment.id,
                comment_id: comment.comment_id,
                content_id: video_id,
                platform: "tiktok".to_string(),
                content: comment.content,
                status: match comment.status {
                    0 => "pending".to_string(),
                    1 => "processing".to_string(),
                    2 => "completed".to_string(),
                    _ => "pending".to_string(),
                },
                user_nickname: comment.user_nickname,
                user_unique_id: comment.user_unique_id,
                suggested_reply: comment.suggested_reply,
                suggested_dm: comment.suggested_dm,
                suggested_reply_post: comment.suggested_reply_post,
                reason: comment.reason,
                create_time: comment.create_time,
                created_at: comment.created_at,
                campaign_id: comment.campaign_id,
                auto_like: campaign.auto_like,
                auto_follow: campaign.auto_follow,
                auto_dm: campaign.auto_dm,
                auto_reply_comments: campaign.auto_reply_comments,
                auto_reply_post: campaign.auto_reply_post,
                profile_name,
                // Platform-specific fields for TikTok
                content_url,
                content_type: Some("VIDEO".to_string()),
                author_unique_id,
                comment_url: None, // TikTok doesn't have direct comment URLs
            });
        }

        Ok(PageResponse::new(list, total, page, per_page))
    }

    /// Get Facebook comments by device_id
    fn get_facebook_comments_by_device(
        &self,
        device_id: &str,
        status_filter: Option<i16>,
        page: i64,
        per_page: i64,
    ) -> Result<PageResponse<UnifiedCommentWithConfigDto>, diesel::result::Error> {
        use glance_mind_db::schema::{
            gm_agent_facebook_comments, gm_agent_facebook_posts, gm_campaigns, gm_crawler_tasks,
            gm_social_accounts, gm_social_groups,
        };

        let mut conn = self
            .pool
            .get()
            .map_err(|_| diesel::result::Error::NotFound)?;

        let offset = (page - 1) * per_page;
        // Status is now i16: 0=pending, 1=processing, 2=completed
        let status_val = status_filter.unwrap_or(0);

        let query = gm_agent_facebook_comments::table
            .inner_join(
                gm_agent_facebook_posts::table
                    .on(gm_agent_facebook_comments::post_db_id.eq(gm_agent_facebook_posts::id)),
            )
            .inner_join(
                gm_crawler_tasks::table
                    .on(gm_agent_facebook_posts::task_id.eq(gm_crawler_tasks::id)),
            )
            .inner_join(gm_campaigns::table.on(gm_crawler_tasks::campaign_id.eq(gm_campaigns::id)))
            .inner_join(
                gm_social_groups::table
                    .on(gm_campaigns::social_group_id.eq(gm_social_groups::id.nullable())),
            )
            .inner_join(
                gm_social_accounts::table.on(gm_social_groups::id
                    .nullable()
                    .eq(gm_social_accounts::group_id)),
            )
            .filter(gm_social_accounts::device_id.eq(device_id))
            .filter(gm_agent_facebook_comments::status.eq(status_val));

        let total: i64 = query.count().get_result(&mut conn)?;

        let results: Vec<(
            FacebookComment,
            glance_mind_db::entity::agent::FacebookPost,
            glance_mind_db::entity::campaign::Campaign,
        )> = query
            .select((
                FacebookComment::as_select(),
                glance_mind_db::entity::agent::FacebookPost::as_select(),
                glance_mind_db::entity::campaign::Campaign::as_select(),
            ))
            .order(gm_agent_facebook_comments::id.desc())
            .limit(per_page)
            .offset(offset)
            .load(&mut conn)?;

        let mut list = Vec::new();
        for (comment, post, campaign) in results {
            let profile_name = self.get_random_profile_name(&mut conn, campaign.social_group_id);

            // Use post_url from comment, or fallback to post's url
            let content_url = comment.post_url.clone().or(post.url.clone());
            // Content type from post_type (e.g., "POST", "VIDEO", "REEL")
            let content_type = post.post_type.clone();

            list.push(UnifiedCommentWithConfigDto {
                id: comment.id,
                comment_id: comment.facebook_comment_id.clone(),
                content_id: post.facebook_post_id,
                platform: "facebook".to_string(),
                content: Some(comment.comment_text),
                status: Self::status_option_i16_to_string(comment.status),
                user_nickname: comment.comment_username,
                user_unique_id: comment.comment_user_id,
                suggested_reply: comment.suggested_reply,
                suggested_dm: comment.suggested_dm,
                suggested_reply_post: comment.suggested_reply_post,
                reason: comment.reason,
                create_time: comment.comment_created_at.map(|dt| dt.naive_utc()),
                created_at: comment.created_at,
                campaign_id: comment.campaign_id,
                auto_like: campaign.auto_like,
                auto_follow: campaign.auto_follow,
                auto_dm: campaign.auto_dm,
                auto_reply_comments: campaign.auto_reply_comments,
                auto_reply_post: campaign.auto_reply_post,
                profile_name,
                // Platform-specific fields for Facebook
                content_url,
                content_type,
                author_unique_id: post.author_id.clone(), // Facebook author ID
                comment_url: comment.comment_url.clone(), // Direct comment URL
            });
        }

        Ok(PageResponse::new(list, total, page, per_page))
    }

    /// Get Instagram comments by device_id
    fn get_instagram_comments_by_device(
        &self,
        device_id: &str,
        status_filter: Option<i16>,
        page: i64,
        per_page: i64,
    ) -> Result<PageResponse<UnifiedCommentWithConfigDto>, diesel::result::Error> {
        use glance_mind_db::schema::{
            gm_agent_instagram_comments, gm_agent_instagram_posts, gm_campaigns, gm_crawler_tasks,
            gm_social_accounts, gm_social_groups,
        };

        let mut conn = self
            .pool
            .get()
            .map_err(|_| diesel::result::Error::NotFound)?;

        let offset = (page - 1) * per_page;
        // Status is now i16: 0=pending, 1=processing, 2=completed
        let status_val = status_filter.unwrap_or(0);

        let query = gm_agent_instagram_comments::table
            .inner_join(
                gm_agent_instagram_posts::table
                    .on(gm_agent_instagram_comments::post_db_id.eq(gm_agent_instagram_posts::id)),
            )
            .inner_join(
                gm_crawler_tasks::table
                    .on(gm_agent_instagram_posts::task_id.eq(gm_crawler_tasks::id)),
            )
            .inner_join(gm_campaigns::table.on(gm_crawler_tasks::campaign_id.eq(gm_campaigns::id)))
            .inner_join(
                gm_social_groups::table
                    .on(gm_campaigns::social_group_id.eq(gm_social_groups::id.nullable())),
            )
            .inner_join(
                gm_social_accounts::table.on(gm_social_groups::id
                    .nullable()
                    .eq(gm_social_accounts::group_id)),
            )
            .filter(gm_social_accounts::device_id.eq(device_id))
            .filter(gm_agent_instagram_comments::status.eq(status_val));

        let total: i64 = query.count().get_result(&mut conn)?;

        let results: Vec<(
            InstagramComment,
            glance_mind_db::entity::agent::InstagramPost,
            glance_mind_db::entity::campaign::Campaign,
        )> = query
            .select((
                InstagramComment::as_select(),
                glance_mind_db::entity::agent::InstagramPost::as_select(),
                glance_mind_db::entity::campaign::Campaign::as_select(),
            ))
            .order(gm_agent_instagram_comments::id.desc())
            .limit(per_page)
            .offset(offset)
            .load(&mut conn)?;

        let mut list = Vec::new();
        for (comment, post, campaign) in results {
            let profile_name = self.get_random_profile_name(&mut conn, campaign.social_group_id);

            // Build Instagram URL: https://www.instagram.com/p/{code}/
            let code = post.code.clone();
            let content_url = Some(format!("https://www.instagram.com/p/{}/", code));
            // Content type from product_type (e.g., "feed", "reels", "igtv")
            let content_type = post.product_type.clone();

            list.push(UnifiedCommentWithConfigDto {
                id: comment.id,
                comment_id: comment.instagram_comment_id,
                content_id: code,
                platform: "instagram".to_string(),
                content: Some(comment.comment_text),
                status: Self::status_option_i16_to_string(comment.status),
                user_nickname: comment.comment_username.clone(),
                user_unique_id: comment.comment_user_id,
                suggested_reply: comment.suggested_reply,
                suggested_dm: comment.suggested_dm,
                suggested_reply_post: comment.suggested_reply_post,
                reason: comment.reason,
                create_time: comment.comment_created_at.map(|dt| dt.naive_utc()),
                created_at: comment.created_at,
                campaign_id: comment.campaign_id,
                auto_like: campaign.auto_like,
                auto_follow: campaign.auto_follow,
                auto_dm: campaign.auto_dm,
                auto_reply_comments: campaign.auto_reply_comments,
                auto_reply_post: campaign.auto_reply_post,
                profile_name,
                // Platform-specific fields for Instagram
                content_url,
                content_type,
                author_unique_id: post.owner_username.clone(), // Instagram owner username
                comment_url: None, // Instagram doesn't have direct comment URLs
            });
        }

        Ok(PageResponse::new(list, total, page, per_page))
    }

    /// Get Reddit comments by device_id
    fn get_reddit_comments_by_device(
        &self,
        device_id: &str,
        status_filter: Option<i16>,
        page: i64,
        per_page: i64,
    ) -> Result<PageResponse<UnifiedCommentWithConfigDto>, diesel::result::Error> {
        use glance_mind_db::schema::{
            gm_agent_reddit_comments, gm_agent_reddit_posts, gm_campaigns, gm_crawler_tasks,
            gm_social_accounts, gm_social_groups,
        };

        let mut conn = self
            .pool
            .get()
            .map_err(|_| diesel::result::Error::NotFound)?;

        let offset = (page - 1) * per_page;
        // Status is now i16: 0=pending, 1=processing, 2=completed
        let status_val = status_filter.unwrap_or(0);

        let query = gm_agent_reddit_comments::table
            .inner_join(
                gm_agent_reddit_posts::table
                    .on(gm_agent_reddit_comments::post_db_id.eq(gm_agent_reddit_posts::id)),
            )
            .inner_join(
                gm_crawler_tasks::table.on(gm_agent_reddit_posts::task_id.eq(gm_crawler_tasks::id)),
            )
            .inner_join(gm_campaigns::table.on(gm_crawler_tasks::campaign_id.eq(gm_campaigns::id)))
            .inner_join(
                gm_social_groups::table
                    .on(gm_campaigns::social_group_id.eq(gm_social_groups::id.nullable())),
            )
            .inner_join(
                gm_social_accounts::table.on(gm_social_groups::id
                    .nullable()
                    .eq(gm_social_accounts::group_id)),
            )
            .filter(gm_social_accounts::device_id.eq(device_id))
            .filter(gm_agent_reddit_comments::status.eq(status_val));

        let total: i64 = query.count().get_result(&mut conn)?;

        let results: Vec<(
            RedditComment,
            glance_mind_db::entity::agent::RedditPost,
            glance_mind_db::entity::campaign::Campaign,
        )> = query
            .select((
                RedditComment::as_select(),
                glance_mind_db::entity::agent::RedditPost::as_select(),
                glance_mind_db::entity::campaign::Campaign::as_select(),
            ))
            .order(gm_agent_reddit_comments::id.desc())
            .limit(per_page)
            .offset(offset)
            .load(&mut conn)?;

        let mut list = Vec::new();
        for (comment, post, campaign) in results {
            let profile_name = self.get_random_profile_name(&mut conn, campaign.social_group_id);

            // Use permalink for content URL, or build from subreddit/post_id
            let content_url = post
                .permalink
                .clone()
                .map(|p| format!("https://www.reddit.com{}", p))
                .or(post.url.clone());
            // Reddit posts can be video or text, check is_video flag
            let content_type = if post.is_video.unwrap_or(false) {
                Some("VIDEO".to_string())
            } else {
                Some("POST".to_string())
            };

            list.push(UnifiedCommentWithConfigDto {
                id: comment.id,
                comment_id: comment.comment_id.clone(),
                content_id: post.post_id,
                platform: "reddit".to_string(),
                content: comment.body,
                status: Self::status_option_i16_to_string(comment.status),
                user_nickname: comment.author.clone(),
                user_unique_id: comment.author.clone(),
                suggested_reply: comment.suggested_reply,
                suggested_dm: comment.suggested_dm,
                suggested_reply_post: comment.suggested_reply_post,
                reason: comment.reason,
                create_time: comment.comment_created_at.map(|dt| dt.naive_utc()),
                created_at: comment.created_at,
                campaign_id: comment.campaign_id,
                auto_like: campaign.auto_like,
                auto_follow: campaign.auto_follow,
                auto_dm: campaign.auto_dm,
                auto_reply_comments: campaign.auto_reply_comments,
                auto_reply_post: campaign.auto_reply_post,
                profile_name,
                // Platform-specific fields for Reddit
                content_url,
                content_type,
                author_unique_id: post.author.clone(), // Reddit post author
                comment_url: None, // Reddit doesn't have direct comment URLs in the same way
            });
        }

        Ok(PageResponse::new(list, total, page, per_page))
    }

    /// Get Twitter comments by device_id
    fn get_twitter_comments_by_device(
        &self,
        device_id: &str,
        status_filter: Option<i16>,
        page: i64,
        per_page: i64,
    ) -> Result<PageResponse<UnifiedCommentWithConfigDto>, diesel::result::Error> {
        use glance_mind_db::schema::{
            gm_agent_twitter_comments, gm_agent_twitter_tweets, gm_campaigns, gm_crawler_tasks,
            gm_social_accounts, gm_social_groups,
        };

        let mut conn = self
            .pool
            .get()
            .map_err(|_| diesel::result::Error::NotFound)?;

        let offset = (page - 1) * per_page;
        // Status is now i16: 0=pending, 1=processing, 2=completed
        let status_val = status_filter.unwrap_or(0);

        let query = gm_agent_twitter_comments::table
            .inner_join(
                gm_agent_twitter_tweets::table
                    .on(gm_agent_twitter_comments::tweet_db_id.eq(gm_agent_twitter_tweets::id)),
            )
            .inner_join(
                gm_crawler_tasks::table
                    .on(gm_agent_twitter_tweets::task_id.eq(gm_crawler_tasks::id)),
            )
            .inner_join(gm_campaigns::table.on(gm_crawler_tasks::campaign_id.eq(gm_campaigns::id)))
            .inner_join(
                gm_social_groups::table
                    .on(gm_campaigns::social_group_id.eq(gm_social_groups::id.nullable())),
            )
            .inner_join(
                gm_social_accounts::table.on(gm_social_groups::id
                    .nullable()
                    .eq(gm_social_accounts::group_id)),
            )
            .filter(gm_social_accounts::device_id.eq(device_id))
            .filter(gm_agent_twitter_comments::status.eq(status_val));

        let total: i64 = query.count().get_result(&mut conn)?;

        let results: Vec<(
            TwitterComment,
            glance_mind_db::entity::agent::TwitterTweet,
            glance_mind_db::entity::campaign::Campaign,
        )> = query
            .select((
                TwitterComment::as_select(),
                glance_mind_db::entity::agent::TwitterTweet::as_select(),
                glance_mind_db::entity::campaign::Campaign::as_select(),
            ))
            .order(gm_agent_twitter_comments::id.desc())
            .limit(per_page)
            .offset(offset)
            .load(&mut conn)?;

        let mut list = Vec::new();
        for (comment, tweet, campaign) in results {
            let profile_name = self.get_random_profile_name(&mut conn, campaign.social_group_id);

            // Build Twitter URL: https://twitter.com/{screen_name}/status/{tweet_id}
            let screen_name = tweet.screen_name.clone();
            let tweet_id = tweet.twitter_tweet_id.clone();
            let content_url = screen_name
                .as_ref()
                .map(|sn| format!("https://twitter.com/{}/status/{}", sn, tweet_id));
            // Twitter doesn't have content types like Facebook
            let content_type = Some("TWEET".to_string());

            list.push(UnifiedCommentWithConfigDto {
                id: comment.id,
                comment_id: comment.twitter_comment_id.clone(),
                content_id: tweet_id,
                platform: "twitter".to_string(),
                content: Some(comment.comment_text),
                status: Self::status_option_i16_to_string(comment.status),
                user_nickname: comment.comment_screen_name.clone(),
                user_unique_id: comment.comment_user_id,
                suggested_reply: comment.suggested_reply,
                suggested_dm: comment.suggested_dm,
                suggested_reply_post: comment.suggested_reply_post,
                reason: comment.reason,
                create_time: comment.comment_created_at.map(|dt| dt.naive_utc()),
                created_at: comment.created_at,
                campaign_id: comment.campaign_id,
                auto_like: campaign.auto_like,
                auto_follow: campaign.auto_follow,
                auto_dm: campaign.auto_dm,
                auto_reply_comments: campaign.auto_reply_comments,
                auto_reply_post: campaign.auto_reply_post,
                profile_name,
                // Platform-specific fields for Twitter
                content_url,
                content_type,
                author_unique_id: screen_name, // Twitter screen name as author
                comment_url: None,             // Twitter doesn't have direct comment URLs
            });
        }

        Ok(PageResponse::new(list, total, page, per_page))
    }

    /// Helper to convert i16 status to string
    fn status_i16_to_string(status: i16) -> String {
        match status {
            0 => "pending".to_string(),
            1 => "processing".to_string(),
            2 => "completed".to_string(),
            _ => "pending".to_string(),
        }
    }

    fn status_option_i16_to_string(status: Option<i16>) -> String {
        match status {
            Some(0) => "pending".to_string(),
            Some(1) => "processing".to_string(),
            Some(2) => "completed".to_string(),
            _ => "pending".to_string(),
        }
    }

    /// Helper to get random profile_name from a social group
    fn get_random_profile_name(
        &self,
        conn: &mut diesel::PgConnection,
        social_group_id: Option<i32>,
    ) -> Option<String> {
        use diesel::dsl::sql;
        use diesel::sql_types::Text;
        use glance_mind_db::schema::gm_social_accounts;

        if let Some(group_id) = social_group_id {
            let result: Result<Option<String>, _> = gm_social_accounts::table
                .filter(gm_social_accounts::group_id.eq(group_id))
                .filter(gm_social_accounts::profile_name.is_not_null())
                .select(gm_social_accounts::profile_name)
                .order(sql::<Text>("RANDOM()"))
                .first(conn);

            result.unwrap_or_default()
        } else {
            None
        }
    }
}
