use crate::config::database::DBPool;
use diesel::prelude::*;
use diesel::SelectableHelper;
use glance_mind_db::entity::crawler::{CrawlerResult, CrawlerTask};
use glance_mind_db::schema::{
    gm_crawler_results as crawler_results, gm_crawler_tasks as crawler_tasks,
};

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

    pub async fn find_results_by_campaign_id(
        &self,
        c_id: i32,
        page: i64,
        page_size: i64,
    ) -> Result<(Vec<CrawlerResult>, i64), diesel::result::Error> {
        use glance_mind_db::entity::agent::AgentVideo;
        use glance_mind_db::schema::gm_agent_videos as agent_videos;

        let mut conn = self.pool.get().unwrap();

        // Count total agent videos for this campaign
        let total: i64 = agent_videos::table
            .inner_join(crawler_tasks::table)
            .filter(crawler_tasks::campaign_id.eq(c_id))
            .count()
            .get_result(&mut conn)?;

        // Fetch agent videos with proper join
        let agent_videos_list: Vec<AgentVideo> = agent_videos::table
            .inner_join(crawler_tasks::table)
            .filter(crawler_tasks::campaign_id.eq(c_id))
            .order(agent_videos::created_at.desc())
            .limit(page_size)
            .offset((page - 1) * page_size)
            .select(AgentVideo::as_select())
            .load(&mut conn)?;

        // Convert AgentVideo to CrawlerResult
        // Note: share_count and play_count are not in CrawlerResult database schema
        // They are only available in the DTO layer
        let items: Vec<CrawlerResult> = agent_videos_list
            .into_iter()
            .map(|av| {
                CrawlerResult {
                    id: av.id,
                    task_id: av.task_id,
                    video_id: av.video_id.unwrap_or_default(),
                    video_title: av.description.clone(),
                    comment_count: av.comment_count,
                    view_count: av.play_count, // Use play_count as view_count
                    like_count: av.like_count,
                    author_name: av.author,
                    processed: true, // Agent videos are processed by definition
                    replied: false,  // TODO: Track this separately if needed
                    created_at: av.created_at,
                    updated_at: Some(av.created_at),
                }
            })
            .collect();

        Ok((items, total))
    }
}
