use crate::schema::{
    gm_agent_comments, gm_agent_facebook_comments, gm_agent_facebook_posts,
    gm_agent_instagram_comments, gm_agent_instagram_posts, gm_agent_reddit_comments,
    gm_agent_reddit_posts, gm_agent_twitter_comments, gm_agent_twitter_tweets, gm_agent_videos,
};
use bigdecimal::BigDecimal;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

// ==================== TikTok ====================

#[derive(Debug, Queryable, Selectable, Serialize, Deserialize, Clone)]
#[diesel(table_name = gm_agent_comments)]
pub struct AgentComment {
    pub id: i32,
    pub video_db_id: i32,
    pub comment_id: String,
    pub user_nickname: Option<String>,
    pub user_unique_id: Option<String>,
    pub content: Option<String>,
    pub reason: Option<String>,
    pub suggested_reply: Option<String>,
    pub create_time: Option<chrono::NaiveDateTime>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub campaign_id: Option<i32>,
    pub status: i16,
    pub suggested_dm: Option<String>,
    pub suggested_reply_post: Option<String>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Queryable, Selectable, Serialize, Deserialize, Clone)]
#[diesel(table_name = gm_agent_videos)]
pub struct AgentVideo {
    pub id: i32,
    pub video_id: Option<String>,
    pub author: Option<String>,
    pub description: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub task_id: i32,
    pub campaign_id: Option<i32>,
    pub like_count: Option<i32>,
    pub comment_count: Option<i32>,
    pub share_count: Option<i32>,
    pub play_count: Option<i32>,
    pub publish_time: Option<i64>,
    pub author_unique_id: Option<String>,
    pub url: Option<String>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

// ==================== Facebook ====================

#[derive(Debug, Queryable, Selectable, Serialize, Deserialize, Clone)]
#[diesel(table_name = gm_agent_facebook_posts)]
pub struct FacebookPost {
    pub id: i32,
    pub task_id: i32,
    pub campaign_id: Option<i32>,
    pub facebook_post_id: String,
    pub post_type: Option<String>,
    pub url: Option<String>,
    pub message: Option<String>,
    pub message_rich: Option<String>,
    pub timestamp: Option<i64>,
    pub posted_at: Option<chrono::DateTime<chrono::Utc>>,
    pub reactions_count: Option<i32>,
    pub comments_count: Option<i32>,
    pub reshare_count: Option<i32>,
    pub reactions_like: Option<i32>,
    pub reactions_love: Option<i32>,
    pub reactions_haha: Option<i32>,
    pub reactions_wow: Option<i32>,
    pub reactions_sad: Option<i32>,
    pub reactions_angry: Option<i32>,
    pub reactions_care: Option<i32>,
    pub author_id: Option<String>,
    pub author_name: Option<String>,
    pub author_url: Option<String>,
    pub author_profile_picture_url: Option<String>,
    pub author_title: Option<String>,
    pub has_image: Option<bool>,
    pub image_url: Option<String>,
    pub image_width: Option<i32>,
    pub image_height: Option<i32>,
    pub image_id: Option<String>,
    pub has_video: Option<bool>,
    pub video_thumbnail: Option<String>,
    pub external_url: Option<String>,
    pub attached_post_url: Option<String>,
    pub comments_id: Option<String>,
    pub shares_id: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Queryable, Selectable, Serialize, Deserialize, Clone)]
#[diesel(table_name = gm_agent_facebook_comments)]
pub struct FacebookComment {
    pub id: i32,
    pub post_db_id: i32,
    pub campaign_id: Option<i32>,
    pub facebook_comment_id: String,
    pub parent_comment_id: Option<String>,
    pub comment_url: Option<String>,
    pub comment_text: String,
    pub reason: Option<String>,
    pub suggested_reply: Option<String>,
    pub suggested_dm: Option<String>,
    pub suggested_reply_post: Option<String>,
    pub status: i16,
    pub comment_user_id: Option<String>,
    pub comment_username: Option<String>,
    pub comment_user_url: Option<String>,
    pub comment_user_profile_picture: Option<String>,
    pub like_count: Option<i32>,
    pub reply_count: Option<i32>,
    pub threading_depth: Option<i32>,
    pub created_at_ts: Option<i64>,
    pub comment_created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub facebook_post_id: Option<String>,
    pub post_url: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

// ==================== Instagram ====================

#[derive(Debug, Queryable, Selectable, Serialize, Deserialize, Clone)]
#[diesel(table_name = gm_agent_instagram_posts)]
pub struct InstagramPost {
    pub id: i32,
    pub task_id: i32,
    pub campaign_id: Option<i32>,
    pub code: String,
    pub instagram_id: Option<String>,
    pub media_type: Option<i32>,
    pub product_type: Option<String>,
    pub caption_text: Option<String>,
    pub owner_username: Option<String>,
    pub owner_id: Option<String>,
    pub owner_full_name: Option<String>,
    pub media_url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub like_count: Option<i32>,
    pub comment_count: Option<i32>,
    pub play_count: Option<i32>,
    pub taken_at_ts: Option<i64>,
    pub posted_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Queryable, Selectable, Serialize, Deserialize, Clone)]
#[diesel(table_name = gm_agent_instagram_comments)]
pub struct InstagramComment {
    pub id: i32,
    pub post_db_id: i32,
    pub campaign_id: Option<i32>,
    pub instagram_comment_id: String,
    pub parent_comment_id: Option<String>,
    pub comment_text: String,
    pub reason: Option<String>,
    pub suggested_reply: Option<String>,
    pub status: i16,
    pub comment_user_id: Option<String>,
    pub comment_username: Option<String>,
    pub comment_user_full_name: Option<String>,
    pub like_count: Option<i32>,
    pub comment_like_count: Option<i32>,
    pub child_comment_count: Option<i32>,
    pub created_at_ts: Option<i64>,
    pub comment_created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub suggested_dm: Option<String>,
    pub suggested_reply_post: Option<String>,
}

// ==================== Reddit ====================

#[derive(Debug, Queryable, Selectable, Serialize, Deserialize, Clone)]
#[diesel(table_name = gm_agent_reddit_posts)]
pub struct RedditPost {
    pub id: i32,
    pub task_id: i32,
    pub campaign_id: Option<i32>,
    pub post_id: String,
    pub post_name: String,
    pub title: String,
    pub selftext: Option<String>,
    pub author: Option<String>,
    pub subreddit: String,
    pub url: Option<String>,
    pub permalink: Option<String>,
    pub domain: Option<String>,
    pub thumbnail: Option<String>,
    pub score: Option<i32>,
    pub upvote_ratio: Option<BigDecimal>,
    pub num_comments: Option<i32>,
    pub is_video: Option<bool>,
    pub post_created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Queryable, Selectable, Serialize, Deserialize, Clone)]
#[diesel(table_name = gm_agent_reddit_comments)]
pub struct RedditComment {
    pub id: i32,
    pub post_db_id: i32,
    pub campaign_id: Option<i32>,
    pub comment_id: String,
    pub comment_name: String,
    pub author: Option<String>,
    pub body: Option<String>,
    pub reason: Option<String>,
    pub suggested_reply: Option<String>,
    pub status: i16,
    pub score: Option<i32>,
    pub parent_id: Option<String>,
    pub is_reply: Option<bool>,
    pub depth: Option<i32>,
    pub comment_created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub suggested_dm: Option<String>,
    pub suggested_reply_post: Option<String>,
}

// ==================== Twitter ====================

#[derive(Debug, Queryable, Selectable, Serialize, Deserialize, Clone)]
#[diesel(table_name = gm_agent_twitter_tweets)]
pub struct TwitterTweet {
    pub id: i32,
    pub task_id: i32,
    pub campaign_id: Option<i32>,
    pub twitter_tweet_id: String,
    pub conversation_id: Option<String>,
    pub full_text: String,
    pub lang: Option<String>,
    pub screen_name: Option<String>,
    pub user_name: Option<String>,
    pub user_id: Option<String>,
    pub user_description: Option<String>,
    pub user_followers_count: Option<i32>,
    pub user_avatar: Option<String>,
    pub user_verified: Option<bool>,
    pub media_urls: Option<Vec<Option<String>>>,
    pub has_media: Option<bool>,
    pub favorite_count: Option<i32>,
    pub retweet_count: Option<i32>,
    pub reply_count: Option<i32>,
    pub quote_count: Option<i32>,
    pub bookmark_count: Option<i32>,
    pub view_count: Option<i32>,
    pub is_reply: Option<bool>,
    pub in_reply_to_status_id: Option<String>,
    pub in_reply_to_user_id: Option<String>,
    pub created_at_str: Option<String>,
    pub created_at_ts: Option<i64>,
    pub tweet_created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Queryable, Selectable, Serialize, Deserialize, Clone)]
#[diesel(table_name = gm_agent_twitter_comments)]
pub struct TwitterComment {
    pub id: i32,
    pub tweet_db_id: i32,
    pub campaign_id: Option<i32>,
    pub twitter_comment_id: String,
    pub conversation_id: Option<String>,
    pub comment_screen_name: Option<String>,
    pub comment_user_name: Option<String>,
    pub comment_user_id: Option<String>,
    pub comment_user_followers: Option<i32>,
    pub comment_text: String,
    pub reason: Option<String>,
    pub suggested_reply: Option<String>,
    pub status: i16,
    pub favorite_count: Option<i32>,
    pub retweet_count: Option<i32>,
    pub reply_count: Option<i32>,
    pub in_reply_to_status_id: Option<String>,
    pub is_reply: Option<bool>,
    pub media_urls: Option<Vec<Option<String>>>,
    pub has_media: Option<bool>,
    pub created_at_str: Option<String>,
    pub created_at_ts: Option<i64>,
    pub comment_created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub suggested_dm: Option<String>,
    pub suggested_reply_post: Option<String>,
}
