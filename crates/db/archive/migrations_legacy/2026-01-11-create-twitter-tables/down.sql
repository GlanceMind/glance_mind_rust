-- Twitter Tables Migration (Down)
-- Drop tables in reverse order (comments first, then tweets)

DROP TABLE IF EXISTS gm_agent_twitter_comments;
DROP TABLE IF EXISTS gm_agent_twitter_tweets;
