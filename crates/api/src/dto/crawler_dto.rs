use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CrawlerTaskDto {
    pub id: i32,
    pub campaign_id: i32,
    pub keywords: Option<Vec<String>>, // Simplify Option<Vec<Option<String>>> to Vec<String> for API if possible, but let's match entity for now or clean up.
    pub max_count: i32,
    pub process_count: i32,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

/// Legacy DTO for backward compatibility
#[derive(Debug, Serialize, Deserialize)]
pub struct CrawlerResultDto {
    pub id: i32,
    pub task_id: i32,
    pub video_id: String,
    pub video_title: Option<String>,
    pub comment_count: Option<i32>,
    pub view_count: Option<i32>,
    pub like_count: Option<i32>,
    pub share_count: Option<i32>,
    pub play_count: Option<i32>,
    pub author_name: Option<String>,
    pub processed: bool,
    pub replied: bool,
    pub created_at: DateTime<Utc>,
}

/// Unified content DTO for all platforms (TikTok, Facebook, Instagram, Reddit, Twitter)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UnifiedContentDto {
    pub id: i32,
    pub task_id: i32,
    pub content_id: String,    // video_id / facebook_post_id / code / post_id / twitter_tweet_id
    pub content_type: String,  // "video" / "post" / "tweet" / "reel"
    pub platform: String,      // "tiktok" / "facebook" / "instagram" / "reddit" / "twitter"
    pub title: Option<String>, // description / message / caption_text / title / full_text
    pub author_name: Option<String>,
    pub author_id: Option<String>,
    pub url: Option<String>,
    pub thumbnail_url: Option<String>,

    // Unified engagement metrics
    pub like_count: Option<i32>,    // like_count / reactions_count / score / favorite_count
    pub comment_count: Option<i32>, // comment_count / comments_count / num_comments / reply_count
    pub share_count: Option<i32>,   // share_count / reshare_count / retweet_count
    pub view_count: Option<i32>,    // play_count / view_count

    pub processed: bool,
    pub replied: bool,
    pub posted_at: Option<DateTime<Utc>>, // Original post time
    pub created_at: DateTime<Utc>,        // Crawled time
    pub campaign_id: Option<i32>,
}
