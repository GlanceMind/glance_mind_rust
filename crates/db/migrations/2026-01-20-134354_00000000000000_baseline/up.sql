--
-- PostgreSQL database dump
--

-- Dumped from database version 15.13
-- Dumped by pg_dump version 17.5

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
-- SET transaction_timeout = 0;  -- Removed: PostgreSQL 17+ only
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

--
-- Name: diesel_manage_updated_at(regclass); Type: FUNCTION; Schema: public; Owner: -
--

CREATE OR REPLACE FUNCTION public.diesel_manage_updated_at(_tbl regclass) RETURNS void
    LANGUAGE plpgsql
    AS $$
BEGIN
    EXECUTE format('CREATE TRIGGER set_updated_at BEFORE UPDATE ON %s
                    FOR EACH ROW EXECUTE PROCEDURE diesel_set_updated_at()', _tbl);
END;
$$;


--
-- Name: diesel_set_updated_at(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE OR REPLACE FUNCTION public.diesel_set_updated_at() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    IF (
        NEW IS DISTINCT FROM OLD AND
        NEW.updated_at IS NOT DISTINCT FROM OLD.updated_at
    ) THEN
        NEW.updated_at := current_timestamp;
    END IF;
    RETURN NEW;
END;
$$;


--
-- Name: update_instagram_comments_updated_at(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE OR REPLACE FUNCTION public.update_instagram_comments_updated_at() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$;


--
-- Name: update_instagram_posts_updated_at(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE OR REPLACE FUNCTION public.update_instagram_posts_updated_at() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$;


--
-- Name: update_referrals_updated_at(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE OR REPLACE FUNCTION public.update_referrals_updated_at() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$;


SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: gm_agent_comments; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE IF NOT EXISTS public.gm_agent_comments (
    id integer NOT NULL,
    video_db_id integer NOT NULL,
    comment_id character varying(255) NOT NULL,
    user_nickname character varying(255),
    user_unique_id character varying(255),
    content text,
    reason text,
    suggested_reply text,
    create_time timestamp without time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    campaign_id integer,
    status smallint DEFAULT 0 NOT NULL,
    suggested_dm text,
    suggested_reply_post text
);


--
-- Name: agent_comments_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE IF NOT EXISTS public.agent_comments_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: agent_comments_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.agent_comments_id_seq OWNED BY public.gm_agent_comments.id;


--
-- Name: gm_agent_videos; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE IF NOT EXISTS public.gm_agent_videos (
    id integer NOT NULL,
    video_id character varying(255),
    author character varying(255),
    description text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    task_id integer NOT NULL,
    campaign_id integer,
    like_count integer DEFAULT 0,
    comment_count integer DEFAULT 0,
    share_count integer DEFAULT 0,
    play_count integer DEFAULT 0,
    publish_time bigint DEFAULT 0,
    author_unique_id character varying(255),
    url text
);


--
-- Name: agent_videos_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE IF NOT EXISTS public.agent_videos_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: agent_videos_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.agent_videos_id_seq OWNED BY public.gm_agent_videos.id;


--
-- Name: gm_ai_models; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE IF NOT EXISTS public.gm_ai_models (
    id integer NOT NULL,
    name character varying NOT NULL,
    provider character varying NOT NULL,
    model_key character varying NOT NULL,
    cost_multiplier numeric(10,2) DEFAULT 1.0 NOT NULL,
    is_active boolean DEFAULT true NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone,
    model_type character varying(50) DEFAULT 'chat'::character varying NOT NULL,
    CONSTRAINT valid_model_type CHECK (((model_type)::text = ANY ((ARRAY['chat'::character varying, 'video'::character varying])::text[])))
);


--
-- Name: ai_models_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE IF NOT EXISTS public.ai_models_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: ai_models_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.ai_models_id_seq OWNED BY public.gm_ai_models.id;


--
-- Name: gm_campaign_accounts; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE IF NOT EXISTS public.gm_campaign_accounts (
    id integer NOT NULL,
    campaign_id integer NOT NULL,
    account_id integer NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone
);


--
-- Name: campaign_accounts_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE IF NOT EXISTS public.campaign_accounts_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: campaign_accounts_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.campaign_accounts_id_seq OWNED BY public.gm_campaign_accounts.id;


--
-- Name: gm_campaigns; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE IF NOT EXISTS public.gm_campaigns (
    id integer NOT NULL,
    user_id integer NOT NULL,
    name character varying NOT NULL,
    status character varying DEFAULT 'DRAFT'::character varying NOT NULL,
    platform_id integer NOT NULL,
    region_id integer NOT NULL,
    ai_model_id integer NOT NULL,
    target_audience text,
    product_prompt text DEFAULT ''::text NOT NULL,
    schedule_config jsonb,
    enable_ai_refactor boolean DEFAULT false,
    persona_id integer,
    max_scan_count integer DEFAULT 1000,
    budget_cap numeric(10,2),
    end_date timestamp with time zone,
    schedule_type character varying NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone,
    keyword text,
    social_group_id integer,
    call_to_action text,
    tone_of_voice text,
    additional_info text,
    total_scanned integer DEFAULT 0 NOT NULL,
    auto_like boolean DEFAULT true NOT NULL,
    auto_follow boolean DEFAULT true NOT NULL,
    auto_dm boolean DEFAULT true NOT NULL,
    pending_consumption numeric DEFAULT 0 NOT NULL,
    actual_consumption numeric DEFAULT 0 NOT NULL,
    is_frozen boolean DEFAULT false NOT NULL,
    search_options jsonb DEFAULT '{}'::jsonb,
    auto_reply_comments boolean DEFAULT true NOT NULL,
    auto_reply_post boolean DEFAULT true NOT NULL
);


--
-- Name: campaigns_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE IF NOT EXISTS public.campaigns_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: campaigns_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.campaigns_id_seq OWNED BY public.gm_campaigns.id;


--
-- Name: gm_crawler_results; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE IF NOT EXISTS public.gm_crawler_results (
    id integer NOT NULL,
    task_id integer NOT NULL,
    video_id character varying(255) NOT NULL,
    video_title text,
    comment_count integer,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone,
    view_count integer DEFAULT 0,
    author_name character varying(255),
    processed boolean DEFAULT false NOT NULL,
    replied boolean DEFAULT false NOT NULL,
    like_count integer
);


--
-- Name: crawler_results_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE IF NOT EXISTS public.crawler_results_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: crawler_results_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.crawler_results_id_seq OWNED BY public.gm_crawler_results.id;


--
-- Name: crawler_task_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE IF NOT EXISTS public.crawler_task_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: gm_crawler_tasks; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE IF NOT EXISTS public.gm_crawler_tasks (
    id integer NOT NULL,
    campaign_id integer NOT NULL,
    keywords text[],
    max_count integer NOT NULL,
    process_count integer DEFAULT 0 NOT NULL,
    status character varying(50) DEFAULT 'init'::character varying NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone,
    search_offset integer DEFAULT 0 NOT NULL,
    search_limit integer DEFAULT 10 NOT NULL
);


--
-- Name: crawler_tasks_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE IF NOT EXISTS public.crawler_tasks_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: crawler_tasks_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.crawler_tasks_id_seq OWNED BY public.gm_crawler_tasks.id;


--
-- Name: gm_admin_users; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE IF NOT EXISTS public.gm_admin_users (
    id integer NOT NULL,
    username character varying(50) NOT NULL,
    email character varying(255) NOT NULL,
    password_hash character varying(255) NOT NULL,
    full_name character varying(255) NOT NULL,
    role character varying(20) DEFAULT 'admin'::character varying NOT NULL,
    is_active boolean DEFAULT true NOT NULL,
    last_login_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone
);


--
-- Name: gm_admin_users_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE IF NOT EXISTS public.gm_admin_users_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: gm_admin_users_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.gm_admin_users_id_seq OWNED BY public.gm_admin_users.id;


--
-- Name: gm_agent_instagram_comments; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE IF NOT EXISTS public.gm_agent_instagram_comments (
    id integer NOT NULL,
    post_db_id integer NOT NULL,
    campaign_id integer,
    instagram_comment_id character varying(255) NOT NULL,
    parent_comment_id character varying(255),
    comment_text text NOT NULL,
    reason text,
    suggested_reply text,
    status character varying(50) DEFAULT 'PENDING'::character varying,
    comment_user_id character varying(255),
    comment_username character varying(255),
    comment_user_full_name character varying(255),
    like_count integer DEFAULT 0,
    comment_like_count integer DEFAULT 0,
    child_comment_count integer DEFAULT 0,
    created_at_ts bigint,
    comment_created_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone,
    suggested_dm text,
    suggested_reply_post text
);


--
-- Name: gm_agent_instagram_comments_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE IF NOT EXISTS public.gm_agent_instagram_comments_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: gm_agent_instagram_comments_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.gm_agent_instagram_comments_id_seq OWNED BY public.gm_agent_instagram_comments.id;


--
-- Name: gm_agent_instagram_posts; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE IF NOT EXISTS public.gm_agent_instagram_posts (
    id integer NOT NULL,
    task_id integer NOT NULL,
    campaign_id integer,
    code character varying(255) NOT NULL,
    instagram_id character varying(255),
    media_type integer DEFAULT 1,
    product_type character varying(50),
    caption_text text DEFAULT ''::text,
    owner_username character varying(255),
    owner_id character varying(255),
    owner_full_name character varying(255),
    media_url text,
    thumbnail_url text,
    like_count integer DEFAULT 0,
    comment_count integer DEFAULT 0,
    play_count integer DEFAULT 0,
    taken_at_ts bigint,
    posted_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone
);


--
-- Name: gm_agent_instagram_posts_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE IF NOT EXISTS public.gm_agent_instagram_posts_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: gm_agent_instagram_posts_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.gm_agent_instagram_posts_id_seq OWNED BY public.gm_agent_instagram_posts.id;


--
-- Name: gm_agent_reddit_comments; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE IF NOT EXISTS public.gm_agent_reddit_comments (
    id integer NOT NULL,
    post_db_id integer NOT NULL,
    campaign_id integer,
    comment_id character varying(255) NOT NULL,
    comment_name character varying(255) NOT NULL,
    author character varying(255),
    body text,
    reason text,
    suggested_reply text,
    status character varying(50) DEFAULT 'PENDING'::character varying,
    score integer DEFAULT 0,
    parent_id character varying(255),
    is_reply boolean DEFAULT false,
    depth integer DEFAULT 0,
    comment_created_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone,
    suggested_dm text,
    suggested_reply_post text
);


--
-- Name: gm_agent_reddit_comments_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE IF NOT EXISTS public.gm_agent_reddit_comments_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: gm_agent_reddit_comments_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.gm_agent_reddit_comments_id_seq OWNED BY public.gm_agent_reddit_comments.id;


--
-- Name: gm_agent_reddit_posts; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE IF NOT EXISTS public.gm_agent_reddit_posts (
    id integer NOT NULL,
    task_id integer NOT NULL,
    campaign_id integer,
    post_id character varying(255) NOT NULL,
    post_name character varying(255) NOT NULL,
    title text NOT NULL,
    selftext text DEFAULT ''::text,
    author character varying(255),
    subreddit character varying(255) NOT NULL,
    url text,
    permalink text,
    domain character varying(255),
    thumbnail text,
    score integer DEFAULT 0,
    upvote_ratio numeric(5,4) DEFAULT 0.0,
    num_comments integer DEFAULT 0,
    is_video boolean DEFAULT false,
    post_created_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone
);


--
-- Name: gm_agent_reddit_posts_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE IF NOT EXISTS public.gm_agent_reddit_posts_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: gm_agent_reddit_posts_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.gm_agent_reddit_posts_id_seq OWNED BY public.gm_agent_reddit_posts.id;


--
-- Name: gm_agent_twitter_comments; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE IF NOT EXISTS public.gm_agent_twitter_comments (
    id integer NOT NULL,
    tweet_db_id integer NOT NULL,
    campaign_id integer,
    twitter_comment_id character varying(255) NOT NULL,
    conversation_id character varying(255),
    comment_screen_name character varying(255),
    comment_user_name character varying(255),
    comment_user_id character varying(255),
    comment_user_followers integer DEFAULT 0,
    comment_text text NOT NULL,
    reason text,
    suggested_reply text,
    status character varying(50) DEFAULT 'PENDING'::character varying,
    favorite_count integer DEFAULT 0,
    retweet_count integer DEFAULT 0,
    reply_count integer DEFAULT 0,
    in_reply_to_status_id character varying(255),
    is_reply boolean DEFAULT true,
    media_urls text[],
    has_media boolean DEFAULT false,
    created_at_str character varying(255),
    created_at_ts bigint,
    comment_created_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone,
    suggested_dm text,
    suggested_reply_post text
);


--
-- Name: gm_agent_twitter_comments_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE IF NOT EXISTS public.gm_agent_twitter_comments_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: gm_agent_twitter_comments_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.gm_agent_twitter_comments_id_seq OWNED BY public.gm_agent_twitter_comments.id;


--
-- Name: gm_agent_twitter_tweets; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE IF NOT EXISTS public.gm_agent_twitter_tweets (
    id integer NOT NULL,
    task_id integer NOT NULL,
    campaign_id integer,
    twitter_tweet_id character varying(255) NOT NULL,
    conversation_id character varying(255),
    full_text text DEFAULT ''::text NOT NULL,
    lang character varying(10),
    screen_name character varying(255),
    user_name character varying(255),
    user_id character varying(255),
    user_description text,
    user_followers_count integer DEFAULT 0,
    user_avatar text,
    user_verified boolean DEFAULT false,
    media_urls text[],
    has_media boolean DEFAULT false,
    favorite_count integer DEFAULT 0,
    retweet_count integer DEFAULT 0,
    reply_count integer DEFAULT 0,
    quote_count integer DEFAULT 0,
    bookmark_count integer DEFAULT 0,
    view_count integer DEFAULT 0,
    is_reply boolean DEFAULT false,
    in_reply_to_status_id character varying(255),
    in_reply_to_user_id character varying(255),
    created_at_str character varying(255),
    created_at_ts bigint,
    tweet_created_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone
);


--
-- Name: gm_agent_twitter_tweets_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE IF NOT EXISTS public.gm_agent_twitter_tweets_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: gm_agent_twitter_tweets_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.gm_agent_twitter_tweets_id_seq OWNED BY public.gm_agent_twitter_tweets.id;


--
-- Name: gm_ai_video_models; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE IF NOT EXISTS public.gm_ai_video_models (
    id integer NOT NULL,
    model_key character varying(100) NOT NULL,
    model_name character varying(255) NOT NULL,
    provider character varying(100) NOT NULL,
    description text,
    features jsonb,
    cost_per_generation numeric NOT NULL,
    cost_per_upload numeric,
    api_endpoint character varying(500),
    model_version character varying(50),
    max_prompt_length integer,
    supported_formats jsonb,
    max_image_size_mb integer,
    estimated_time_minutes integer,
    daily_limit integer,
    is_active boolean DEFAULT true NOT NULL,
    is_default boolean DEFAULT false NOT NULL,
    sort_order integer DEFAULT 0 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone
);


--
-- Name: gm_ai_video_models_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE IF NOT EXISTS public.gm_ai_video_models_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: gm_ai_video_models_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.gm_ai_video_models_id_seq OWNED BY public.gm_ai_video_models.id;


--
-- Name: gm_campaign_templates; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE IF NOT EXISTS public.gm_campaign_templates (
    id integer NOT NULL,
    campaign_id integer NOT NULL,
    weight integer NOT NULL,
    reply_prompt text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone,
    dm_prompt text,
    reply_post_prompt text
);


--
-- Name: gm_campaign_templates_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE IF NOT EXISTS public.gm_campaign_templates_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: gm_campaign_templates_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.gm_campaign_templates_id_seq OWNED BY public.gm_campaign_templates.id;


--
-- Name: gm_email_verifications; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE IF NOT EXISTS public.gm_email_verifications (
    id integer NOT NULL,
    email character varying(255) NOT NULL,
    code character varying(6) NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    verified boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    ip_address character varying(45),
    user_agent text
);


--
-- Name: gm_email_verifications_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE IF NOT EXISTS public.gm_email_verifications_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: gm_email_verifications_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.gm_email_verifications_id_seq OWNED BY public.gm_email_verifications.id;


--
-- Name: gm_login_logs; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE IF NOT EXISTS public.gm_login_logs (
    id integer NOT NULL,
    user_id integer NOT NULL,
    ip_address text,
    user_agent text,
    login_status character varying(20) DEFAULT 'SUCCESS'::character varying NOT NULL,
    failure_reason text,
    login_at timestamp with time zone DEFAULT now() NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: gm_login_logs_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE IF NOT EXISTS public.gm_login_logs_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: gm_login_logs_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.gm_login_logs_id_seq OWNED BY public.gm_login_logs.id;


--
-- Name: gm_platforms; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE IF NOT EXISTS public.gm_platforms (
    id integer NOT NULL,
    name character varying NOT NULL,
    display_name character varying NOT NULL,
    is_active boolean DEFAULT true NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone,
    base_url character varying DEFAULT ''::character varying NOT NULL,
    page_size integer DEFAULT 20 NOT NULL
);


--
-- Name: gm_pricing_rules; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE IF NOT EXISTS public.gm_pricing_rules (
    id integer NOT NULL,
    action_type character varying NOT NULL,
    platform_id integer,
    cost_points numeric(10,2) NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone
);


--
-- Name: gm_promo_codes; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE IF NOT EXISTS public.gm_promo_codes (
    id integer NOT NULL,
    code character varying(36) NOT NULL,
    points integer NOT NULL,
    is_active boolean DEFAULT true NOT NULL,
    expires_at timestamp with time zone,
    used_by_user_id integer,
    used_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone
);


--
-- Name: gm_promo_codes_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE IF NOT EXISTS public.gm_promo_codes_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: gm_promo_codes_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.gm_promo_codes_id_seq OWNED BY public.gm_promo_codes.id;


--
-- Name: gm_referral_earnings; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE IF NOT EXISTS public.gm_referral_earnings (
    id integer NOT NULL,
    referral_id integer NOT NULL,
    transaction_id integer,
    amount numeric(10,2) NOT NULL,
    description text,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: gm_referral_earnings_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE IF NOT EXISTS public.gm_referral_earnings_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: gm_referral_earnings_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.gm_referral_earnings_id_seq OWNED BY public.gm_referral_earnings.id;


--
-- Name: gm_referrals; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE IF NOT EXISTS public.gm_referrals (
    id integer NOT NULL,
    referrer_id integer NOT NULL,
    referee_id integer NOT NULL,
    commission_rate numeric(5,2) DEFAULT 10.00 NOT NULL,
    total_earned numeric(10,2) DEFAULT 0.00 NOT NULL,
    status character varying(20) DEFAULT 'ACTIVE'::character varying NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone
);


--
-- Name: gm_referrals_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE IF NOT EXISTS public.gm_referrals_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: gm_referrals_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.gm_referrals_id_seq OWNED BY public.gm_referrals.id;


--
-- Name: gm_regions; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE IF NOT EXISTS public.gm_regions (
    id integer NOT NULL,
    platform_id integer NOT NULL,
    code character varying NOT NULL,
    display_name character varying NOT NULL,
    is_active boolean DEFAULT true NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone,
    name character varying DEFAULT ''::character varying NOT NULL
);


--
-- Name: gm_social_accounts; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE IF NOT EXISTS public.gm_social_accounts (
    id integer NOT NULL,
    user_id integer NOT NULL,
    platform_id integer NOT NULL,
    username character varying NOT NULL,
    proxy_url character varying,
    status character varying DEFAULT 'ACTIVE'::character varying NOT NULL,
    health_score integer DEFAULT 100,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone,
    group_id integer,
    cookie text DEFAULT ''::text NOT NULL,
    daily_max_replies integer DEFAULT 50 NOT NULL,
    device_id character varying(255),
    profile_name character varying(255),
    CONSTRAINT social_accounts_health_score_check CHECK (((health_score >= 0) AND (health_score <= 100)))
);


--
-- Name: gm_social_groups; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE IF NOT EXISTS public.gm_social_groups (
    id integer NOT NULL,
    user_id integer NOT NULL,
    platform_id integer NOT NULL,
    group_name character varying NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone
);


--
-- Name: gm_upload_tasks; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE IF NOT EXISTS public.gm_upload_tasks (
    id integer NOT NULL,
    user_id integer NOT NULL,
    social_account_id integer NOT NULL,
    task_type character varying(50) DEFAULT 'upload'::character varying NOT NULL,
    metadata jsonb NOT NULL,
    status character varying(20) DEFAULT 'init'::character varying NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone,
    platform_id integer,
    CONSTRAINT valid_status CHECK (((status)::text = ANY ((ARRAY['init'::character varying, 'processing'::character varying, 'done'::character varying, 'failed'::character varying])::text[])))
);


--
-- Name: gm_upload_tasks_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE IF NOT EXISTS public.gm_upload_tasks_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: gm_upload_tasks_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.gm_upload_tasks_id_seq OWNED BY public.gm_upload_tasks.id;


--
-- Name: gm_user_wallets; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE IF NOT EXISTS public.gm_user_wallets (
    user_id integer NOT NULL,
    balance_points numeric(10,2) DEFAULT 0.00 NOT NULL,
    frozen_points numeric(10,2) DEFAULT 0.00 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone,
    deposit_cny numeric DEFAULT 0 NOT NULL,
    deposit_usd numeric DEFAULT 0 NOT NULL,
    CONSTRAINT user_wallets_balance_points_check CHECK ((balance_points >= (0)::numeric)),
    CONSTRAINT user_wallets_frozen_points_check CHECK ((frozen_points >= (0)::numeric))
);


--
-- Name: gm_users; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE IF NOT EXISTS public.gm_users (
    id integer NOT NULL,
    email character varying(255),
    password_hash character varying(255) NOT NULL,
    invitation_code character varying(50),
    referred_by character varying(50),
    company_name character varying(255),
    api_key character varying(255),
    status character varying(50) DEFAULT 'PENDING_VERIFICATION'::character varying NOT NULL,
    created_at timestamp with time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at timestamp with time zone,
    full_name character varying DEFAULT ''::character varying NOT NULL,
    role character varying DEFAULT 'user'::character varying NOT NULL,
    is_active boolean DEFAULT true NOT NULL,
    invite_code character varying(36),
    invited_by character varying(36),
    username character varying(50)
);


--
-- Name: gm_video_generation_tasks; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE IF NOT EXISTS public.gm_video_generation_tasks (
    id integer NOT NULL,
    user_id integer NOT NULL,
    task_id character varying(255) NOT NULL,
    generation_id character varying(255),
    prompt text,
    media_id character varying(255),
    status character varying(50) DEFAULT 'pending'::character varying NOT NULL,
    progress_pct numeric(3,2),
    video_width integer,
    video_height integer,
    video_url text,
    thumbnail_url text,
    provider_post_id character varying(255),
    provider_response jsonb,
    cost_points numeric(10,2) DEFAULT 200.00 NOT NULL,
    wallet_transaction_id integer,
    error_message text,
    retry_count integer DEFAULT 0 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone,
    completed_at timestamp with time zone,
    model_id integer,
    title character varying(255),
    orientation character varying(20) DEFAULT 'portrait'::character varying,
    video_seconds character varying(10),
    video_size character varying(20),
    CONSTRAINT valid_status CHECK (((status)::text = ANY ((ARRAY['pending'::character varying, 'queued'::character varying, 'processing'::character varying, 'succeeded'::character varying, 'failed'::character varying, 'cancelled'::character varying])::text[])))
);


--
-- Name: gm_video_generation_tasks_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE IF NOT EXISTS public.gm_video_generation_tasks_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: gm_video_generation_tasks_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.gm_video_generation_tasks_id_seq OWNED BY public.gm_video_generation_tasks.id;


--
-- Name: gm_wallet_transactions; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE IF NOT EXISTS public.gm_wallet_transactions (
    id integer NOT NULL,
    user_id integer NOT NULL,
    amount numeric(10,2) NOT NULL,
    type character varying NOT NULL,
    payment_method character varying,
    external_txn_id character varying,
    reference_id integer,
    description text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone
);


--
-- Name: platforms_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE IF NOT EXISTS public.platforms_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: platforms_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.platforms_id_seq OWNED BY public.gm_platforms.id;


--
-- Name: pricing_rules_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE IF NOT EXISTS public.pricing_rules_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: pricing_rules_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.pricing_rules_id_seq OWNED BY public.gm_pricing_rules.id;


--
-- Name: regions_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE IF NOT EXISTS public.regions_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: regions_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.regions_id_seq OWNED BY public.gm_regions.id;


--
-- Name: social_accounts_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE IF NOT EXISTS public.social_accounts_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: social_accounts_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.social_accounts_id_seq OWNED BY public.gm_social_accounts.id;


--
-- Name: social_groups_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE IF NOT EXISTS public.social_groups_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: social_groups_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.social_groups_id_seq OWNED BY public.gm_social_groups.id;


--
-- Name: users_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE IF NOT EXISTS public.users_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: users_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.users_id_seq OWNED BY public.gm_users.id;


--
-- Name: wallet_transactions_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE IF NOT EXISTS public.wallet_transactions_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: wallet_transactions_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.wallet_transactions_id_seq OWNED BY public.gm_wallet_transactions.id;


--
-- Name: gm_admin_users id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_admin_users ALTER COLUMN id SET DEFAULT nextval('public.gm_admin_users_id_seq'::regclass);


--
-- Name: gm_agent_comments id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_comments ALTER COLUMN id SET DEFAULT nextval('public.agent_comments_id_seq'::regclass);


--
-- Name: gm_agent_instagram_comments id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_instagram_comments ALTER COLUMN id SET DEFAULT nextval('public.gm_agent_instagram_comments_id_seq'::regclass);


--
-- Name: gm_agent_instagram_posts id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_instagram_posts ALTER COLUMN id SET DEFAULT nextval('public.gm_agent_instagram_posts_id_seq'::regclass);


--
-- Name: gm_agent_reddit_comments id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_reddit_comments ALTER COLUMN id SET DEFAULT nextval('public.gm_agent_reddit_comments_id_seq'::regclass);


--
-- Name: gm_agent_reddit_posts id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_reddit_posts ALTER COLUMN id SET DEFAULT nextval('public.gm_agent_reddit_posts_id_seq'::regclass);


--
-- Name: gm_agent_twitter_comments id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_twitter_comments ALTER COLUMN id SET DEFAULT nextval('public.gm_agent_twitter_comments_id_seq'::regclass);


--
-- Name: gm_agent_twitter_tweets id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_twitter_tweets ALTER COLUMN id SET DEFAULT nextval('public.gm_agent_twitter_tweets_id_seq'::regclass);


--
-- Name: gm_agent_videos id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_videos ALTER COLUMN id SET DEFAULT nextval('public.agent_videos_id_seq'::regclass);


--
-- Name: gm_ai_models id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_ai_models ALTER COLUMN id SET DEFAULT nextval('public.ai_models_id_seq'::regclass);


--
-- Name: gm_ai_video_models id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_ai_video_models ALTER COLUMN id SET DEFAULT nextval('public.gm_ai_video_models_id_seq'::regclass);


--
-- Name: gm_campaign_accounts id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_campaign_accounts ALTER COLUMN id SET DEFAULT nextval('public.campaign_accounts_id_seq'::regclass);


--
-- Name: gm_campaign_templates id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_campaign_templates ALTER COLUMN id SET DEFAULT nextval('public.gm_campaign_templates_id_seq'::regclass);


--
-- Name: gm_campaigns id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_campaigns ALTER COLUMN id SET DEFAULT nextval('public.campaigns_id_seq'::regclass);


--
-- Name: gm_crawler_results id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_crawler_results ALTER COLUMN id SET DEFAULT nextval('public.crawler_results_id_seq'::regclass);


--
-- Name: gm_crawler_tasks id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_crawler_tasks ALTER COLUMN id SET DEFAULT nextval('public.crawler_tasks_id_seq'::regclass);


--
-- Name: gm_email_verifications id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_email_verifications ALTER COLUMN id SET DEFAULT nextval('public.gm_email_verifications_id_seq'::regclass);


--
-- Name: gm_login_logs id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_login_logs ALTER COLUMN id SET DEFAULT nextval('public.gm_login_logs_id_seq'::regclass);


--
-- Name: gm_platforms id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_platforms ALTER COLUMN id SET DEFAULT nextval('public.platforms_id_seq'::regclass);


--
-- Name: gm_pricing_rules id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_pricing_rules ALTER COLUMN id SET DEFAULT nextval('public.pricing_rules_id_seq'::regclass);


--
-- Name: gm_promo_codes id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_promo_codes ALTER COLUMN id SET DEFAULT nextval('public.gm_promo_codes_id_seq'::regclass);


--
-- Name: gm_referral_earnings id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_referral_earnings ALTER COLUMN id SET DEFAULT nextval('public.gm_referral_earnings_id_seq'::regclass);


--
-- Name: gm_referrals id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_referrals ALTER COLUMN id SET DEFAULT nextval('public.gm_referrals_id_seq'::regclass);


--
-- Name: gm_regions id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_regions ALTER COLUMN id SET DEFAULT nextval('public.regions_id_seq'::regclass);


--
-- Name: gm_social_accounts id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_social_accounts ALTER COLUMN id SET DEFAULT nextval('public.social_accounts_id_seq'::regclass);


--
-- Name: gm_social_groups id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_social_groups ALTER COLUMN id SET DEFAULT nextval('public.social_groups_id_seq'::regclass);


--
-- Name: gm_upload_tasks id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_upload_tasks ALTER COLUMN id SET DEFAULT nextval('public.gm_upload_tasks_id_seq'::regclass);


--
-- Name: gm_users id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_users ALTER COLUMN id SET DEFAULT nextval('public.users_id_seq'::regclass);


--
-- Name: gm_video_generation_tasks id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_video_generation_tasks ALTER COLUMN id SET DEFAULT nextval('public.gm_video_generation_tasks_id_seq'::regclass);


--
-- Name: gm_wallet_transactions id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_wallet_transactions ALTER COLUMN id SET DEFAULT nextval('public.wallet_transactions_id_seq'::regclass);


--
-- Name: gm_agent_comments agent_comments_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_comments
    ADD CONSTRAINT agent_comments_pkey PRIMARY KEY (id);


--
-- Name: gm_agent_videos agent_videos_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_videos
    ADD CONSTRAINT agent_videos_pkey PRIMARY KEY (id);


--
-- Name: gm_ai_models ai_models_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_ai_models
    ADD CONSTRAINT ai_models_pkey PRIMARY KEY (id);


--
-- Name: gm_campaign_accounts campaign_accounts_campaign_id_account_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_campaign_accounts
    ADD CONSTRAINT campaign_accounts_campaign_id_account_id_key UNIQUE (campaign_id, account_id);


--
-- Name: gm_campaign_accounts campaign_accounts_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_campaign_accounts
    ADD CONSTRAINT campaign_accounts_pkey PRIMARY KEY (id);


--
-- Name: gm_campaigns campaigns_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_campaigns
    ADD CONSTRAINT campaigns_pkey PRIMARY KEY (id);


--
-- Name: gm_crawler_results crawler_results_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_crawler_results
    ADD CONSTRAINT crawler_results_pkey PRIMARY KEY (id);


--
-- Name: gm_crawler_tasks crawler_tasks_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_crawler_tasks
    ADD CONSTRAINT crawler_tasks_pkey PRIMARY KEY (id);


--
-- Name: gm_admin_users gm_admin_users_email_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_admin_users
    ADD CONSTRAINT gm_admin_users_email_key UNIQUE (email);


--
-- Name: gm_admin_users gm_admin_users_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_admin_users
    ADD CONSTRAINT gm_admin_users_pkey PRIMARY KEY (id);


--
-- Name: gm_admin_users gm_admin_users_username_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_admin_users
    ADD CONSTRAINT gm_admin_users_username_key UNIQUE (username);


--
-- Name: gm_agent_instagram_comments gm_agent_instagram_comments_instagram_comment_id_post_db_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_instagram_comments
    ADD CONSTRAINT gm_agent_instagram_comments_instagram_comment_id_post_db_id_key UNIQUE (instagram_comment_id, post_db_id);


--
-- Name: gm_agent_instagram_comments gm_agent_instagram_comments_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_instagram_comments
    ADD CONSTRAINT gm_agent_instagram_comments_pkey PRIMARY KEY (id);


--
-- Name: gm_agent_instagram_posts gm_agent_instagram_posts_code_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_instagram_posts
    ADD CONSTRAINT gm_agent_instagram_posts_code_key UNIQUE (code);


--
-- Name: gm_agent_instagram_posts gm_agent_instagram_posts_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_instagram_posts
    ADD CONSTRAINT gm_agent_instagram_posts_pkey PRIMARY KEY (id);


--
-- Name: gm_agent_reddit_comments gm_agent_reddit_comments_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_reddit_comments
    ADD CONSTRAINT gm_agent_reddit_comments_pkey PRIMARY KEY (id);


--
-- Name: gm_agent_reddit_posts gm_agent_reddit_posts_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_reddit_posts
    ADD CONSTRAINT gm_agent_reddit_posts_pkey PRIMARY KEY (id);


--
-- Name: gm_agent_reddit_posts gm_agent_reddit_posts_task_id_post_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_reddit_posts
    ADD CONSTRAINT gm_agent_reddit_posts_task_id_post_id_key UNIQUE (task_id, post_id);


--
-- Name: gm_agent_twitter_comments gm_agent_twitter_comments_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_twitter_comments
    ADD CONSTRAINT gm_agent_twitter_comments_pkey PRIMARY KEY (id);


--
-- Name: gm_agent_twitter_comments gm_agent_twitter_comments_twitter_comment_id_tweet_db_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_twitter_comments
    ADD CONSTRAINT gm_agent_twitter_comments_twitter_comment_id_tweet_db_id_key UNIQUE (twitter_comment_id, tweet_db_id);


--
-- Name: gm_agent_twitter_tweets gm_agent_twitter_tweets_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_twitter_tweets
    ADD CONSTRAINT gm_agent_twitter_tweets_pkey PRIMARY KEY (id);


--
-- Name: gm_agent_twitter_tweets gm_agent_twitter_tweets_twitter_tweet_id_task_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_twitter_tweets
    ADD CONSTRAINT gm_agent_twitter_tweets_twitter_tweet_id_task_id_key UNIQUE (twitter_tweet_id, task_id);


--
-- Name: gm_agent_videos gm_agent_videos_task_id_video_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_videos
    ADD CONSTRAINT gm_agent_videos_task_id_video_id_key UNIQUE (task_id, video_id);


--
-- Name: gm_ai_video_models gm_ai_video_models_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_ai_video_models
    ADD CONSTRAINT gm_ai_video_models_pkey PRIMARY KEY (id);


--
-- Name: gm_campaign_templates gm_campaign_templates_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_campaign_templates
    ADD CONSTRAINT gm_campaign_templates_pkey PRIMARY KEY (id);


--
-- Name: gm_email_verifications gm_email_verifications_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_email_verifications
    ADD CONSTRAINT gm_email_verifications_pkey PRIMARY KEY (id);


--
-- Name: gm_login_logs gm_login_logs_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_login_logs
    ADD CONSTRAINT gm_login_logs_pkey PRIMARY KEY (id);


--
-- Name: gm_promo_codes gm_promo_codes_code_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_promo_codes
    ADD CONSTRAINT gm_promo_codes_code_key UNIQUE (code);


--
-- Name: gm_promo_codes gm_promo_codes_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_promo_codes
    ADD CONSTRAINT gm_promo_codes_pkey PRIMARY KEY (id);


--
-- Name: gm_referral_earnings gm_referral_earnings_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_referral_earnings
    ADD CONSTRAINT gm_referral_earnings_pkey PRIMARY KEY (id);


--
-- Name: gm_referrals gm_referrals_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_referrals
    ADD CONSTRAINT gm_referrals_pkey PRIMARY KEY (id);


--
-- Name: gm_upload_tasks gm_upload_tasks_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_upload_tasks
    ADD CONSTRAINT gm_upload_tasks_pkey PRIMARY KEY (id);


--
-- Name: gm_users gm_users_invite_code_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_users
    ADD CONSTRAINT gm_users_invite_code_key UNIQUE (invite_code);


--
-- Name: gm_users gm_users_username_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_users
    ADD CONSTRAINT gm_users_username_key UNIQUE (username);


--
-- Name: gm_video_generation_tasks gm_video_generation_tasks_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_video_generation_tasks
    ADD CONSTRAINT gm_video_generation_tasks_pkey PRIMARY KEY (id);


--
-- Name: gm_video_generation_tasks gm_video_generation_tasks_task_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_video_generation_tasks
    ADD CONSTRAINT gm_video_generation_tasks_task_id_key UNIQUE (task_id);


--
-- Name: gm_platforms platforms_name_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_platforms
    ADD CONSTRAINT platforms_name_key UNIQUE (name);


--
-- Name: gm_platforms platforms_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_platforms
    ADD CONSTRAINT platforms_pkey PRIMARY KEY (id);


--
-- Name: gm_pricing_rules pricing_rules_action_type_platform_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_pricing_rules
    ADD CONSTRAINT pricing_rules_action_type_platform_id_key UNIQUE (action_type, platform_id);


--
-- Name: gm_pricing_rules pricing_rules_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_pricing_rules
    ADD CONSTRAINT pricing_rules_pkey PRIMARY KEY (id);


--
-- Name: gm_regions regions_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_regions
    ADD CONSTRAINT regions_pkey PRIMARY KEY (id);


--
-- Name: gm_regions regions_platform_id_code_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_regions
    ADD CONSTRAINT regions_platform_id_code_key UNIQUE (platform_id, code);


--
-- Name: gm_social_accounts social_accounts_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_social_accounts
    ADD CONSTRAINT social_accounts_pkey PRIMARY KEY (id);


--
-- Name: gm_social_groups social_groups_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_social_groups
    ADD CONSTRAINT social_groups_pkey PRIMARY KEY (id);


--
-- Name: gm_referrals uq_referee; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_referrals
    ADD CONSTRAINT uq_referee UNIQUE (referee_id);


--
-- Name: gm_user_wallets user_wallets_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_user_wallets
    ADD CONSTRAINT user_wallets_pkey PRIMARY KEY (user_id);


--
-- Name: gm_users users_email_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_users
    ADD CONSTRAINT users_email_key UNIQUE (email);


--
-- Name: gm_users users_invitation_code_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_users
    ADD CONSTRAINT users_invitation_code_key UNIQUE (invitation_code);


--
-- Name: gm_users users_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_users
    ADD CONSTRAINT users_pkey PRIMARY KEY (id);


--
-- Name: gm_wallet_transactions wallet_transactions_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_wallet_transactions
    ADD CONSTRAINT wallet_transactions_pkey PRIMARY KEY (id);


--
-- Name: idx_admin_users_email; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_admin_users_email ON public.gm_admin_users USING btree (email);


--
-- Name: idx_admin_users_username; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_admin_users_username ON public.gm_admin_users USING btree (username);


--
-- Name: idx_agent_comments_status; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_agent_comments_status ON public.gm_agent_comments USING btree (status);


--
-- Name: idx_agent_videos_task_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_agent_videos_task_id ON public.gm_agent_videos USING btree (task_id);


--
-- Name: idx_ai_models_model_type; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_ai_models_model_type ON public.gm_ai_models USING btree (model_type);


--
-- Name: idx_campaign_accounts_account_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_campaign_accounts_account_id ON public.gm_campaign_accounts USING btree (account_id);


--
-- Name: idx_campaign_accounts_campaign_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_campaign_accounts_campaign_id ON public.gm_campaign_accounts USING btree (campaign_id);


--
-- Name: idx_campaigns_ai_model_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_campaigns_ai_model_id ON public.gm_campaigns USING btree (ai_model_id);


--
-- Name: idx_campaigns_consumption; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_campaigns_consumption ON public.gm_campaigns USING btree (pending_consumption, actual_consumption);


--
-- Name: idx_campaigns_platform_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_campaigns_platform_id ON public.gm_campaigns USING btree (platform_id);


--
-- Name: idx_campaigns_region_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_campaigns_region_id ON public.gm_campaigns USING btree (region_id);


--
-- Name: idx_campaigns_status; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_campaigns_status ON public.gm_campaigns USING btree (status);


--
-- Name: idx_campaigns_total_scanned; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_campaigns_total_scanned ON public.gm_campaigns USING btree (total_scanned);


--
-- Name: idx_crawler_results_task_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_crawler_results_task_id ON public.gm_crawler_results USING btree (task_id);


--
-- Name: idx_crawler_results_video_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_crawler_results_video_id ON public.gm_crawler_results USING btree (video_id);


--
-- Name: idx_crawler_tasks_campaign_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_crawler_tasks_campaign_id ON public.gm_crawler_tasks USING btree (campaign_id);


--
-- Name: idx_crawler_tasks_status; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_crawler_tasks_status ON public.gm_crawler_tasks USING btree (status);


--
-- Name: idx_email_verifications_code; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_email_verifications_code ON public.gm_email_verifications USING btree (code);


--
-- Name: idx_email_verifications_email; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_email_verifications_email ON public.gm_email_verifications USING btree (email);


--
-- Name: idx_email_verifications_expires_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_email_verifications_expires_at ON public.gm_email_verifications USING btree (expires_at);


--
-- Name: idx_instagram_comments_campaign_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_instagram_comments_campaign_id ON public.gm_agent_instagram_comments USING btree (campaign_id);


--
-- Name: idx_instagram_comments_comment_username; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_instagram_comments_comment_username ON public.gm_agent_instagram_comments USING btree (comment_username);


--
-- Name: idx_instagram_comments_instagram_comment_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_instagram_comments_instagram_comment_id ON public.gm_agent_instagram_comments USING btree (instagram_comment_id);


--
-- Name: idx_instagram_comments_parent_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_instagram_comments_parent_id ON public.gm_agent_instagram_comments USING btree (parent_comment_id);


--
-- Name: idx_instagram_comments_post_db_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_instagram_comments_post_db_id ON public.gm_agent_instagram_comments USING btree (post_db_id);


--
-- Name: idx_instagram_comments_status; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_instagram_comments_status ON public.gm_agent_instagram_comments USING btree (status);


--
-- Name: idx_instagram_posts_campaign_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_instagram_posts_campaign_id ON public.gm_agent_instagram_posts USING btree (campaign_id);


--
-- Name: idx_instagram_posts_code; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_instagram_posts_code ON public.gm_agent_instagram_posts USING btree (code);


--
-- Name: idx_instagram_posts_media_type; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_instagram_posts_media_type ON public.gm_agent_instagram_posts USING btree (media_type);


--
-- Name: idx_instagram_posts_owner_username; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_instagram_posts_owner_username ON public.gm_agent_instagram_posts USING btree (owner_username);


--
-- Name: idx_instagram_posts_posted_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_instagram_posts_posted_at ON public.gm_agent_instagram_posts USING btree (posted_at);


--
-- Name: idx_instagram_posts_task_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_instagram_posts_task_id ON public.gm_agent_instagram_posts USING btree (task_id);


--
-- Name: idx_login_logs_ip_address; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_login_logs_ip_address ON public.gm_login_logs USING btree (ip_address);


--
-- Name: idx_login_logs_login_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_login_logs_login_at ON public.gm_login_logs USING btree (login_at);


--
-- Name: idx_login_logs_user_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_login_logs_user_id ON public.gm_login_logs USING btree (user_id);


--
-- Name: idx_pricing_rules_action_platform; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_pricing_rules_action_platform ON public.gm_pricing_rules USING btree (action_type, platform_id);


--
-- Name: idx_promo_codes_code; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_promo_codes_code ON public.gm_promo_codes USING btree (code);


--
-- Name: idx_promo_codes_expires_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_promo_codes_expires_at ON public.gm_promo_codes USING btree (expires_at);


--
-- Name: idx_promo_codes_is_active; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_promo_codes_is_active ON public.gm_promo_codes USING btree (is_active);


--
-- Name: idx_reddit_comments_campaign_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_reddit_comments_campaign_id ON public.gm_agent_reddit_comments USING btree (campaign_id);


--
-- Name: idx_reddit_comments_comment_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_reddit_comments_comment_id ON public.gm_agent_reddit_comments USING btree (comment_id);


--
-- Name: idx_reddit_comments_created_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_reddit_comments_created_at ON public.gm_agent_reddit_comments USING btree (created_at DESC);


--
-- Name: idx_reddit_comments_post_db_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_reddit_comments_post_db_id ON public.gm_agent_reddit_comments USING btree (post_db_id);


--
-- Name: idx_reddit_comments_status; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_reddit_comments_status ON public.gm_agent_reddit_comments USING btree (status);


--
-- Name: idx_reddit_posts_campaign_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_reddit_posts_campaign_id ON public.gm_agent_reddit_posts USING btree (campaign_id);


--
-- Name: idx_reddit_posts_created_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_reddit_posts_created_at ON public.gm_agent_reddit_posts USING btree (created_at DESC);


--
-- Name: idx_reddit_posts_post_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_reddit_posts_post_id ON public.gm_agent_reddit_posts USING btree (post_id);


--
-- Name: idx_reddit_posts_subreddit; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_reddit_posts_subreddit ON public.gm_agent_reddit_posts USING btree (subreddit);


--
-- Name: idx_reddit_posts_task_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_reddit_posts_task_id ON public.gm_agent_reddit_posts USING btree (task_id);


--
-- Name: idx_referral_earnings_referral; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_referral_earnings_referral ON public.gm_referral_earnings USING btree (referral_id);


--
-- Name: idx_referral_earnings_transaction; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_referral_earnings_transaction ON public.gm_referral_earnings USING btree (transaction_id);


--
-- Name: idx_referrals_referrer; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_referrals_referrer ON public.gm_referrals USING btree (referrer_id);


--
-- Name: idx_referrals_status; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_referrals_status ON public.gm_referrals USING btree (status);


--
-- Name: idx_regions_platform_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_regions_platform_id ON public.gm_regions USING btree (platform_id);


--
-- Name: idx_social_accounts_platform_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_social_accounts_platform_id ON public.gm_social_accounts USING btree (platform_id);


--
-- Name: idx_social_accounts_user_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_social_accounts_user_id ON public.gm_social_accounts USING btree (user_id);


--
-- Name: idx_social_groups_user_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_social_groups_user_id ON public.gm_social_groups USING btree (user_id);


--
-- Name: idx_twitter_comments_campaign_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_twitter_comments_campaign_id ON public.gm_agent_twitter_comments USING btree (campaign_id);


--
-- Name: idx_twitter_comments_screen_name; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_twitter_comments_screen_name ON public.gm_agent_twitter_comments USING btree (comment_screen_name);


--
-- Name: idx_twitter_comments_status; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_twitter_comments_status ON public.gm_agent_twitter_comments USING btree (status);


--
-- Name: idx_twitter_comments_tweet_db_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_twitter_comments_tweet_db_id ON public.gm_agent_twitter_comments USING btree (tweet_db_id);


--
-- Name: idx_twitter_comments_twitter_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_twitter_comments_twitter_id ON public.gm_agent_twitter_comments USING btree (twitter_comment_id);


--
-- Name: idx_twitter_tweets_campaign_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_twitter_tweets_campaign_id ON public.gm_agent_twitter_tweets USING btree (campaign_id);


--
-- Name: idx_twitter_tweets_conversation_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_twitter_tweets_conversation_id ON public.gm_agent_twitter_tweets USING btree (conversation_id);


--
-- Name: idx_twitter_tweets_screen_name; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_twitter_tweets_screen_name ON public.gm_agent_twitter_tweets USING btree (screen_name);


--
-- Name: idx_twitter_tweets_task_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_twitter_tweets_task_id ON public.gm_agent_twitter_tweets USING btree (task_id);


--
-- Name: idx_twitter_tweets_twitter_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_twitter_tweets_twitter_id ON public.gm_agent_twitter_tweets USING btree (twitter_tweet_id);


--
-- Name: idx_upload_tasks_created_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_upload_tasks_created_at ON public.gm_upload_tasks USING btree (created_at DESC);


--
-- Name: idx_upload_tasks_platform_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_upload_tasks_platform_id ON public.gm_upload_tasks USING btree (platform_id);


--
-- Name: idx_upload_tasks_social_account_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_upload_tasks_social_account_id ON public.gm_upload_tasks USING btree (social_account_id);


--
-- Name: idx_upload_tasks_status; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_upload_tasks_status ON public.gm_upload_tasks USING btree (status);


--
-- Name: idx_upload_tasks_status_created; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_upload_tasks_status_created ON public.gm_upload_tasks USING btree (status, created_at DESC);


--
-- Name: idx_upload_tasks_user_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_upload_tasks_user_id ON public.gm_upload_tasks USING btree (user_id);


--
-- Name: idx_users_email; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_users_email ON public.gm_users USING btree (email);


--
-- Name: idx_users_invitation_code; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_users_invitation_code ON public.gm_users USING btree (invitation_code);


--
-- Name: idx_users_invite_code; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_users_invite_code ON public.gm_users USING btree (invite_code);


--
-- Name: idx_users_invited_by; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_users_invited_by ON public.gm_users USING btree (invited_by);


--
-- Name: idx_users_username_unique; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX IF NOT EXISTS idx_users_username_unique ON public.gm_users USING btree (lower((username)::text));


--
-- Name: idx_video_tasks_created_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_video_tasks_created_at ON public.gm_video_generation_tasks USING btree (created_at);


--
-- Name: idx_video_tasks_orientation; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_video_tasks_orientation ON public.gm_video_generation_tasks USING btree (orientation);


--
-- Name: idx_video_tasks_provider_post_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_video_tasks_provider_post_id ON public.gm_video_generation_tasks USING btree (provider_post_id);


--
-- Name: idx_video_tasks_status; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_video_tasks_status ON public.gm_video_generation_tasks USING btree (status);


--
-- Name: idx_video_tasks_status_updated; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_video_tasks_status_updated ON public.gm_video_generation_tasks USING btree (status, updated_at) WHERE ((status)::text = ANY ((ARRAY['pending'::character varying, 'queued'::character varying, 'processing'::character varying])::text[]));


--
-- Name: idx_video_tasks_task_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_video_tasks_task_id ON public.gm_video_generation_tasks USING btree (task_id);


--
-- Name: idx_video_tasks_title; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_video_tasks_title ON public.gm_video_generation_tasks USING btree (title);


--
-- Name: idx_video_tasks_user_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_video_tasks_user_id ON public.gm_video_generation_tasks USING btree (user_id);


--
-- Name: idx_wallet_transactions_created_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_wallet_transactions_created_at ON public.gm_wallet_transactions USING btree (created_at DESC);


--
-- Name: idx_wallet_transactions_type; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_wallet_transactions_type ON public.gm_wallet_transactions USING btree (type);


--
-- Name: idx_wallet_transactions_user_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX IF NOT EXISTS idx_wallet_transactions_user_id ON public.gm_wallet_transactions USING btree (user_id);


--
-- Name: gm_referrals referrals_updated_at_trigger; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER referrals_updated_at_trigger BEFORE UPDATE ON public.gm_referrals FOR EACH ROW EXECUTE FUNCTION public.update_referrals_updated_at();


--
-- Name: gm_ai_models set_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER set_updated_at BEFORE UPDATE ON public.gm_ai_models FOR EACH ROW EXECUTE FUNCTION public.diesel_set_updated_at();


--
-- Name: gm_campaign_accounts set_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER set_updated_at BEFORE UPDATE ON public.gm_campaign_accounts FOR EACH ROW EXECUTE FUNCTION public.diesel_set_updated_at();


--
-- Name: gm_campaigns set_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER set_updated_at BEFORE UPDATE ON public.gm_campaigns FOR EACH ROW EXECUTE FUNCTION public.diesel_set_updated_at();


--
-- Name: gm_crawler_results set_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER set_updated_at BEFORE UPDATE ON public.gm_crawler_results FOR EACH ROW EXECUTE FUNCTION public.diesel_set_updated_at();


--
-- Name: gm_crawler_tasks set_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER set_updated_at BEFORE UPDATE ON public.gm_crawler_tasks FOR EACH ROW EXECUTE FUNCTION public.diesel_set_updated_at();


--
-- Name: gm_platforms set_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER set_updated_at BEFORE UPDATE ON public.gm_platforms FOR EACH ROW EXECUTE FUNCTION public.diesel_set_updated_at();


--
-- Name: gm_pricing_rules set_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER set_updated_at BEFORE UPDATE ON public.gm_pricing_rules FOR EACH ROW EXECUTE FUNCTION public.diesel_set_updated_at();


--
-- Name: gm_regions set_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER set_updated_at BEFORE UPDATE ON public.gm_regions FOR EACH ROW EXECUTE FUNCTION public.diesel_set_updated_at();


--
-- Name: gm_social_accounts set_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER set_updated_at BEFORE UPDATE ON public.gm_social_accounts FOR EACH ROW EXECUTE FUNCTION public.diesel_set_updated_at();


--
-- Name: gm_social_groups set_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER set_updated_at BEFORE UPDATE ON public.gm_social_groups FOR EACH ROW EXECUTE FUNCTION public.diesel_set_updated_at();


--
-- Name: gm_user_wallets set_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER set_updated_at BEFORE UPDATE ON public.gm_user_wallets FOR EACH ROW EXECUTE FUNCTION public.diesel_set_updated_at();


--
-- Name: gm_users set_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER set_updated_at BEFORE UPDATE ON public.gm_users FOR EACH ROW EXECUTE FUNCTION public.diesel_set_updated_at();


--
-- Name: gm_wallet_transactions set_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER set_updated_at BEFORE UPDATE ON public.gm_wallet_transactions FOR EACH ROW EXECUTE FUNCTION public.diesel_set_updated_at();


--
-- Name: gm_agent_reddit_comments set_updated_at_reddit_comments; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER set_updated_at_reddit_comments BEFORE UPDATE ON public.gm_agent_reddit_comments FOR EACH ROW EXECUTE FUNCTION public.diesel_set_updated_at();


--
-- Name: gm_agent_reddit_posts set_updated_at_reddit_posts; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER set_updated_at_reddit_posts BEFORE UPDATE ON public.gm_agent_reddit_posts FOR EACH ROW EXECUTE FUNCTION public.diesel_set_updated_at();


--
-- Name: gm_agent_instagram_comments trigger_update_instagram_comments_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER trigger_update_instagram_comments_updated_at BEFORE UPDATE ON public.gm_agent_instagram_comments FOR EACH ROW EXECUTE FUNCTION public.update_instagram_comments_updated_at();


--
-- Name: gm_agent_instagram_posts trigger_update_instagram_posts_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER trigger_update_instagram_posts_updated_at BEFORE UPDATE ON public.gm_agent_instagram_posts FOR EACH ROW EXECUTE FUNCTION public.update_instagram_posts_updated_at();


--
-- Name: gm_agent_comments agent_comments_video_db_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_comments
    ADD CONSTRAINT agent_comments_video_db_id_fkey FOREIGN KEY (video_db_id) REFERENCES public.gm_agent_videos(id) ON DELETE CASCADE;


--
-- Name: gm_campaign_accounts campaign_accounts_account_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_campaign_accounts
    ADD CONSTRAINT campaign_accounts_account_id_fkey FOREIGN KEY (account_id) REFERENCES public.gm_social_accounts(id) ON DELETE CASCADE;


--
-- Name: gm_campaign_accounts campaign_accounts_campaign_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_campaign_accounts
    ADD CONSTRAINT campaign_accounts_campaign_id_fkey FOREIGN KEY (campaign_id) REFERENCES public.gm_campaigns(id) ON DELETE CASCADE;


--
-- Name: gm_campaigns campaigns_ai_model_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_campaigns
    ADD CONSTRAINT campaigns_ai_model_id_fkey FOREIGN KEY (ai_model_id) REFERENCES public.gm_ai_models(id);


--
-- Name: gm_campaigns campaigns_platform_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_campaigns
    ADD CONSTRAINT campaigns_platform_id_fkey FOREIGN KEY (platform_id) REFERENCES public.gm_platforms(id);


--
-- Name: gm_campaigns campaigns_region_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_campaigns
    ADD CONSTRAINT campaigns_region_id_fkey FOREIGN KEY (region_id) REFERENCES public.gm_regions(id);


--
-- Name: gm_campaigns campaigns_social_group_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_campaigns
    ADD CONSTRAINT campaigns_social_group_id_fkey FOREIGN KEY (social_group_id) REFERENCES public.gm_social_groups(id);


--
-- Name: gm_campaigns campaigns_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_campaigns
    ADD CONSTRAINT campaigns_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.gm_users(id);


--
-- Name: gm_crawler_results crawler_results_task_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_crawler_results
    ADD CONSTRAINT crawler_results_task_id_fkey FOREIGN KEY (task_id) REFERENCES public.gm_crawler_tasks(id) ON DELETE CASCADE;


--
-- Name: gm_crawler_tasks crawler_tasks_campaign_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_crawler_tasks
    ADD CONSTRAINT crawler_tasks_campaign_id_fkey FOREIGN KEY (campaign_id) REFERENCES public.gm_campaigns(id) ON DELETE CASCADE;


--
-- Name: gm_agent_videos crawler_tasks_id; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_videos
    ADD CONSTRAINT crawler_tasks_id FOREIGN KEY (task_id) REFERENCES public.gm_crawler_tasks(id);


--
-- Name: gm_agent_comments fk_agent_comments_campaign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_comments
    ADD CONSTRAINT fk_agent_comments_campaign FOREIGN KEY (campaign_id) REFERENCES public.gm_campaigns(id) ON DELETE CASCADE;


--
-- Name: gm_agent_videos fk_agent_videos_campaign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_videos
    ADD CONSTRAINT fk_agent_videos_campaign FOREIGN KEY (campaign_id) REFERENCES public.gm_campaigns(id) ON DELETE CASCADE;


--
-- Name: gm_campaign_templates fk_campaign_templates_campaign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_campaign_templates
    ADD CONSTRAINT fk_campaign_templates_campaign FOREIGN KEY (campaign_id) REFERENCES public.gm_campaigns(id) ON DELETE CASCADE;


--
-- Name: gm_agent_instagram_comments gm_agent_instagram_comments_campaign_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_instagram_comments
    ADD CONSTRAINT gm_agent_instagram_comments_campaign_id_fkey FOREIGN KEY (campaign_id) REFERENCES public.gm_campaigns(id) ON DELETE CASCADE;


--
-- Name: gm_agent_instagram_comments gm_agent_instagram_comments_post_db_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_instagram_comments
    ADD CONSTRAINT gm_agent_instagram_comments_post_db_id_fkey FOREIGN KEY (post_db_id) REFERENCES public.gm_agent_instagram_posts(id) ON DELETE CASCADE;


--
-- Name: gm_agent_instagram_posts gm_agent_instagram_posts_campaign_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_instagram_posts
    ADD CONSTRAINT gm_agent_instagram_posts_campaign_id_fkey FOREIGN KEY (campaign_id) REFERENCES public.gm_campaigns(id) ON DELETE CASCADE;


--
-- Name: gm_agent_instagram_posts gm_agent_instagram_posts_task_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_instagram_posts
    ADD CONSTRAINT gm_agent_instagram_posts_task_id_fkey FOREIGN KEY (task_id) REFERENCES public.gm_crawler_tasks(id) ON DELETE CASCADE;


--
-- Name: gm_agent_reddit_comments gm_agent_reddit_comments_campaign_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_reddit_comments
    ADD CONSTRAINT gm_agent_reddit_comments_campaign_id_fkey FOREIGN KEY (campaign_id) REFERENCES public.gm_campaigns(id) ON DELETE CASCADE;


--
-- Name: gm_agent_reddit_comments gm_agent_reddit_comments_post_db_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_reddit_comments
    ADD CONSTRAINT gm_agent_reddit_comments_post_db_id_fkey FOREIGN KEY (post_db_id) REFERENCES public.gm_agent_reddit_posts(id) ON DELETE CASCADE;


--
-- Name: gm_agent_reddit_posts gm_agent_reddit_posts_campaign_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_reddit_posts
    ADD CONSTRAINT gm_agent_reddit_posts_campaign_id_fkey FOREIGN KEY (campaign_id) REFERENCES public.gm_campaigns(id) ON DELETE CASCADE;


--
-- Name: gm_agent_reddit_posts gm_agent_reddit_posts_task_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_reddit_posts
    ADD CONSTRAINT gm_agent_reddit_posts_task_id_fkey FOREIGN KEY (task_id) REFERENCES public.gm_crawler_tasks(id) ON DELETE CASCADE;


--
-- Name: gm_agent_twitter_comments gm_agent_twitter_comments_campaign_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_twitter_comments
    ADD CONSTRAINT gm_agent_twitter_comments_campaign_id_fkey FOREIGN KEY (campaign_id) REFERENCES public.gm_campaigns(id) ON DELETE CASCADE;


--
-- Name: gm_agent_twitter_comments gm_agent_twitter_comments_tweet_db_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_twitter_comments
    ADD CONSTRAINT gm_agent_twitter_comments_tweet_db_id_fkey FOREIGN KEY (tweet_db_id) REFERENCES public.gm_agent_twitter_tweets(id) ON DELETE CASCADE;


--
-- Name: gm_agent_twitter_tweets gm_agent_twitter_tweets_campaign_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_twitter_tweets
    ADD CONSTRAINT gm_agent_twitter_tweets_campaign_id_fkey FOREIGN KEY (campaign_id) REFERENCES public.gm_campaigns(id) ON DELETE CASCADE;


--
-- Name: gm_agent_twitter_tweets gm_agent_twitter_tweets_task_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_twitter_tweets
    ADD CONSTRAINT gm_agent_twitter_tweets_task_id_fkey FOREIGN KEY (task_id) REFERENCES public.gm_crawler_tasks(id) ON DELETE CASCADE;


--
-- Name: gm_login_logs gm_login_logs_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_login_logs
    ADD CONSTRAINT gm_login_logs_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.gm_users(id) ON DELETE CASCADE;


--
-- Name: gm_referral_earnings gm_referral_earnings_referral_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_referral_earnings
    ADD CONSTRAINT gm_referral_earnings_referral_id_fkey FOREIGN KEY (referral_id) REFERENCES public.gm_referrals(id) ON DELETE CASCADE;


--
-- Name: gm_referrals gm_referrals_referee_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_referrals
    ADD CONSTRAINT gm_referrals_referee_id_fkey FOREIGN KEY (referee_id) REFERENCES public.gm_users(id) ON DELETE CASCADE;


--
-- Name: gm_referrals gm_referrals_referrer_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_referrals
    ADD CONSTRAINT gm_referrals_referrer_id_fkey FOREIGN KEY (referrer_id) REFERENCES public.gm_users(id) ON DELETE CASCADE;


--
-- Name: gm_upload_tasks gm_upload_tasks_platform_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_upload_tasks
    ADD CONSTRAINT gm_upload_tasks_platform_id_fkey FOREIGN KEY (platform_id) REFERENCES public.gm_platforms(id) ON DELETE SET NULL;


--
-- Name: gm_upload_tasks gm_upload_tasks_social_account_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_upload_tasks
    ADD CONSTRAINT gm_upload_tasks_social_account_id_fkey FOREIGN KEY (social_account_id) REFERENCES public.gm_social_accounts(id) ON DELETE CASCADE;


--
-- Name: gm_upload_tasks gm_upload_tasks_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_upload_tasks
    ADD CONSTRAINT gm_upload_tasks_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.gm_users(id) ON DELETE CASCADE;


--
-- Name: gm_video_generation_tasks gm_video_generation_tasks_model_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_video_generation_tasks
    ADD CONSTRAINT gm_video_generation_tasks_model_id_fkey FOREIGN KEY (model_id) REFERENCES public.gm_ai_models(id);


--
-- Name: gm_video_generation_tasks gm_video_generation_tasks_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_video_generation_tasks
    ADD CONSTRAINT gm_video_generation_tasks_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.gm_users(id);


--
-- Name: gm_video_generation_tasks gm_video_generation_tasks_wallet_transaction_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_video_generation_tasks
    ADD CONSTRAINT gm_video_generation_tasks_wallet_transaction_id_fkey FOREIGN KEY (wallet_transaction_id) REFERENCES public.gm_wallet_transactions(id);


--
-- Name: gm_pricing_rules pricing_rules_platform_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_pricing_rules
    ADD CONSTRAINT pricing_rules_platform_id_fkey FOREIGN KEY (platform_id) REFERENCES public.gm_platforms(id) ON DELETE CASCADE;


--
-- Name: gm_regions regions_platform_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_regions
    ADD CONSTRAINT regions_platform_id_fkey FOREIGN KEY (platform_id) REFERENCES public.gm_platforms(id) ON DELETE CASCADE;


--
-- Name: gm_social_accounts social_accounts_group_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_social_accounts
    ADD CONSTRAINT social_accounts_group_id_fkey FOREIGN KEY (group_id) REFERENCES public.gm_social_groups(id) ON DELETE CASCADE;


--
-- Name: gm_social_accounts social_accounts_platform_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_social_accounts
    ADD CONSTRAINT social_accounts_platform_id_fkey FOREIGN KEY (platform_id) REFERENCES public.gm_platforms(id);


--
-- Name: gm_social_accounts social_accounts_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_social_accounts
    ADD CONSTRAINT social_accounts_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.gm_users(id) ON DELETE CASCADE;


--
-- Name: gm_social_groups social_groups_platform_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_social_groups
    ADD CONSTRAINT social_groups_platform_id_fkey FOREIGN KEY (platform_id) REFERENCES public.gm_platforms(id);


--
-- Name: gm_social_groups social_groups_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_social_groups
    ADD CONSTRAINT social_groups_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.gm_users(id) ON DELETE CASCADE;


--
-- Name: gm_user_wallets user_wallets_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_user_wallets
    ADD CONSTRAINT user_wallets_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.gm_users(id) ON DELETE CASCADE;


--
-- Name: gm_wallet_transactions wallet_transactions_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_wallet_transactions
    ADD CONSTRAINT wallet_transactions_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.gm_users(id) ON DELETE CASCADE;


--
-- PostgreSQL database dump complete
--

