// @generated automatically by Diesel CLI.

diesel::table! {
    drama_chapter_scene_asset_links (project_id, chapter_id, scene_asset_id) {
        project_id -> Text,
        chapter_id -> Text,
        user_id -> Int8,
        scene_asset_id -> Text,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    drama_private_characters (id) {
        id -> Text,
        user_id -> Int8,
        name -> Text,
        gender -> Nullable<Text>,
        age -> Nullable<Text>,
        appearance -> Nullable<Text>,
        personality -> Nullable<Text>,
        voice_id -> Nullable<Text>,
        reference_image_url -> Nullable<Text>,
        notes -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    drama_private_scene_assets (id) {
        id -> Text,
        user_id -> Int8,
        name -> Text,
        category -> Nullable<Text>,
        location_description -> Nullable<Text>,
        time_of_day -> Nullable<Text>,
        mood -> Nullable<Text>,
        reference_image_urls -> Jsonb,
        camera_notes -> Nullable<Text>,
        notes -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    drama_private_style_assets (id) {
        id -> Text,
        user_id -> Int8,
        name -> Text,
        visual_style -> Nullable<Text>,
        color_tone -> Nullable<Text>,
        aspect_ratio -> Nullable<Text>,
        resolution -> Nullable<Text>,
        lighting_mood -> Nullable<Text>,
        reference_image_urls -> Jsonb,
        notes -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    drama_project_character_links (project_id, character_id) {
        project_id -> Text,
        user_id -> Int8,
        character_id -> Text,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    drama_project_meta (project_id) {
        project_id -> Text,
        user_id -> Int8,
        characters -> Jsonb,
        style_references -> Jsonb,
        text_materials -> Jsonb,
        visual_settings -> Jsonb,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    drama_project_scene_asset_links (project_id, scene_asset_id) {
        project_id -> Text,
        user_id -> Int8,
        scene_asset_id -> Text,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    drama_project_style_asset_links (project_id, style_asset_id) {
        project_id -> Text,
        user_id -> Int8,
        style_asset_id -> Text,
        is_primary -> Bool,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    gm_admin_users (id) {
        id -> Int4,
        #[max_length = 50]
        username -> Varchar,
        #[max_length = 255]
        email -> Varchar,
        #[max_length = 255]
        password_hash -> Varchar,
        #[max_length = 255]
        full_name -> Varchar,
        #[max_length = 20]
        role -> Varchar,
        is_active -> Bool,
        last_login_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    gm_agent_comments (id) {
        id -> Int4,
        video_db_id -> Int4,
        #[max_length = 255]
        comment_id -> Varchar,
        #[max_length = 255]
        user_nickname -> Nullable<Varchar>,
        #[max_length = 255]
        user_unique_id -> Nullable<Varchar>,
        content -> Nullable<Text>,
        reason -> Nullable<Text>,
        suggested_reply -> Nullable<Text>,
        create_time -> Nullable<Timestamp>,
        created_at -> Timestamptz,
        campaign_id -> Nullable<Int4>,
        status -> Int2,
        suggested_dm -> Nullable<Text>,
        suggested_reply_post -> Nullable<Text>,
        updated_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    gm_agent_facebook_comments (id) {
        id -> Int4,
        post_db_id -> Int4,
        campaign_id -> Nullable<Int4>,
        #[max_length = 255]
        facebook_comment_id -> Varchar,
        #[max_length = 255]
        parent_comment_id -> Nullable<Varchar>,
        comment_url -> Nullable<Text>,
        comment_text -> Text,
        reason -> Nullable<Text>,
        suggested_reply -> Nullable<Text>,
        suggested_dm -> Nullable<Text>,
        suggested_reply_post -> Nullable<Text>,
        status -> Nullable<Int2>,
        #[max_length = 255]
        comment_user_id -> Nullable<Varchar>,
        #[max_length = 255]
        comment_username -> Nullable<Varchar>,
        comment_user_url -> Nullable<Text>,
        comment_user_profile_picture -> Nullable<Text>,
        like_count -> Nullable<Int4>,
        reply_count -> Nullable<Int4>,
        threading_depth -> Nullable<Int4>,
        created_at_ts -> Nullable<Int8>,
        comment_created_at -> Nullable<Timestamptz>,
        #[max_length = 255]
        facebook_post_id -> Nullable<Varchar>,
        post_url -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    gm_agent_facebook_posts (id) {
        id -> Int4,
        task_id -> Int4,
        campaign_id -> Nullable<Int4>,
        #[max_length = 255]
        facebook_post_id -> Varchar,
        #[max_length = 50]
        post_type -> Nullable<Varchar>,
        url -> Nullable<Text>,
        message -> Nullable<Text>,
        message_rich -> Nullable<Text>,
        timestamp -> Nullable<Int8>,
        posted_at -> Nullable<Timestamptz>,
        reactions_count -> Nullable<Int4>,
        comments_count -> Nullable<Int4>,
        reshare_count -> Nullable<Int4>,
        reactions_like -> Nullable<Int4>,
        reactions_love -> Nullable<Int4>,
        reactions_haha -> Nullable<Int4>,
        reactions_wow -> Nullable<Int4>,
        reactions_sad -> Nullable<Int4>,
        reactions_angry -> Nullable<Int4>,
        reactions_care -> Nullable<Int4>,
        #[max_length = 255]
        author_id -> Nullable<Varchar>,
        #[max_length = 255]
        author_name -> Nullable<Varchar>,
        author_url -> Nullable<Text>,
        author_profile_picture_url -> Nullable<Text>,
        #[max_length = 255]
        author_title -> Nullable<Varchar>,
        has_image -> Nullable<Bool>,
        image_url -> Nullable<Text>,
        image_width -> Nullable<Int4>,
        image_height -> Nullable<Int4>,
        #[max_length = 255]
        image_id -> Nullable<Varchar>,
        has_video -> Nullable<Bool>,
        video_thumbnail -> Nullable<Text>,
        external_url -> Nullable<Text>,
        attached_post_url -> Nullable<Text>,
        #[max_length = 255]
        comments_id -> Nullable<Varchar>,
        #[max_length = 255]
        shares_id -> Nullable<Varchar>,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    gm_agent_instagram_comments (id) {
        id -> Int4,
        post_db_id -> Int4,
        campaign_id -> Nullable<Int4>,
        #[max_length = 255]
        instagram_comment_id -> Varchar,
        #[max_length = 255]
        parent_comment_id -> Nullable<Varchar>,
        comment_text -> Text,
        reason -> Nullable<Text>,
        suggested_reply -> Nullable<Text>,
        #[max_length = 255]
        comment_user_id -> Nullable<Varchar>,
        #[max_length = 255]
        comment_username -> Nullable<Varchar>,
        #[max_length = 255]
        comment_user_full_name -> Nullable<Varchar>,
        like_count -> Nullable<Int4>,
        comment_like_count -> Nullable<Int4>,
        child_comment_count -> Nullable<Int4>,
        created_at_ts -> Nullable<Int8>,
        comment_created_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
        suggested_dm -> Nullable<Text>,
        suggested_reply_post -> Nullable<Text>,
        status -> Nullable<Int2>,
    }
}

diesel::table! {
    gm_agent_instagram_posts (id) {
        id -> Int4,
        task_id -> Int4,
        campaign_id -> Nullable<Int4>,
        #[max_length = 255]
        code -> Varchar,
        #[max_length = 255]
        instagram_id -> Nullable<Varchar>,
        media_type -> Nullable<Int4>,
        #[max_length = 50]
        product_type -> Nullable<Varchar>,
        caption_text -> Nullable<Text>,
        #[max_length = 255]
        owner_username -> Nullable<Varchar>,
        #[max_length = 255]
        owner_id -> Nullable<Varchar>,
        #[max_length = 255]
        owner_full_name -> Nullable<Varchar>,
        media_url -> Nullable<Text>,
        thumbnail_url -> Nullable<Text>,
        like_count -> Nullable<Int4>,
        comment_count -> Nullable<Int4>,
        play_count -> Nullable<Int4>,
        taken_at_ts -> Nullable<Int8>,
        posted_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    gm_agent_reddit_comments (id) {
        id -> Int4,
        post_db_id -> Int4,
        campaign_id -> Nullable<Int4>,
        #[max_length = 255]
        comment_id -> Varchar,
        #[max_length = 255]
        comment_name -> Varchar,
        #[max_length = 255]
        author -> Nullable<Varchar>,
        body -> Nullable<Text>,
        reason -> Nullable<Text>,
        suggested_reply -> Nullable<Text>,
        score -> Nullable<Int4>,
        #[max_length = 255]
        parent_id -> Nullable<Varchar>,
        is_reply -> Nullable<Bool>,
        depth -> Nullable<Int4>,
        comment_created_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
        suggested_dm -> Nullable<Text>,
        suggested_reply_post -> Nullable<Text>,
        status -> Nullable<Int2>,
    }
}

diesel::table! {
    gm_agent_reddit_posts (id) {
        id -> Int4,
        task_id -> Int4,
        campaign_id -> Nullable<Int4>,
        #[max_length = 255]
        post_id -> Varchar,
        #[max_length = 255]
        post_name -> Varchar,
        title -> Text,
        selftext -> Nullable<Text>,
        #[max_length = 255]
        author -> Nullable<Varchar>,
        #[max_length = 255]
        subreddit -> Varchar,
        url -> Nullable<Text>,
        permalink -> Nullable<Text>,
        #[max_length = 255]
        domain -> Nullable<Varchar>,
        thumbnail -> Nullable<Text>,
        score -> Nullable<Int4>,
        upvote_ratio -> Nullable<Numeric>,
        num_comments -> Nullable<Int4>,
        is_video -> Nullable<Bool>,
        post_created_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    gm_agent_twitter_comments (id) {
        id -> Int4,
        tweet_db_id -> Int4,
        campaign_id -> Nullable<Int4>,
        #[max_length = 255]
        twitter_comment_id -> Varchar,
        #[max_length = 255]
        conversation_id -> Nullable<Varchar>,
        #[max_length = 255]
        comment_screen_name -> Nullable<Varchar>,
        #[max_length = 255]
        comment_user_name -> Nullable<Varchar>,
        #[max_length = 255]
        comment_user_id -> Nullable<Varchar>,
        comment_user_followers -> Nullable<Int4>,
        comment_text -> Text,
        reason -> Nullable<Text>,
        suggested_reply -> Nullable<Text>,
        favorite_count -> Nullable<Int4>,
        retweet_count -> Nullable<Int4>,
        reply_count -> Nullable<Int4>,
        #[max_length = 255]
        in_reply_to_status_id -> Nullable<Varchar>,
        is_reply -> Nullable<Bool>,
        media_urls -> Nullable<Array<Nullable<Text>>>,
        has_media -> Nullable<Bool>,
        #[max_length = 255]
        created_at_str -> Nullable<Varchar>,
        created_at_ts -> Nullable<Int8>,
        comment_created_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
        suggested_dm -> Nullable<Text>,
        suggested_reply_post -> Nullable<Text>,
        status -> Nullable<Int2>,
    }
}

diesel::table! {
    gm_agent_twitter_tweets (id) {
        id -> Int4,
        task_id -> Int4,
        campaign_id -> Nullable<Int4>,
        #[max_length = 255]
        twitter_tweet_id -> Varchar,
        #[max_length = 255]
        conversation_id -> Nullable<Varchar>,
        full_text -> Text,
        #[max_length = 10]
        lang -> Nullable<Varchar>,
        #[max_length = 255]
        screen_name -> Nullable<Varchar>,
        #[max_length = 255]
        user_name -> Nullable<Varchar>,
        #[max_length = 255]
        user_id -> Nullable<Varchar>,
        user_description -> Nullable<Text>,
        user_followers_count -> Nullable<Int4>,
        user_avatar -> Nullable<Text>,
        user_verified -> Nullable<Bool>,
        media_urls -> Nullable<Array<Nullable<Text>>>,
        has_media -> Nullable<Bool>,
        favorite_count -> Nullable<Int4>,
        retweet_count -> Nullable<Int4>,
        reply_count -> Nullable<Int4>,
        quote_count -> Nullable<Int4>,
        bookmark_count -> Nullable<Int4>,
        view_count -> Nullable<Int4>,
        is_reply -> Nullable<Bool>,
        #[max_length = 255]
        in_reply_to_status_id -> Nullable<Varchar>,
        #[max_length = 255]
        in_reply_to_user_id -> Nullable<Varchar>,
        #[max_length = 255]
        created_at_str -> Nullable<Varchar>,
        created_at_ts -> Nullable<Int8>,
        tweet_created_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    gm_agent_videos (id) {
        id -> Int4,
        #[max_length = 255]
        video_id -> Nullable<Varchar>,
        #[max_length = 255]
        author -> Nullable<Varchar>,
        description -> Nullable<Text>,
        created_at -> Timestamptz,
        task_id -> Int4,
        campaign_id -> Nullable<Int4>,
        like_count -> Nullable<Int4>,
        comment_count -> Nullable<Int4>,
        share_count -> Nullable<Int4>,
        play_count -> Nullable<Int4>,
        publish_time -> Nullable<Int8>,
        #[max_length = 255]
        author_unique_id -> Nullable<Varchar>,
        url -> Nullable<Text>,
        updated_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    gm_ai_models (id) {
        id -> Int4,
        name -> Varchar,
        provider -> Varchar,
        model_key -> Varchar,
        cost_multiplier -> Numeric,
        is_active -> Bool,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
        #[max_length = 50]
        model_type -> Varchar,
    }
}

diesel::table! {
    gm_ai_video_models (id) {
        id -> Int4,
        #[max_length = 100]
        model_key -> Varchar,
        #[max_length = 255]
        model_name -> Varchar,
        #[max_length = 100]
        provider -> Varchar,
        description -> Nullable<Text>,
        features -> Nullable<Jsonb>,
        cost_per_generation -> Numeric,
        cost_per_upload -> Nullable<Numeric>,
        #[max_length = 500]
        api_endpoint -> Nullable<Varchar>,
        #[max_length = 50]
        model_version -> Nullable<Varchar>,
        max_prompt_length -> Nullable<Int4>,
        supported_formats -> Nullable<Jsonb>,
        max_image_size_mb -> Nullable<Int4>,
        estimated_time_minutes -> Nullable<Int4>,
        daily_limit -> Nullable<Int4>,
        is_active -> Bool,
        is_default -> Bool,
        sort_order -> Int4,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    gm_aipub_ai_tasks (id) {
        id -> Int4,
        plan_id -> Int4,
        #[max_length = 20]
        task_type -> Varchar,
        #[max_length = 50]
        external_service -> Varchar,
        #[max_length = 200]
        external_job_id -> Nullable<Varchar>,
        input -> Jsonb,
        result -> Nullable<Jsonb>,
        #[max_length = 20]
        status -> Varchar,
        progress -> Nullable<Int4>,
        error_message -> Nullable<Text>,
        retry_count -> Nullable<Int4>,
        sequence -> Int4,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
        completed_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    gm_aipub_plans (id) {
        id -> Int4,
        user_id -> Int4,
        group_id -> Nullable<Int4>,
        social_account_id -> Nullable<Int4>,
        platform_id -> Int4,
        #[max_length = 20]
        content_type -> Varchar,
        #[max_length = 20]
        plan_type -> Varchar,
        ai_task_types -> Nullable<Array<Nullable<Text>>>,
        ai_service_config -> Nullable<Jsonb>,
        ai_input -> Nullable<Jsonb>,
        content -> Nullable<Jsonb>,
        #[max_length = 20]
        status -> Varchar,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
        #[max_length = 200]
        name -> Nullable<Varchar>,
        image_ai_model_id -> Nullable<Int4>,
        #[max_length = 20]
        billing_status -> Varchar,
        frozen_cost -> Numeric,
        consumed_cost -> Numeric,
        frozen_at -> Nullable<Timestamptz>,
        chat_ai_model_id -> Nullable<Int4>,
        video_ai_model_id -> Nullable<Int4>,
        behavior -> Nullable<Jsonb>,
        schedule -> Nullable<Jsonb>,
    }
}

diesel::table! {
    gm_aipub_tasks (id) {
        id -> Int4,
        plan_id -> Int4,
        social_account_id -> Int4,
        ai_task_id -> Nullable<Int4>,
        content -> Jsonb,
        #[max_length = 20]
        status -> Varchar,
        #[max_length = 500]
        result_url -> Nullable<Varchar>,
        error_message -> Nullable<Text>,
        retry_count -> Nullable<Int4>,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
        published_at -> Nullable<Timestamptz>,
        video_stage_started_at -> Nullable<Timestamptz>,
        video_ai_task_id -> Nullable<Int4>,
        media_results -> Nullable<Jsonb>,
        post_publish_results -> Nullable<Jsonb>,
        #[max_length = 64]
        failed_error_code -> Nullable<Varchar>,
        execution_log -> Nullable<Text>,
        #[max_length = 128]
        platform_post_id -> Nullable<Varchar>,
    }
}

diesel::table! {
    gm_campaign_accounts (id) {
        id -> Int4,
        campaign_id -> Int4,
        account_id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    gm_campaign_templates (id) {
        id -> Int4,
        campaign_id -> Int4,
        library_template_id -> Nullable<Int4>,
        weight -> Int4,
        reply_prompt -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
        dm_prompt -> Nullable<Text>,
        reply_post_prompt -> Nullable<Text>,
        #[max_length = 255]
        name -> Nullable<Varchar>,
    }
}

diesel::table! {
    gm_reply_template_library (id) {
        id -> Int4,
        user_id -> Int4,
        #[max_length = 255]
        name -> Varchar,
        description -> Nullable<Text>,
        weight -> Int4,
        dm_prompt -> Nullable<Text>,
        reply_prompt -> Nullable<Text>,
        reply_post_prompt -> Nullable<Text>,
        usage_count -> Int4,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    gm_campaigns (id) {
        id -> Int4,
        user_id -> Int4,
        name -> Varchar,
        status -> Varchar,
        platform_id -> Int4,
        region_id -> Int4,
        ai_model_id -> Int4,
        target_audience -> Nullable<Text>,
        product_prompt -> Text,
        schedule_config -> Nullable<Jsonb>,
        enable_ai_refactor -> Nullable<Bool>,
        persona_id -> Nullable<Int4>,
        max_scan_count -> Nullable<Int4>,
        budget_cap -> Nullable<Numeric>,
        end_date -> Nullable<Timestamptz>,
        schedule_type -> Varchar,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
        keyword -> Nullable<Text>,
        social_group_id -> Nullable<Int4>,
        call_to_action -> Nullable<Text>,
        tone_of_voice -> Nullable<Text>,
        additional_info -> Nullable<Text>,
        total_scanned -> Int4,
        auto_like -> Bool,
        auto_follow -> Bool,
        auto_dm -> Bool,
        pending_consumption -> Numeric,
        actual_consumption -> Numeric,
        is_frozen -> Bool,
        search_options -> Nullable<Jsonb>,
        auto_reply_comments -> Bool,
        auto_reply_post -> Bool,
        completed_reason -> Nullable<Text>,
    }
}

diesel::table! {
    gm_crawler_results (id) {
        id -> Int4,
        task_id -> Int4,
        #[max_length = 255]
        video_id -> Varchar,
        video_title -> Nullable<Text>,
        comment_count -> Nullable<Int4>,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
        view_count -> Nullable<Int4>,
        #[max_length = 255]
        author_name -> Nullable<Varchar>,
        processed -> Bool,
        replied -> Bool,
        like_count -> Nullable<Int4>,
    }
}

diesel::table! {
    gm_crawler_tasks (id) {
        id -> Int4,
        campaign_id -> Int4,
        keywords -> Nullable<Array<Nullable<Text>>>,
        max_count -> Int4,
        process_count -> Int4,
        #[max_length = 50]
        status -> Varchar,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
        search_offset -> Int4,
        search_limit -> Int4,
        reserved_amount -> Nullable<Numeric>,
        actual_consumption -> Nullable<Numeric>,
        settled_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    gm_drama_callback_events (id) {
        id -> Int8,
        #[max_length = 255]
        event_id -> Varchar,
        #[max_length = 255]
        project_id -> Varchar,
        #[max_length = 255]
        run_id -> Varchar,
        sequence -> Int8,
        #[max_length = 50]
        stage_code -> Nullable<Varchar>,
        #[max_length = 100]
        event_type -> Varchar,
        payload -> Jsonb,
        occurred_at -> Timestamptz,
        ingested_at -> Timestamptz,
    }
}

diesel::table! {
    gm_drama_cost_events (id) {
        id -> Int8,
        #[max_length = 255]
        project_id -> Varchar,
        #[max_length = 255]
        run_id -> Nullable<Varchar>,
        #[max_length = 100]
        cost_type -> Varchar,
        amount_cents -> Int8,
        #[max_length = 50]
        stage_code -> Nullable<Varchar>,
        #[max_length = 100]
        provider -> Nullable<Varchar>,
        #[max_length = 255]
        event_id -> Nullable<Varchar>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    gm_drama_project_projections (id) {
        id -> Int4,
        #[max_length = 255]
        project_id -> Varchar,
        user_id -> Int4,
        #[max_length = 500]
        title -> Varchar,
        description -> Text,
        #[max_length = 50]
        status -> Varchar,
        #[max_length = 50]
        content_type -> Nullable<Varchar>,
        #[max_length = 50]
        platform -> Nullable<Varchar>,
        #[max_length = 50]
        current_stage -> Nullable<Varchar>,
        #[max_length = 50]
        pending_stage -> Nullable<Varchar>,
        progress_percent -> Float4,
        #[max_length = 255]
        run_id -> Nullable<Varchar>,
        interaction_version -> Int4,
        last_event_sequence -> Int8,
        error_message -> Nullable<Text>,
        cost_reserve_cents -> Int8,
        cost_consumed_cents -> Int8,
        interaction_payload -> Nullable<Jsonb>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        completed_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    gm_email_verifications (id) {
        id -> Int4,
        #[max_length = 255]
        email -> Varchar,
        #[max_length = 6]
        code -> Varchar,
        expires_at -> Timestamptz,
        verified -> Bool,
        created_at -> Timestamptz,
        #[max_length = 45]
        ip_address -> Nullable<Varchar>,
        user_agent -> Nullable<Text>,
    }
}

diesel::table! {
    gm_login_logs (id) {
        id -> Int4,
        user_id -> Int4,
        ip_address -> Nullable<Text>,
        user_agent -> Nullable<Text>,
        #[max_length = 20]
        login_status -> Varchar,
        failure_reason -> Nullable<Text>,
        login_at -> Timestamptz,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    gm_material_folders (id) {
        id -> Int4,
        user_id -> Int4,
        parent_id -> Nullable<Int4>,
        #[max_length = 255]
        name -> Varchar,
        depth -> Int2,
        sort_order -> Int4,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    gm_notifications (id) {
        id -> Int4,
        #[max_length = 20]
        notification_type -> Varchar,
        #[max_length = 255]
        title -> Varchar,
        #[max_length = 255]
        title_zh -> Nullable<Varchar>,
        description -> Nullable<Text>,
        description_zh -> Nullable<Text>,
        #[max_length = 500]
        link -> Nullable<Varchar>,
        #[max_length = 100]
        link_text -> Nullable<Varchar>,
        #[max_length = 100]
        link_text_zh -> Nullable<Varchar>,
        important -> Nullable<Bool>,
        published_at -> Nullable<Timestamptz>,
        expires_at -> Nullable<Timestamptz>,
        created_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    gm_novel_architecture_checkpoints (id) {
        id -> Int8,
        project_id -> Text,
        core_seed_result -> Nullable<Text>,
        character_dynamics_result -> Nullable<Text>,
        character_state_result -> Nullable<Text>,
        world_building_result -> Nullable<Text>,
        plot_arch_result -> Nullable<Text>,
        #[max_length = 32]
        status -> Varchar,
        source_stage_run_id -> Nullable<Int8>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    gm_novel_architectures (id) {
        id -> Int8,
        project_id -> Text,
        core_seed_text -> Text,
        character_dynamics_text -> Text,
        world_building_text -> Text,
        plot_architecture_text -> Text,
        full_text -> Text,
        version_no -> Int4,
        is_current -> Bool,
        source_stage_run_id -> Nullable<Int8>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    gm_novel_blueprint_chapters (id) {
        id -> Int8,
        blueprint_id -> Int8,
        project_id -> Text,
        chapter_number -> Int4,
        #[max_length = 255]
        chapter_title -> Varchar,
        #[max_length = 255]
        chapter_role -> Varchar,
        chapter_purpose -> Text,
        #[max_length = 100]
        suspense_level -> Varchar,
        foreshadowing -> Text,
        #[max_length = 100]
        plot_twist_level -> Varchar,
        chapter_summary -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    gm_novel_blueprints (id) {
        id -> Int8,
        project_id -> Text,
        raw_text -> Text,
        chunk_size -> Nullable<Int4>,
        generated_chapter_count -> Int4,
        version_no -> Int4,
        is_current -> Bool,
        source_stage_run_id -> Nullable<Int8>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    gm_novel_chapter_prompts (id) {
        id -> Int8,
        project_id -> Text,
        chapter_number -> Int4,
        blueprint_chapter_id -> Nullable<Int8>,
        user_guidance -> Nullable<Text>,
        characters_involved -> Nullable<Text>,
        key_items -> Nullable<Text>,
        scene_location -> Nullable<Text>,
        time_constraint -> Nullable<Text>,
        short_summary -> Nullable<Text>,
        previous_excerpt -> Nullable<Text>,
        filtered_context -> Nullable<Text>,
        prompt_text -> Text,
        edited_prompt_text -> Nullable<Text>,
        is_current -> Bool,
        source_stage_run_id -> Nullable<Int8>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    gm_novel_chapters (id) {
        id -> Int8,
        project_id -> Text,
        chapter_number -> Int4,
        blueprint_chapter_id -> Nullable<Int8>,
        prompt_id -> Nullable<Int8>,
        #[max_length = 255]
        title -> Varchar,
        user_guidance -> Nullable<Text>,
        characters_involved -> Nullable<Text>,
        key_items -> Nullable<Text>,
        scene_location -> Nullable<Text>,
        time_constraint -> Nullable<Text>,
        draft_text -> Nullable<Text>,
        final_text -> Nullable<Text>,
        #[max_length = 32]
        status -> Varchar,
        is_enriched -> Bool,
        target_words -> Nullable<Int4>,
        draft_word_count -> Int4,
        final_word_count -> Int4,
        #[max_length = 32]
        consistency_status -> Nullable<Varchar>,
        source_stage_run_id -> Nullable<Int8>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        finalized_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    gm_novel_character_state_snapshots (id) {
        id -> Int8,
        project_id -> Text,
        chapter_number -> Nullable<Int4>,
        state_text -> Text,
        version_no -> Int4,
        is_current -> Bool,
        source_stage_run_id -> Nullable<Int8>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    gm_novel_consistency_checks (id) {
        id -> Int8,
        project_id -> Text,
        chapter_number -> Int4,
        novel_setting_text -> Nullable<Text>,
        character_state_text -> Nullable<Text>,
        global_summary_text -> Nullable<Text>,
        plot_arcs_text -> Nullable<Text>,
        chapter_text -> Text,
        result_text -> Text,
        #[max_length = 32]
        status -> Varchar,
        source_stage_run_id -> Nullable<Int8>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    gm_novel_embedding_profiles (id) {
        id -> Int8,
        user_id -> Int4,
        #[max_length = 100]
        name -> Varchar,
        #[max_length = 50]
        interface_format -> Varchar,
        base_url -> Text,
        api_key -> Text,
        #[max_length = 200]
        model_name -> Varchar,
        retrieval_k -> Int4,
        is_default -> Bool,
        is_active -> Bool,
        metadata -> Jsonb,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        deleted_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    gm_novel_global_summary_snapshots (id) {
        id -> Int8,
        project_id -> Text,
        chapter_number -> Nullable<Int4>,
        summary_text -> Text,
        version_no -> Int4,
        is_current -> Bool,
        source_stage_run_id -> Nullable<Int8>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    gm_novel_jobs (id) {
        id -> Int8,
        project_id -> Text,
        chapter_number -> Nullable<Int4>,
        #[max_length = 50]
        stage_code -> Varchar,
        #[max_length = 50]
        task_type -> Varchar,
        #[max_length = 32]
        status -> Varchar,
        #[max_length = 255]
        idempotency_key -> Nullable<Varchar>,
        request_payload -> Jsonb,
        result_payload -> Jsonb,
        error_payload -> Jsonb,
        created_by -> Nullable<Int4>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        started_at -> Nullable<Timestamptz>,
        completed_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    gm_novel_knowledge_chunks (id) {
        id -> Int8,
        project_id -> Text,
        knowledge_import_id -> Int8,
        chunk_index -> Int4,
        content -> Text,
        metadata -> Jsonb,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    gm_novel_knowledge_imports (id) {
        id -> Int8,
        project_id -> Text,
        #[max_length = 255]
        source_name -> Varchar,
        #[max_length = 50]
        source_type -> Varchar,
        original_text -> Text,
        segment_count -> Int4,
        #[max_length = 32]
        status -> Varchar,
        source_stage_run_id -> Nullable<Int8>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    gm_novel_llm_profiles (id) {
        id -> Int8,
        user_id -> Int4,
        #[max_length = 100]
        name -> Varchar,
        #[max_length = 50]
        interface_format -> Varchar,
        base_url -> Text,
        api_key -> Text,
        #[max_length = 200]
        model_name -> Varchar,
        temperature -> Float8,
        max_tokens -> Int4,
        timeout_seconds -> Int4,
        is_default -> Bool,
        is_active -> Bool,
        metadata -> Jsonb,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        deleted_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    gm_novel_memory_chunks (id) {
        id -> Int8,
        project_id -> Text,
        #[max_length = 50]
        source_type -> Varchar,
        source_ref_id -> Nullable<Int8>,
        chapter_number -> Nullable<Int4>,
        chunk_index -> Int4,
        content -> Text,
        embedding_profile_id -> Nullable<Int8>,
        embedding_json -> Nullable<Jsonb>,
        embedding_dim -> Nullable<Int4>,
        #[max_length = 32]
        embedding_status -> Varchar,
        metadata -> Jsonb,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    gm_novel_plot_arc_snapshots (id) {
        id -> Int8,
        project_id -> Text,
        chapter_number -> Nullable<Int4>,
        plot_arcs_text -> Text,
        version_no -> Int4,
        is_current -> Bool,
        source_stage_run_id -> Nullable<Int8>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    gm_novel_project_config_snapshots (id) {
        id -> Int8,
        project_id -> Text,
        architecture_llm_profile_id -> Nullable<Int8>,
        chapter_outline_llm_profile_id -> Nullable<Int8>,
        prompt_draft_llm_profile_id -> Nullable<Int8>,
        final_chapter_llm_profile_id -> Nullable<Int8>,
        consistency_review_llm_profile_id -> Nullable<Int8>,
        embedding_profile_id -> Nullable<Int8>,
        proxy_setting -> Jsonb,
        webdav_config -> Jsonb,
        other_params -> Jsonb,
        is_current -> Bool,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    gm_novel_projects (project_id) {
        project_id -> Text,
        user_id -> Int4,
        #[max_length = 255]
        title -> Varchar,
        topic -> Text,
        #[max_length = 100]
        genre -> Varchar,
        description -> Text,
        num_chapters -> Int4,
        target_words_per_chapter -> Int4,
        default_user_guidance -> Nullable<Text>,
        #[max_length = 32]
        status -> Varchar,
        #[max_length = 50]
        current_stage -> Nullable<Varchar>,
        #[max_length = 50]
        pending_stage -> Nullable<Varchar>,
        current_chapter_number -> Nullable<Int4>,
        progress_percent -> Float4,
        interaction_version -> Int4,
        last_error_message -> Nullable<Text>,
        metadata -> Jsonb,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        completed_at -> Nullable<Timestamptz>,
        deleted_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    gm_novel_stage_events (id) {
        id -> Int8,
        project_id -> Text,
        job_id -> Nullable<Int8>,
        stage_run_id -> Nullable<Int8>,
        chapter_number -> Nullable<Int4>,
        sequence -> Int8,
        #[max_length = 100]
        event_type -> Varchar,
        #[max_length = 50]
        stage_code -> Nullable<Varchar>,
        payload -> Jsonb,
        occurred_at -> Timestamptz,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    gm_novel_stage_runs (id) {
        id -> Int8,
        project_id -> Text,
        job_id -> Int8,
        chapter_number -> Int4,
        #[max_length = 50]
        stage_code -> Varchar,
        #[max_length = 32]
        status -> Varchar,
        #[max_length = 128]
        input_hash -> Varchar,
        input_payload -> Jsonb,
        output_payload -> Jsonb,
        error_message -> Nullable<Text>,
        attempt_no -> Int4,
        started_at -> Nullable<Timestamptz>,
        completed_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    gm_platforms (id) {
        id -> Int4,
        name -> Varchar,
        display_name -> Varchar,
        is_active -> Bool,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
        base_url -> Varchar,
        page_size -> Int4,
    }
}

diesel::table! {
    gm_pricing_rules (id) {
        id -> Int4,
        action_type -> Varchar,
        platform_id -> Nullable<Int4>,
        cost_points -> Numeric,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    gm_promo_codes (id) {
        id -> Int4,
        #[max_length = 36]
        code -> Varchar,
        points -> Int4,
        is_active -> Bool,
        expires_at -> Nullable<Timestamptz>,
        used_by_user_id -> Nullable<Int4>,
        used_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    gm_referral_earnings (id) {
        id -> Int4,
        referral_id -> Int4,
        transaction_id -> Nullable<Int4>,
        amount -> Numeric,
        description -> Nullable<Text>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    gm_referrals (id) {
        id -> Int4,
        referrer_id -> Int4,
        referee_id -> Int4,
        commission_rate -> Numeric,
        total_earned -> Numeric,
        #[max_length = 20]
        status -> Varchar,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    gm_regions (id) {
        id -> Int4,
        platform_id -> Int4,
        code -> Varchar,
        display_name -> Varchar,
        is_active -> Bool,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
        name -> Varchar,
    }
}

diesel::table! {
    gm_social_accounts (id) {
        id -> Int4,
        user_id -> Int4,
        platform_id -> Int4,
        username -> Varchar,
        proxy_url -> Nullable<Varchar>,
        status -> Varchar,
        health_score -> Nullable<Int4>,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
        group_id -> Nullable<Int4>,
        cookie -> Text,
        daily_max_replies -> Int4,
        #[max_length = 255]
        device_id -> Nullable<Varchar>,
        #[max_length = 255]
        profile_name -> Nullable<Varchar>,
    }
}

diesel::table! {
    gm_social_groups (id) {
        id -> Int4,
        user_id -> Int4,
        platform_id -> Int4,
        group_name -> Varchar,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    gm_upload_tasks (id) {
        id -> Int4,
        user_id -> Int4,
        social_account_id -> Int4,
        #[max_length = 50]
        task_type -> Varchar,
        metadata -> Jsonb,
        #[max_length = 20]
        status -> Varchar,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
        platform_id -> Nullable<Int4>,
    }
}

diesel::table! {
    gm_user_materials (id) {
        id -> Int4,
        user_id -> Int4,
        video_url -> Nullable<Text>,
        prompt -> Nullable<Text>,
        thumbnail_url -> Nullable<Text>,
        #[max_length = 100]
        tag -> Nullable<Varchar>,
        #[max_length = 255]
        title -> Nullable<Varchar>,
        description -> Nullable<Text>,
        duration -> Nullable<Int4>,
        file_size -> Nullable<Int8>,
        is_active -> Nullable<Bool>,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
        folder_id -> Nullable<Int4>,
        #[max_length = 20]
        media_type -> Varchar,
        #[max_length = 100]
        mime_type -> Nullable<Varchar>,
        #[max_length = 1024]
        file_url -> Nullable<Varchar>,
    }
}

diesel::table! {
    gm_user_notification_reads (id) {
        id -> Int4,
        user_id -> Int4,
        notification_id -> Int4,
        read_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    gm_user_wallets (user_id) {
        user_id -> Int4,
        balance_points -> Numeric,
        frozen_points -> Numeric,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
        deposit_cny -> Numeric,
        deposit_usd -> Numeric,
    }
}

diesel::table! {
    gm_users (id) {
        id -> Int4,
        #[max_length = 255]
        email -> Nullable<Varchar>,
        #[max_length = 255]
        password_hash -> Varchar,
        #[max_length = 50]
        invitation_code -> Nullable<Varchar>,
        #[max_length = 50]
        referred_by -> Nullable<Varchar>,
        #[max_length = 255]
        company_name -> Nullable<Varchar>,
        #[max_length = 255]
        api_key -> Nullable<Varchar>,
        #[max_length = 50]
        status -> Varchar,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
        full_name -> Varchar,
        role -> Varchar,
        is_active -> Bool,
        #[max_length = 36]
        invite_code -> Nullable<Varchar>,
        #[max_length = 36]
        invited_by -> Nullable<Varchar>,
        #[max_length = 50]
        username -> Nullable<Varchar>,
        permissions -> Int8,
    }
}

diesel::table! {
    gm_video_generation_tasks (id) {
        id -> Int4,
        user_id -> Int4,
        #[max_length = 255]
        task_id -> Varchar,
        #[max_length = 255]
        generation_id -> Nullable<Varchar>,
        prompt -> Nullable<Text>,
        #[max_length = 255]
        media_id -> Nullable<Varchar>,
        #[max_length = 50]
        status -> Varchar,
        progress_pct -> Nullable<Numeric>,
        video_width -> Nullable<Int4>,
        video_height -> Nullable<Int4>,
        video_url -> Nullable<Text>,
        thumbnail_url -> Nullable<Text>,
        #[max_length = 255]
        provider_post_id -> Nullable<Varchar>,
        provider_response -> Nullable<Jsonb>,
        cost_points -> Numeric,
        wallet_transaction_id -> Nullable<Int4>,
        error_message -> Nullable<Text>,
        retry_count -> Int4,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
        completed_at -> Nullable<Timestamptz>,
        model_id -> Nullable<Int4>,
        #[max_length = 255]
        title -> Nullable<Varchar>,
        #[max_length = 20]
        orientation -> Nullable<Varchar>,
        #[max_length = 10]
        video_seconds -> Nullable<Varchar>,
        #[max_length = 20]
        video_size -> Nullable<Varchar>,
    }
}

diesel::table! {
    gm_wallet_transactions (id) {
        id -> Int4,
        user_id -> Int4,
        amount -> Numeric,
        #[sql_name = "type"]
        type_ -> Varchar,
        payment_method -> Nullable<Varchar>,
        external_txn_id -> Nullable<Varchar>,
        reference_id -> Nullable<Int4>,
        description -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
        #[max_length = 50]
        reference_type -> Nullable<Varchar>,
    }
}

diesel::table! {
    hb_novel_provider_request_logs (id) {
        id -> Int8,
        project_id -> Text,
        job_id -> Nullable<Int8>,
        #[max_length = 50]
        stage_code -> Nullable<Varchar>,
        #[max_length = 20]
        provider_kind -> Varchar,
        #[max_length = 50]
        interface_format -> Varchar,
        #[max_length = 200]
        model_name -> Nullable<Varchar>,
        request_summary -> Jsonb,
        response_summary -> Jsonb,
        #[max_length = 32]
        status -> Varchar,
        latency_ms -> Nullable<Int4>,
        error_message -> Nullable<Text>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    hb_novel_vector_operations (id) {
        id -> Int8,
        project_id -> Text,
        #[max_length = 50]
        operation_type -> Varchar,
        knowledge_import_id -> Nullable<Int8>,
        chapter_number -> Nullable<Int4>,
        #[max_length = 32]
        status -> Varchar,
        detail -> Jsonb,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    hb_novel_worker_attempts (id) {
        id -> Int8,
        worker_task_id -> Text,
        attempt_no -> Int4,
        #[max_length = 100]
        worker_name -> Nullable<Varchar>,
        #[max_length = 100]
        host_name -> Nullable<Varchar>,
        #[max_length = 32]
        status -> Varchar,
        error_message -> Nullable<Text>,
        started_at -> Timestamptz,
        completed_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    hb_novel_worker_tasks (id) {
        id -> Text,
        project_id -> Text,
        job_id -> Nullable<Int8>,
        #[max_length = 50]
        stage_code -> Varchar,
        #[max_length = 50]
        task_type -> Varchar,
        chapter_number -> Nullable<Int4>,
        #[max_length = 32]
        status -> Varchar,
        payload -> Jsonb,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        started_at -> Nullable<Timestamptz>,
        completed_at -> Nullable<Timestamptz>,
    }
}

diesel::joinable!(drama_chapter_scene_asset_links -> gm_users (user_id));
diesel::joinable!(drama_private_characters -> gm_users (user_id));
diesel::joinable!(drama_private_scene_assets -> gm_users (user_id));
diesel::joinable!(drama_private_style_assets -> gm_users (user_id));
diesel::joinable!(drama_project_character_links -> gm_users (user_id));
diesel::joinable!(drama_project_meta -> gm_users (user_id));
diesel::joinable!(drama_project_scene_asset_links -> gm_users (user_id));
diesel::joinable!(drama_project_style_asset_links -> gm_users (user_id));
diesel::joinable!(gm_agent_comments -> gm_agent_videos (video_db_id));
diesel::joinable!(gm_agent_comments -> gm_campaigns (campaign_id));
diesel::joinable!(gm_agent_facebook_comments -> gm_agent_facebook_posts (post_db_id));
diesel::joinable!(gm_agent_facebook_comments -> gm_campaigns (campaign_id));
diesel::joinable!(gm_agent_facebook_posts -> gm_campaigns (campaign_id));
diesel::joinable!(gm_agent_facebook_posts -> gm_crawler_tasks (task_id));
diesel::joinable!(gm_agent_instagram_comments -> gm_agent_instagram_posts (post_db_id));
diesel::joinable!(gm_agent_instagram_comments -> gm_campaigns (campaign_id));
diesel::joinable!(gm_agent_instagram_posts -> gm_campaigns (campaign_id));
diesel::joinable!(gm_agent_instagram_posts -> gm_crawler_tasks (task_id));
diesel::joinable!(gm_agent_reddit_comments -> gm_agent_reddit_posts (post_db_id));
diesel::joinable!(gm_agent_reddit_comments -> gm_campaigns (campaign_id));
diesel::joinable!(gm_agent_reddit_posts -> gm_campaigns (campaign_id));
diesel::joinable!(gm_agent_reddit_posts -> gm_crawler_tasks (task_id));
diesel::joinable!(gm_agent_twitter_comments -> gm_agent_twitter_tweets (tweet_db_id));
diesel::joinable!(gm_agent_twitter_comments -> gm_campaigns (campaign_id));
diesel::joinable!(gm_agent_twitter_tweets -> gm_campaigns (campaign_id));
diesel::joinable!(gm_agent_twitter_tweets -> gm_crawler_tasks (task_id));
diesel::joinable!(gm_agent_videos -> gm_campaigns (campaign_id));
diesel::joinable!(gm_agent_videos -> gm_crawler_tasks (task_id));
diesel::joinable!(gm_aipub_ai_tasks -> gm_aipub_plans (plan_id));
diesel::joinable!(gm_aipub_plans -> gm_ai_models (image_ai_model_id));
diesel::joinable!(gm_aipub_plans -> gm_platforms (platform_id));
diesel::joinable!(gm_aipub_plans -> gm_social_accounts (social_account_id));
diesel::joinable!(gm_aipub_plans -> gm_social_groups (group_id));
diesel::joinable!(gm_aipub_plans -> gm_users (user_id));
diesel::joinable!(gm_aipub_tasks -> gm_aipub_ai_tasks (ai_task_id));
diesel::joinable!(gm_aipub_tasks -> gm_aipub_plans (plan_id));
diesel::joinable!(gm_aipub_tasks -> gm_social_accounts (social_account_id));
diesel::joinable!(gm_campaign_accounts -> gm_campaigns (campaign_id));
diesel::joinable!(gm_campaign_accounts -> gm_social_accounts (account_id));
diesel::joinable!(gm_campaign_templates -> gm_campaigns (campaign_id));
diesel::joinable!(gm_campaign_templates -> gm_reply_template_library (library_template_id));
diesel::joinable!(gm_campaigns -> gm_ai_models (ai_model_id));
diesel::joinable!(gm_campaigns -> gm_platforms (platform_id));
diesel::joinable!(gm_campaigns -> gm_regions (region_id));
diesel::joinable!(gm_campaigns -> gm_social_groups (social_group_id));
diesel::joinable!(gm_campaigns -> gm_users (user_id));
diesel::joinable!(gm_crawler_results -> gm_crawler_tasks (task_id));
diesel::joinable!(gm_crawler_tasks -> gm_campaigns (campaign_id));
diesel::joinable!(gm_login_logs -> gm_users (user_id));
diesel::joinable!(gm_novel_architecture_checkpoints -> gm_novel_projects (project_id));
diesel::joinable!(gm_novel_architecture_checkpoints -> gm_novel_stage_runs (source_stage_run_id));
diesel::joinable!(gm_novel_architectures -> gm_novel_projects (project_id));
diesel::joinable!(gm_novel_architectures -> gm_novel_stage_runs (source_stage_run_id));
diesel::joinable!(gm_novel_blueprint_chapters -> gm_novel_blueprints (blueprint_id));
diesel::joinable!(gm_novel_blueprint_chapters -> gm_novel_projects (project_id));
diesel::joinable!(gm_novel_blueprints -> gm_novel_projects (project_id));
diesel::joinable!(gm_novel_blueprints -> gm_novel_stage_runs (source_stage_run_id));
diesel::joinable!(gm_novel_chapter_prompts -> gm_novel_blueprint_chapters (blueprint_chapter_id));
diesel::joinable!(gm_novel_chapter_prompts -> gm_novel_projects (project_id));
diesel::joinable!(gm_novel_chapter_prompts -> gm_novel_stage_runs (source_stage_run_id));
diesel::joinable!(gm_novel_chapters -> gm_novel_blueprint_chapters (blueprint_chapter_id));
diesel::joinable!(gm_novel_chapters -> gm_novel_chapter_prompts (prompt_id));
diesel::joinable!(gm_novel_chapters -> gm_novel_projects (project_id));
diesel::joinable!(gm_novel_chapters -> gm_novel_stage_runs (source_stage_run_id));
diesel::joinable!(gm_novel_character_state_snapshots -> gm_novel_projects (project_id));
diesel::joinable!(gm_novel_character_state_snapshots -> gm_novel_stage_runs (source_stage_run_id));
diesel::joinable!(gm_novel_consistency_checks -> gm_novel_projects (project_id));
diesel::joinable!(gm_novel_consistency_checks -> gm_novel_stage_runs (source_stage_run_id));
diesel::joinable!(gm_novel_embedding_profiles -> gm_users (user_id));
diesel::joinable!(gm_novel_global_summary_snapshots -> gm_novel_projects (project_id));
diesel::joinable!(gm_novel_global_summary_snapshots -> gm_novel_stage_runs (source_stage_run_id));
diesel::joinable!(gm_novel_jobs -> gm_novel_projects (project_id));
diesel::joinable!(gm_novel_jobs -> gm_users (created_by));
diesel::joinable!(gm_novel_knowledge_chunks -> gm_novel_knowledge_imports (knowledge_import_id));
diesel::joinable!(gm_novel_knowledge_chunks -> gm_novel_projects (project_id));
diesel::joinable!(gm_novel_knowledge_imports -> gm_novel_projects (project_id));
diesel::joinable!(gm_novel_knowledge_imports -> gm_novel_stage_runs (source_stage_run_id));
diesel::joinable!(gm_novel_llm_profiles -> gm_users (user_id));
diesel::joinable!(gm_novel_memory_chunks -> gm_novel_embedding_profiles (embedding_profile_id));
diesel::joinable!(gm_novel_memory_chunks -> gm_novel_projects (project_id));
diesel::joinable!(gm_novel_plot_arc_snapshots -> gm_novel_projects (project_id));
diesel::joinable!(gm_novel_plot_arc_snapshots -> gm_novel_stage_runs (source_stage_run_id));
diesel::joinable!(gm_novel_project_config_snapshots -> gm_novel_embedding_profiles (embedding_profile_id));
diesel::joinable!(gm_novel_project_config_snapshots -> gm_novel_projects (project_id));
diesel::joinable!(gm_novel_projects -> gm_users (user_id));
diesel::joinable!(gm_novel_stage_events -> gm_novel_jobs (job_id));
diesel::joinable!(gm_novel_stage_events -> gm_novel_projects (project_id));
diesel::joinable!(gm_novel_stage_events -> gm_novel_stage_runs (stage_run_id));
diesel::joinable!(gm_novel_stage_runs -> gm_novel_jobs (job_id));
diesel::joinable!(gm_novel_stage_runs -> gm_novel_projects (project_id));
diesel::joinable!(gm_pricing_rules -> gm_platforms (platform_id));
diesel::joinable!(gm_referral_earnings -> gm_referrals (referral_id));
diesel::joinable!(gm_regions -> gm_platforms (platform_id));
diesel::joinable!(gm_social_accounts -> gm_platforms (platform_id));
diesel::joinable!(gm_social_accounts -> gm_social_groups (group_id));
diesel::joinable!(gm_social_accounts -> gm_users (user_id));
diesel::joinable!(gm_social_groups -> gm_platforms (platform_id));
diesel::joinable!(gm_social_groups -> gm_users (user_id));
diesel::joinable!(gm_upload_tasks -> gm_platforms (platform_id));
diesel::joinable!(gm_upload_tasks -> gm_social_accounts (social_account_id));
diesel::joinable!(gm_upload_tasks -> gm_users (user_id));
diesel::joinable!(gm_user_materials -> gm_material_folders (folder_id));
diesel::joinable!(gm_user_notification_reads -> gm_notifications (notification_id));
diesel::joinable!(gm_user_notification_reads -> gm_users (user_id));
diesel::joinable!(gm_user_wallets -> gm_users (user_id));
diesel::joinable!(gm_video_generation_tasks -> gm_ai_models (model_id));
diesel::joinable!(gm_video_generation_tasks -> gm_users (user_id));
diesel::joinable!(gm_video_generation_tasks -> gm_wallet_transactions (wallet_transaction_id));
diesel::joinable!(gm_wallet_transactions -> gm_users (user_id));
diesel::joinable!(hb_novel_provider_request_logs -> gm_novel_jobs (job_id));
diesel::joinable!(hb_novel_vector_operations -> gm_novel_knowledge_imports (knowledge_import_id));
diesel::joinable!(hb_novel_worker_attempts -> hb_novel_worker_tasks (worker_task_id));
diesel::joinable!(hb_novel_worker_tasks -> gm_novel_jobs (job_id));

diesel::allow_tables_to_appear_in_same_query!(
    drama_chapter_scene_asset_links,
    drama_private_characters,
    drama_private_scene_assets,
    drama_private_style_assets,
    drama_project_character_links,
    drama_project_meta,
    drama_project_scene_asset_links,
    drama_project_style_asset_links,
    gm_admin_users,
    gm_agent_comments,
    gm_agent_facebook_comments,
    gm_agent_facebook_posts,
    gm_agent_instagram_comments,
    gm_agent_instagram_posts,
    gm_agent_reddit_comments,
    gm_agent_reddit_posts,
    gm_agent_twitter_comments,
    gm_agent_twitter_tweets,
    gm_agent_videos,
    gm_ai_models,
    gm_ai_video_models,
    gm_aipub_ai_tasks,
    gm_aipub_plans,
    gm_aipub_tasks,
    gm_campaign_accounts,
    gm_campaign_templates,
    gm_campaigns,
    gm_crawler_results,
    gm_crawler_tasks,
    gm_drama_callback_events,
    gm_drama_cost_events,
    gm_drama_project_projections,
    gm_email_verifications,
    gm_login_logs,
    gm_material_folders,
    gm_notifications,
    gm_novel_architecture_checkpoints,
    gm_novel_architectures,
    gm_novel_blueprint_chapters,
    gm_novel_blueprints,
    gm_novel_chapter_prompts,
    gm_novel_chapters,
    gm_novel_character_state_snapshots,
    gm_novel_consistency_checks,
    gm_novel_embedding_profiles,
    gm_novel_global_summary_snapshots,
    gm_novel_jobs,
    gm_novel_knowledge_chunks,
    gm_novel_knowledge_imports,
    gm_novel_llm_profiles,
    gm_novel_memory_chunks,
    gm_novel_plot_arc_snapshots,
    gm_novel_project_config_snapshots,
    gm_novel_projects,
    gm_novel_stage_events,
    gm_novel_stage_runs,
    gm_platforms,
    gm_pricing_rules,
    gm_promo_codes,
    gm_referral_earnings,
    gm_referrals,
    gm_regions,
    gm_reply_template_library,
    gm_social_accounts,
    gm_social_groups,
    gm_upload_tasks,
    gm_user_materials,
    gm_user_notification_reads,
    gm_user_wallets,
    gm_users,
    gm_video_generation_tasks,
    gm_wallet_transactions,
    hb_novel_provider_request_logs,
    hb_novel_vector_operations,
    hb_novel_worker_attempts,
    hb_novel_worker_tasks,
);
