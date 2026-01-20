--
-- PostgreSQL database dump
--

-- Dumped from database version 15.13
-- Dumped by pg_dump version 17.5

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
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

CREATE FUNCTION public.diesel_manage_updated_at(_tbl regclass) RETURNS void
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

CREATE FUNCTION public.diesel_set_updated_at() RETURNS trigger
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


SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: __diesel_schema_migrations; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.__diesel_schema_migrations (
    version character varying(50) NOT NULL,
    run_on timestamp without time zone DEFAULT now() NOT NULL
);


--
-- Name: gm_agent_comments; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.gm_agent_comments (
    id integer NOT NULL,
    video_db_id integer NOT NULL,
    comment_id character varying(255) NOT NULL,
    user_nickname character varying(255),
    user_unique_id character varying(255),
    content text,
    reason text,
    suggested_reply text,
    create_time timestamp without time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: agent_comments_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.agent_comments_id_seq
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
-- Name: agent_reddit_comments_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.agent_reddit_comments_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: agent_reddit_comments_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.agent_reddit_comments_id_seq OWNED BY public.gm_agent_reddit_comments.id;


--
-- Name: agent_reddit_posts_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.agent_reddit_posts_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: agent_reddit_posts_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.agent_reddit_posts_id_seq OWNED BY public.gm_agent_reddit_posts.id;


--
-- Name: gm_agent_reddit_comments; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.gm_agent_reddit_comments (
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
    updated_at timestamp with time zone
);


--
-- Name: gm_agent_reddit_posts; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.gm_agent_reddit_posts (
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
-- Name: gm_agent_videos; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.gm_agent_videos (
    id integer NOT NULL,
    video_id character varying(255),
    author character varying(255),
    description text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    task_id integer NOT NULL
);


--
-- Name: agent_videos_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.agent_videos_id_seq
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

CREATE TABLE public.gm_ai_models (
    id integer NOT NULL,
    name character varying NOT NULL,
    provider character varying NOT NULL,
    model_key character varying NOT NULL,
    cost_multiplier numeric(10,2) DEFAULT 1.0 NOT NULL,
    is_active boolean DEFAULT true NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone
);


--
-- Name: ai_models_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.ai_models_id_seq
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

CREATE TABLE public.gm_campaign_accounts (
    id integer NOT NULL,
    campaign_id integer NOT NULL,
    account_id integer NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone
);


--
-- Name: campaign_accounts_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.campaign_accounts_id_seq
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
-- Name: gm_campaign_templates; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.gm_campaign_templates (
    id integer NOT NULL,
    campaign_id integer NOT NULL,
    template_content text NOT NULL,
    tone_instruction text,
    weight integer DEFAULT 10 NOT NULL,
    reply_prompt text,
    forbidden_words_prompt text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone
);


--
-- Name: campaign_templates_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.campaign_templates_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: campaign_templates_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.campaign_templates_id_seq OWNED BY public.gm_campaign_templates.id;


--
-- Name: gm_campaigns; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.gm_campaigns (
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
    additional_info text
);


--
-- Name: campaigns_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.campaigns_id_seq
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

CREATE TABLE public.gm_crawler_results (
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
    replied boolean DEFAULT false NOT NULL
);


--
-- Name: crawler_results_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.crawler_results_id_seq
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

CREATE SEQUENCE public.crawler_task_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: gm_crawler_tasks; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.gm_crawler_tasks (
    id integer NOT NULL,
    campaign_id integer NOT NULL,
    keywords text[],
    max_count integer NOT NULL,
    process_count integer DEFAULT 0 NOT NULL,
    status character varying(50) DEFAULT 'init'::character varying NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone
);


--
-- Name: crawler_tasks_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.crawler_tasks_id_seq
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
-- Name: gm_platforms; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.gm_platforms (
    id integer NOT NULL,
    name character varying NOT NULL,
    display_name character varying NOT NULL,
    is_active boolean DEFAULT true NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone,
    base_url character varying DEFAULT ''::character varying NOT NULL
);


--
-- Name: gm_pricing_rules; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.gm_pricing_rules (
    id integer NOT NULL,
    action_type character varying NOT NULL,
    platform_id integer,
    cost_points numeric(10,2) NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone
);


--
-- Name: gm_regions; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.gm_regions (
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

CREATE TABLE public.gm_social_accounts (
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
    CONSTRAINT social_accounts_health_score_check CHECK (((health_score >= 0) AND (health_score <= 100)))
);


--
-- Name: gm_social_groups; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.gm_social_groups (
    id integer NOT NULL,
    user_id integer NOT NULL,
    platform_id integer NOT NULL,
    group_name character varying NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone
);


--
-- Name: gm_user_wallets; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.gm_user_wallets (
    user_id integer NOT NULL,
    balance_points numeric(10,2) DEFAULT 0.00 NOT NULL,
    frozen_points numeric(10,2) DEFAULT 0.00 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone,
    CONSTRAINT user_wallets_balance_points_check CHECK ((balance_points >= (0)::numeric)),
    CONSTRAINT user_wallets_frozen_points_check CHECK ((frozen_points >= (0)::numeric))
);


--
-- Name: gm_users; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.gm_users (
    id integer NOT NULL,
    email character varying(255) NOT NULL,
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
    is_active boolean DEFAULT true NOT NULL
);


--
-- Name: gm_wallet_transactions; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.gm_wallet_transactions (
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

CREATE SEQUENCE public.platforms_id_seq
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

CREATE SEQUENCE public.pricing_rules_id_seq
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

CREATE SEQUENCE public.regions_id_seq
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

CREATE SEQUENCE public.social_accounts_id_seq
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

CREATE SEQUENCE public.social_groups_id_seq
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

CREATE SEQUENCE public.users_id_seq
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

CREATE SEQUENCE public.wallet_transactions_id_seq
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
-- Name: gm_agent_comments id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_comments ALTER COLUMN id SET DEFAULT nextval('public.agent_comments_id_seq'::regclass);


--
-- Name: gm_agent_reddit_comments id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_reddit_comments ALTER COLUMN id SET DEFAULT nextval('public.agent_reddit_comments_id_seq'::regclass);


--
-- Name: gm_agent_reddit_posts id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_reddit_posts ALTER COLUMN id SET DEFAULT nextval('public.agent_reddit_posts_id_seq'::regclass);


--
-- Name: gm_agent_videos id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_videos ALTER COLUMN id SET DEFAULT nextval('public.agent_videos_id_seq'::regclass);


--
-- Name: gm_ai_models id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_ai_models ALTER COLUMN id SET DEFAULT nextval('public.ai_models_id_seq'::regclass);


--
-- Name: gm_campaign_accounts id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_campaign_accounts ALTER COLUMN id SET DEFAULT nextval('public.campaign_accounts_id_seq'::regclass);


--
-- Name: gm_campaign_templates id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_campaign_templates ALTER COLUMN id SET DEFAULT nextval('public.campaign_templates_id_seq'::regclass);


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
-- Name: gm_platforms id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_platforms ALTER COLUMN id SET DEFAULT nextval('public.platforms_id_seq'::regclass);


--
-- Name: gm_pricing_rules id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_pricing_rules ALTER COLUMN id SET DEFAULT nextval('public.pricing_rules_id_seq'::regclass);


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
-- Name: gm_users id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_users ALTER COLUMN id SET DEFAULT nextval('public.users_id_seq'::regclass);


--
-- Name: gm_wallet_transactions id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_wallet_transactions ALTER COLUMN id SET DEFAULT nextval('public.wallet_transactions_id_seq'::regclass);


--
-- Name: __diesel_schema_migrations __diesel_schema_migrations_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.__diesel_schema_migrations
    ADD CONSTRAINT __diesel_schema_migrations_pkey PRIMARY KEY (version);


--
-- Name: gm_agent_comments agent_comments_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_comments
    ADD CONSTRAINT agent_comments_pkey PRIMARY KEY (id);


--
-- Name: gm_agent_reddit_comments agent_reddit_comments_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_reddit_comments
    ADD CONSTRAINT agent_reddit_comments_pkey PRIMARY KEY (id);


--
-- Name: gm_agent_reddit_posts agent_reddit_posts_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_reddit_posts
    ADD CONSTRAINT agent_reddit_posts_pkey PRIMARY KEY (id);


--
-- Name: gm_agent_reddit_posts agent_reddit_posts_task_id_post_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_reddit_posts
    ADD CONSTRAINT agent_reddit_posts_task_id_post_id_key UNIQUE (task_id, post_id);


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
-- Name: gm_campaign_templates campaign_templates_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_campaign_templates
    ADD CONSTRAINT campaign_templates_pkey PRIMARY KEY (id);


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
-- Name: gm_agent_videos gm_agent_videos_task_id_video_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_videos
    ADD CONSTRAINT gm_agent_videos_task_id_video_id_key UNIQUE (task_id, video_id);


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
-- Name: idx_agent_videos_task_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_agent_videos_task_id ON public.gm_agent_videos USING btree (task_id);


--
-- Name: idx_reddit_comments_campaign_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_reddit_comments_campaign_id ON public.gm_agent_reddit_comments USING btree (campaign_id);


--
-- Name: idx_reddit_comments_comment_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_reddit_comments_comment_id ON public.gm_agent_reddit_comments USING btree (comment_id);


--
-- Name: idx_reddit_comments_created_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_reddit_comments_created_at ON public.gm_agent_reddit_comments USING btree (created_at DESC);


--
-- Name: idx_reddit_comments_post_db_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_reddit_comments_post_db_id ON public.gm_agent_reddit_comments USING btree (post_db_id);


--
-- Name: idx_reddit_comments_status; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_reddit_comments_status ON public.gm_agent_reddit_comments USING btree (status);


--
-- Name: idx_reddit_posts_campaign_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_reddit_posts_campaign_id ON public.gm_agent_reddit_posts USING btree (campaign_id);


--
-- Name: idx_reddit_posts_created_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_reddit_posts_created_at ON public.gm_agent_reddit_posts USING btree (created_at DESC);


--
-- Name: idx_reddit_posts_post_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_reddit_posts_post_id ON public.gm_agent_reddit_posts USING btree (post_id);


--
-- Name: idx_reddit_posts_subreddit; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_reddit_posts_subreddit ON public.gm_agent_reddit_posts USING btree (subreddit);


--
-- Name: idx_reddit_posts_task_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_reddit_posts_task_id ON public.gm_agent_reddit_posts USING btree (task_id);


--
-- Name: idx_campaign_accounts_account_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_campaign_accounts_account_id ON public.gm_campaign_accounts USING btree (account_id);


--
-- Name: idx_campaign_accounts_campaign_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_campaign_accounts_campaign_id ON public.gm_campaign_accounts USING btree (campaign_id);


--
-- Name: idx_campaigns_ai_model_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_campaigns_ai_model_id ON public.gm_campaigns USING btree (ai_model_id);


--
-- Name: idx_campaigns_platform_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_campaigns_platform_id ON public.gm_campaigns USING btree (platform_id);


--
-- Name: idx_campaigns_region_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_campaigns_region_id ON public.gm_campaigns USING btree (region_id);


--
-- Name: idx_campaigns_status; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_campaigns_status ON public.gm_campaigns USING btree (status);


--
-- Name: idx_crawler_results_task_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_crawler_results_task_id ON public.gm_crawler_results USING btree (task_id);


--
-- Name: idx_crawler_results_video_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_crawler_results_video_id ON public.gm_crawler_results USING btree (video_id);


--
-- Name: idx_crawler_tasks_campaign_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_crawler_tasks_campaign_id ON public.gm_crawler_tasks USING btree (campaign_id);


--
-- Name: idx_crawler_tasks_status; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_crawler_tasks_status ON public.gm_crawler_tasks USING btree (status);


--
-- Name: idx_pricing_rules_action_platform; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_pricing_rules_action_platform ON public.gm_pricing_rules USING btree (action_type, platform_id);


--
-- Name: idx_regions_platform_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_regions_platform_id ON public.gm_regions USING btree (platform_id);


--
-- Name: idx_social_accounts_platform_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_social_accounts_platform_id ON public.gm_social_accounts USING btree (platform_id);


--
-- Name: idx_social_accounts_user_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_social_accounts_user_id ON public.gm_social_accounts USING btree (user_id);


--
-- Name: idx_social_groups_user_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_social_groups_user_id ON public.gm_social_groups USING btree (user_id);


--
-- Name: idx_users_email; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_users_email ON public.gm_users USING btree (email);


--
-- Name: idx_users_invitation_code; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_users_invitation_code ON public.gm_users USING btree (invitation_code);


--
-- Name: idx_wallet_transactions_created_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_wallet_transactions_created_at ON public.gm_wallet_transactions USING btree (created_at DESC);


--
-- Name: idx_wallet_transactions_type; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_wallet_transactions_type ON public.gm_wallet_transactions USING btree (type);


--
-- Name: idx_wallet_transactions_user_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_wallet_transactions_user_id ON public.gm_wallet_transactions USING btree (user_id);


--
-- Name: gm_ai_models set_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER set_updated_at BEFORE UPDATE ON public.gm_ai_models FOR EACH ROW EXECUTE FUNCTION public.diesel_set_updated_at();


--
-- Name: gm_agent_reddit_comments set_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER set_updated_at BEFORE UPDATE ON public.gm_agent_reddit_comments FOR EACH ROW EXECUTE FUNCTION public.diesel_set_updated_at();


--
-- Name: gm_agent_reddit_posts set_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER set_updated_at BEFORE UPDATE ON public.gm_agent_reddit_posts FOR EACH ROW EXECUTE FUNCTION public.diesel_set_updated_at();


--
-- Name: gm_campaign_accounts set_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER set_updated_at BEFORE UPDATE ON public.gm_campaign_accounts FOR EACH ROW EXECUTE FUNCTION public.diesel_set_updated_at();


--
-- Name: gm_campaign_templates set_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER set_updated_at BEFORE UPDATE ON public.gm_campaign_templates FOR EACH ROW EXECUTE FUNCTION public.diesel_set_updated_at();


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
-- Name: gm_agent_comments agent_comments_video_db_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_comments
    ADD CONSTRAINT agent_comments_video_db_id_fkey FOREIGN KEY (video_db_id) REFERENCES public.gm_agent_videos(id) ON DELETE CASCADE;


--
-- Name: gm_agent_reddit_comments reddit_comments_campaign_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_reddit_comments
    ADD CONSTRAINT reddit_comments_campaign_id_fkey FOREIGN KEY (campaign_id) REFERENCES public.gm_campaigns(id) ON DELETE CASCADE;


--
-- Name: gm_agent_reddit_comments reddit_comments_post_db_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_reddit_comments
    ADD CONSTRAINT reddit_comments_post_db_id_fkey FOREIGN KEY (post_db_id) REFERENCES public.gm_agent_reddit_posts(id) ON DELETE CASCADE;


--
-- Name: gm_agent_reddit_posts reddit_posts_campaign_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_reddit_posts
    ADD CONSTRAINT reddit_posts_campaign_id_fkey FOREIGN KEY (campaign_id) REFERENCES public.gm_campaigns(id) ON DELETE CASCADE;


--
-- Name: gm_agent_reddit_posts reddit_posts_task_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_agent_reddit_posts
    ADD CONSTRAINT reddit_posts_task_id_fkey FOREIGN KEY (task_id) REFERENCES public.gm_crawler_tasks(id) ON DELETE CASCADE;


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
-- Name: gm_campaign_templates campaign_templates_campaign_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gm_campaign_templates
    ADD CONSTRAINT campaign_templates_campaign_id_fkey FOREIGN KEY (campaign_id) REFERENCES public.gm_campaigns(id) ON DELETE CASCADE;


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


--
-- Data for Name: gm_ai_models; Type: TABLE DATA; Schema: public; Owner: -
--

INSERT INTO public.gm_ai_models VALUES (1, 'GPT-4o', 'OPENAI', 'gpt-4o-2024-05-13', 2.00, true, '2025-12-31 04:36:30.18398+00', NULL);
INSERT INTO public.gm_ai_models VALUES (2, 'GPT-3.5 Turbo', 'OPENAI', 'gpt-3.5-turbo', 0.50, true, '2025-12-31 04:36:30.18398+00', NULL);
INSERT INTO public.gm_ai_models VALUES (3, 'Claude 3.5 Sonnet', 'ANTHROPIC', 'claude-3-5-sonnet-20241022', 1.50, true, '2025-12-31 04:36:30.18398+00', NULL);
INSERT INTO public.gm_ai_models VALUES (4, 'Gemini 1.5 Pro', 'GEMINI', 'gemini-1.5-pro', 1.00, true, '2025-12-31 04:36:30.18398+00', NULL);


--
-- Data for Name: gm_platforms; Type: TABLE DATA; Schema: public; Owner: -
--

INSERT INTO public.gm_platforms VALUES (1, 'REDDIT', 'Reddit', true, '2025-12-31 04:36:30.178207+00', NULL, '');
INSERT INTO public.gm_platforms VALUES (2, 'TIKTOK', 'TikTok', true, '2025-12-31 04:36:30.178207+00', NULL, '');
INSERT INTO public.gm_platforms VALUES (3, 'FACEBOOK', 'Facebook', true, '2025-12-31 04:36:30.178207+00', NULL, '');


--
-- Data for Name: gm_pricing_rules; Type: TABLE DATA; Schema: public; Owner: -
--

INSERT INTO public.gm_pricing_rules VALUES (1, 'SCAN_POST', NULL, 0.01, '2025-12-31 04:36:30.186477+00', NULL);
INSERT INTO public.gm_pricing_rules VALUES (2, 'AI_ANALYZE', NULL, 0.05, '2025-12-31 04:36:30.186477+00', NULL);
INSERT INTO public.gm_pricing_rules VALUES (3, 'GENERATE_REPLY', NULL, 0.10, '2025-12-31 04:36:30.186477+00', NULL);
INSERT INTO public.gm_pricing_rules VALUES (4, 'POST_REPLY', NULL, 0.20, '2025-12-31 04:36:30.186477+00', NULL);


--
-- Data for Name: gm_regions; Type: TABLE DATA; Schema: public; Owner: -
--

INSERT INTO public.gm_regions VALUES (1, 1, 'GLOBAL', 'Global', true, '2025-12-31 04:36:30.180287+00', NULL, '');
INSERT INTO public.gm_regions VALUES (2, 1, 'US', 'United States', true, '2025-12-31 04:36:30.180287+00', NULL, '');
INSERT INTO public.gm_regions VALUES (3, 1, 'UK', 'United Kingdom', true, '2025-12-31 04:36:30.180287+00', NULL, '');
INSERT INTO public.gm_regions VALUES (4, 1, 'JP', 'Japan', true, '2025-12-31 04:36:30.180287+00', NULL, '');
INSERT INTO public.gm_regions VALUES (5, 1, 'CN', 'China', true, '2025-12-31 04:36:30.180287+00', NULL, '');
INSERT INTO public.gm_regions VALUES (6, 1, 'EU', 'European Union', true, '2025-12-31 04:36:30.180287+00', NULL, '');
INSERT INTO public.gm_regions VALUES (7, 2, 'GLOBAL', 'Global', true, '2025-12-31 04:36:30.180287+00', NULL, '');
INSERT INTO public.gm_regions VALUES (8, 2, 'US', 'United States', true, '2025-12-31 04:36:30.180287+00', NULL, '');
INSERT INTO public.gm_regions VALUES (9, 2, 'UK', 'United Kingdom', true, '2025-12-31 04:36:30.180287+00', NULL, '');
INSERT INTO public.gm_regions VALUES (10, 2, 'JP', 'Japan', true, '2025-12-31 04:36:30.180287+00', NULL, '');
INSERT INTO public.gm_regions VALUES (11, 2, 'CN', 'China', true, '2025-12-31 04:36:30.180287+00', NULL, '');
INSERT INTO public.gm_regions VALUES (12, 2, 'EU', 'European Union', true, '2025-12-31 04:36:30.180287+00', NULL, '');
INSERT INTO public.gm_regions VALUES (13, 3, 'GLOBAL', 'Global', true, '2025-12-31 04:36:30.180287+00', NULL, '');
INSERT INTO public.gm_regions VALUES (14, 3, 'US', 'United States', true, '2025-12-31 04:36:30.180287+00', NULL, '');
INSERT INTO public.gm_regions VALUES (15, 3, 'UK', 'United Kingdom', true, '2025-12-31 04:36:30.180287+00', NULL, '');
INSERT INTO public.gm_regions VALUES (16, 3, 'JP', 'Japan', true, '2025-12-31 04:36:30.180287+00', NULL, '');
INSERT INTO public.gm_regions VALUES (17, 3, 'CN', 'China', true, '2025-12-31 04:36:30.180287+00', NULL, '');
INSERT INTO public.gm_regions VALUES (18, 3, 'EU', 'European Union', true, '2025-12-31 04:36:30.180287+00', NULL, '');
