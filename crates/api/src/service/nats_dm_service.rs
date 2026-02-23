use crate::dto::dm_dto::*;
use crate::error::api_error::ApiError;
use async_nats::jetstream::{self, kv, stream};
use async_nats::Client as NatsClient;
use chrono::Utc;
use futures::StreamExt;
use serde::Serialize;
use std::collections::HashMap;
use uuid::Uuid;

/// Command payload published to `dm.cmd.{device_id}` for executor consumption.
#[derive(Debug, Serialize)]
struct DmReplyCommand<'a> {
    cmd_id: String,
    conv_id: &'a str,
    social_account_id: i32,
    platform_id: i32,
    profile_name: &'a str,
    remote_username: &'a str,
    content: &'a str,
    content_type: &'a str,
    timestamp: String,
}

/// NATS JetStream DM Service
///
/// Reads/writes DM data from NATS Streams and KV Buckets.
/// Does NOT touch PostgreSQL except for `send_reply` (to look up device_id).
#[derive(Clone)]
pub struct NatsDmService {
    js: jetstream::Context,
    /// Retained to keep the NATS connection alive; `js` borrows it internally.
    #[allow(dead_code)]
    client: NatsClient,
}

impl NatsDmService {
    pub fn new(client: NatsClient) -> Self {
        let js = jetstream::new(client.clone());
        Self { js, client }
    }

    /// Initialize NATS Streams and KV Buckets (idempotent).
    pub async fn init_infrastructure(&self) -> Result<(), ApiError> {
        // DM_MESSAGES stream
        self.js
            .get_or_create_stream(jetstream::stream::Config {
                name: "DM_MESSAGES".to_string(),
                subjects: vec!["dm.msg.>".to_string()],
                retention: stream::RetentionPolicy::Limits,
                max_age: std::time::Duration::from_secs(30 * 24 * 3600),
                storage: stream::StorageType::File,
                duplicate_window: std::time::Duration::from_secs(600),
                ..Default::default()
            })
            .await
            .map_err(|e| ApiError::InternalServerError(format!("DM_MESSAGES init: {e}")))?;

        // DM_COMMANDS stream (WorkQueue)
        self.js
            .get_or_create_stream(jetstream::stream::Config {
                name: "DM_COMMANDS".to_string(),
                subjects: vec!["dm.cmd.>".to_string()],
                retention: stream::RetentionPolicy::WorkQueue,
                max_age: std::time::Duration::from_secs(24 * 3600),
                storage: stream::StorageType::File,
                ..Default::default()
            })
            .await
            .map_err(|e| ApiError::InternalServerError(format!("DM_COMMANDS init: {e}")))?;

        // DM_EVENTS stream
        self.js
            .get_or_create_stream(jetstream::stream::Config {
                name: "DM_EVENTS".to_string(),
                subjects: vec!["dm.evt.>".to_string()],
                retention: stream::RetentionPolicy::Limits,
                max_age: std::time::Duration::from_secs(3600),
                storage: stream::StorageType::Memory,
                ..Default::default()
            })
            .await
            .map_err(|e| ApiError::InternalServerError(format!("DM_EVENTS init: {e}")))?;

        // KV buckets (ignore AlreadyExists, log other errors)
        for (bucket, storage, max_age) in [
            ("dm_conversations", stream::StorageType::File, None),
            ("dm_device_heartbeat", stream::StorageType::Memory, Some(std::time::Duration::from_secs(120))),
            ("dm_monitor_config", stream::StorageType::File, None),
        ] {
            let cfg = kv::Config {
                bucket: bucket.to_string(),
                storage,
                max_age: max_age.unwrap_or_default(),
                ..Default::default()
            };
            if let Err(e) = self.js.create_key_value(cfg).await {
                let msg = e.to_string();
                if !msg.contains("already") {
                    tracing::warn!("KV bucket {bucket} create: {e}");
                }
            }
        }

        tracing::info!("NATS DM infrastructure initialized (3 streams + 3 KV buckets)");
        Ok(())
    }

    // =========================================================================
    // Conversations (KV: dm_conversations)
    // =========================================================================

    pub async fn list_conversations(
        &self,
        user_id: i32,
        query: DmConversationsQuery,
    ) -> Result<DmConversationsResponse, ApiError> {
        let kv = self.js.get_key_value("dm_conversations").await
            .map_err(|e| ApiError::InternalServerError(format!("KV open: {e}")))?;

        let prefix = format!("{user_id}.");

        // Collect all keys (Keys implements futures::Stream)
        let keys_stream = kv.keys().await
            .map_err(|e| ApiError::InternalServerError(format!("KV keys: {e}")))?;
        let all_keys: Vec<String> = keys_stream
            .filter_map(|r| async { r.ok() })
            .collect()
            .await;

        tracing::debug!(
            "DM list_conversations: user_id={user_id}, prefix={prefix}, all_keys_count={}",
            all_keys.len()
        );

        let mut conversations = Vec::new();

        for key in &all_keys {
            if !key.starts_with(&prefix) {
                continue;
            }
            // kv.get() returns Result<Option<Bytes>>
            match kv.get(key).await {
                Ok(Some(bytes)) => {
                    match serde_json::from_slice::<ConversationMetaDto>(&bytes) {
                        Ok(conv) => {
                            // Apply filters
                            if let Some(pid) = query.platform_id {
                                if conv.platform_id != pid {
                                    continue;
                                }
                            }
                            if let Some(ref did) = query.device_id {
                                if &conv.device_id != did {
                                    continue;
                                }
                            }
                            if let Some(aid) = query.account_id {
                                if conv.social_account_id != aid {
                                    continue;
                                }
                            }
                            conversations.push(conv);
                        }
                        Err(e) => {
                            tracing::warn!("DM KV deserialize error for key={key}: {e}");
                        }
                    }
                }
                Ok(None) => {
                    tracing::debug!("DM KV key={key} not found (deleted?)");
                }
                Err(e) => {
                    tracing::warn!("DM KV get error for key={key}: {e}");
                }
            }
        }

        conversations.sort_by(|a, b| b.last_message_at.cmp(&a.last_message_at));

        let device_status = self.get_device_status_map(&conversations).await;

        Ok(DmConversationsResponse { conversations, device_status })
    }

    /// Fetch messages for a conversation, returning the **latest** `limit` messages.
    ///
    /// Pagination: pass `before_seq` (NATS stream sequence) to load older messages.
    /// Returns `has_more = true` when there are still older messages beyond the
    /// returned window.
    pub async fn get_messages(
        &self,
        conv_id: &str,
        query: DmMessagesQuery,
    ) -> Result<DmMessagesResponse, ApiError> {
        let stream = self.js.get_stream("DM_MESSAGES").await
            .map_err(|e| ApiError::InternalServerError(format!("Stream open: {e}")))?;

        let limit = query.limit.unwrap_or(50).min(100);
        let subject = format!("dm.msg.{conv_id}");

        tracing::debug!(
            "DM get_messages: conv_id={conv_id}, limit={limit}, before_seq={:?}",
            query.before_seq
        );

        // Create an ephemeral consumer filtered to this conversation's subject.
        // DeliverPolicy::All starts from the first message so we get the full history.
        let consumer = stream
            .create_consumer(jetstream::consumer::pull::Config {
                filter_subject: subject.clone(),
                deliver_policy: jetstream::consumer::DeliverPolicy::All,
                ..Default::default()
            })
            .await
            .map_err(|e| ApiError::InternalServerError(format!("Consumer create: {e}")))?;

        // Fetch ALL messages for this conversation in batches.
        // A single fetch(500) may miss newer messages when conversations have 500+ messages.
        // Loop until no more messages are returned.
        let fetch_batch_size: usize = 1000;
        let max_total: usize = 10000; // safety limit
        let mut all_messages: Vec<DmMessageDto> = Vec::new();
        let mut deser_errors = 0u32;

        loop {
            let mut messages_batch = consumer
                .fetch()
                .max_messages(fetch_batch_size)
                .expires(std::time::Duration::from_secs(3))
                .messages()
                .await
                .map_err(|e| ApiError::InternalServerError(format!("Fetch: {e}")))?;

            let mut batch_count: usize = 0;
            while let Some(result) = messages_batch.next().await {
                match result {
                    Ok(msg) => {
                        match serde_json::from_slice::<DmMessageDto>(&msg.payload) {
                            Ok(mut dm_msg) => {
                                if let Ok(info) = msg.info() {
                                    dm_msg.nats_seq = Some(info.stream_sequence);
                                }
                                all_messages.push(dm_msg);
                                batch_count += 1;
                            }
                            Err(e) => {
                                deser_errors += 1;
                                if deser_errors <= 3 {
                                    tracing::warn!(
                                        "DM get_messages: deserialize error for subject={subject}: {e}"
                                    );
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("DM get_messages: stream error: {e}");
                        continue;
                    }
                }
            }

            // If we got fewer than batch size, we've consumed all messages
            if batch_count < fetch_batch_size || all_messages.len() >= max_total {
                break;
            }
        }

        if all_messages.len() >= max_total {
            tracing::warn!(
                "DM get_messages: hit safety limit ({max_total}) for conv_id={conv_id}, some old messages may be missing"
            );
        }

        tracing::debug!(
            "DM get_messages: conv_id={conv_id}, fetched {} messages (deser_errors={})",
            all_messages.len(),
            deser_errors
        );

        // Sort by nats_seq (most reliable) then timestamp
        all_messages.sort_by(|a, b| {
            match (a.nats_seq, b.nats_seq) {
                (Some(sa), Some(sb)) => sa.cmp(&sb),
                _ => a.timestamp.cmp(&b.timestamp),
            }
        });

        // Apply `before_seq` filter: keep only messages whose nats_seq < before_seq
        if let Some(before_seq) = query.before_seq {
            all_messages.retain(|m| m.nats_seq.map_or(true, |s| s < before_seq));
        }

        // Take the last `limit` messages (newest) and report has_more
        let has_more = all_messages.len() > limit;
        let messages = if all_messages.len() > limit {
            all_messages.split_off(all_messages.len() - limit)
        } else {
            all_messages
        };

        Ok(DmMessagesResponse { messages, has_more })
    }

    pub async fn send_reply(
        &self,
        conv_id: &str,
        device_id: &str,
        social_account_id: i32,
        platform_id: i32,
        profile_name: &str,
        remote_username: &str,
        content: &str,
        content_type: &str,
    ) -> Result<DmReplyResponse, ApiError> {
        let cmd_id = Uuid::new_v4().to_string();
        let cmd = DmReplyCommand {
            cmd_id: cmd_id.clone(),
            conv_id,
            social_account_id,
            platform_id,
            profile_name,
            remote_username,
            content,
            content_type,
            timestamp: Utc::now().to_rfc3339(),
        };

        let payload = serde_json::to_vec(&cmd)
            .map_err(|e| ApiError::InternalServerError(format!("Serialize cmd: {e}")))?;

        let subject = format!("dm.cmd.{device_id}");
        self.js
            .publish(subject, payload.into())
            .await
            .map_err(|e| ApiError::InternalServerError(format!("Publish cmd: {e}")))?
            .await
            .map_err(|e| ApiError::InternalServerError(format!("Publish ack: {e}")))?;

        Ok(DmReplyResponse { cmd_id, status: "queued".to_string() })
    }

    pub async fn mark_read(&self, user_id: i32, conv_id: &str) -> Result<(), ApiError> {
        let kv = self.js.get_key_value("dm_conversations").await
            .map_err(|e| ApiError::InternalServerError(format!("KV open: {e}")))?;

        let key = format!("{user_id}.{conv_id}");
        if let Ok(Some(bytes)) = kv.get(&key).await {
            if let Ok(mut conv) = serde_json::from_slice::<ConversationMetaDto>(&bytes) {
                conv.unread_count = 0;
                conv.updated_at = Utc::now().to_rfc3339();
                let data = serde_json::to_vec(&conv)
                    .map_err(|e| ApiError::InternalServerError(format!("Serialize: {e}")))?;
                kv.put(&key, data.into()).await
                    .map_err(|e| ApiError::InternalServerError(format!("KV put: {e}")))?;
            }
        }
        Ok(())
    }

    pub async fn update_settings(
        &self,
        user_id: i32,
        conv_id: &str,
        settings: DmSettingsRequest,
    ) -> Result<(), ApiError> {
        let kv = self.js.get_key_value("dm_conversations").await
            .map_err(|e| ApiError::InternalServerError(format!("KV open: {e}")))?;

        let key = format!("{user_id}.{conv_id}");
        if let Ok(Some(bytes)) = kv.get(&key).await {
            if let Ok(mut conv) = serde_json::from_slice::<ConversationMetaDto>(&bytes) {
                if let Some(reply_mode) = settings.reply_mode {
                    conv.reply_mode = reply_mode;
                }
                if let Some(status) = settings.status {
                    conv.status = status;
                }
                conv.updated_at = Utc::now().to_rfc3339();
                let data = serde_json::to_vec(&conv)
                    .map_err(|e| ApiError::InternalServerError(format!("Serialize: {e}")))?;
                kv.put(&key, data.into()).await
                    .map_err(|e| ApiError::InternalServerError(format!("KV put: {e}")))?;
            }
        }
        Ok(())
    }

    pub async fn get_stats(&self, user_id: i32) -> Result<DmStatsResponse, ApiError> {
        let kv = self.js.get_key_value("dm_conversations").await
            .map_err(|e| ApiError::InternalServerError(format!("KV open: {e}")))?;

        let prefix = format!("{user_id}.");
        let keys_stream = kv.keys().await
            .map_err(|e| ApiError::InternalServerError(format!("KV keys: {e}")))?;
        let all_keys: Vec<String> = keys_stream
            .filter_map(|r| async { r.ok() })
            .collect()
            .await;

        let mut total_unread = 0i32;
        let mut platform_map: HashMap<(i32, String), (usize, i32)> = HashMap::new();

        for key in &all_keys {
            if !key.starts_with(&prefix) { continue; }
            if let Ok(Some(bytes)) = kv.get(key).await {
                match serde_json::from_slice::<ConversationMetaDto>(&bytes) {
                    Ok(conv) => {
                        total_unread += conv.unread_count;
                        let entry = platform_map
                            .entry((conv.platform_id, conv.platform_name.clone()))
                            .or_insert((0, 0));
                        entry.0 += 1;
                        entry.1 += conv.unread_count;
                    }
                    Err(e) => {
                        tracing::warn!("DM stats: deserialize error for key={key}: {e}");
                    }
                }
            }
        }

        let total_conversations = platform_map.values().map(|(c, _)| c).sum();
        let per_platform = platform_map
            .into_iter()
            .map(|((pid, pname), (count, unread))| DmPlatformStats {
                platform_id: pid, platform_name: pname,
                conversations: count, unread,
            })
            .collect();

        Ok(DmStatsResponse { total_conversations, total_unread, per_platform })
    }

    pub async fn generate_nats_token(
        &self,
        _user_id: i32,
    ) -> Result<DmNatsTokenResponse, ApiError> {
        // TODO: implement per-user NATS JWT for fine-grained access control.
        // Currently returns the shared server token. Each user gets identical access.
        let token = std::env::var("NATS_TOKEN").map_err(|_| {
            ApiError::InternalServerError("NATS_TOKEN not configured".into())
        })?;
        let expires_at = (Utc::now() + chrono::Duration::hours(1)).to_rfc3339();
        Ok(DmNatsTokenResponse { token, expires_at })
    }

    // =========================================================================
    // Monitor Config (KV: dm_monitor_config)
    // =========================================================================

    pub async fn get_monitor_config(
        &self,
        device_id: &str,
    ) -> Result<Option<DmMonitorConfigDto>, ApiError> {
        let kv = self.js.get_key_value("dm_monitor_config").await
            .map_err(|e| ApiError::InternalServerError(format!("KV open: {e}")))?;
        match kv.get(device_id).await {
            Ok(Some(bytes)) => {
                let cfg = serde_json::from_slice::<DmMonitorConfigDto>(&bytes)
                    .map_err(|e| ApiError::InternalServerError(format!("Deserialize: {e}")))?;
                Ok(Some(cfg))
            }
            Ok(None) => Ok(None),
            Err(e) => {
                tracing::warn!("DM monitor config get error for {device_id}: {e}");
                Ok(None)
            }
        }
    }

    pub async fn update_monitor_config(
        &self,
        device_id: &str,
        req: DmMonitorConfigUpdateRequest,
    ) -> Result<DmMonitorConfigDto, ApiError> {
        let kv = self.js.get_key_value("dm_monitor_config").await
            .map_err(|e| ApiError::InternalServerError(format!("KV open: {e}")))?;

        let mut cfg = match kv.get(device_id).await {
            Ok(Some(bytes)) => serde_json::from_slice::<DmMonitorConfigDto>(&bytes)
                .unwrap_or_else(|_| DmMonitorConfigDto {
                    device_id: device_id.to_string(),
                    ..Default::default()
                }),
            _ => DmMonitorConfigDto {
                device_id: device_id.to_string(),
                ..Default::default()
            },
        };

        if let Some(v) = req.enabled { cfg.enabled = v; }
        if let Some(v) = req.poll_interval_seconds { cfg.poll_interval_seconds = v; }
        if let Some(v) = req.max_concurrent_monitors { cfg.max_concurrent_monitors = v; }
        if let Some(v) = req.inbox_linger_seconds { cfg.inbox_linger_seconds = v; }
        if let Some(v) = req.platforms { cfg.platforms = v; }
        cfg.updated_at = Utc::now().to_rfc3339();

        let data = serde_json::to_vec(&cfg)
            .map_err(|e| ApiError::InternalServerError(format!("Serialize: {e}")))?;
        kv.put(device_id, data.into()).await
            .map_err(|e| ApiError::InternalServerError(format!("KV put: {e}")))?;

        Ok(cfg)
    }

    // =========================================================================
    // Internal helpers
    // =========================================================================

    async fn get_device_status_map(
        &self,
        conversations: &[ConversationMetaDto],
    ) -> HashMap<String, DeviceStatusDto> {
        let mut result = HashMap::new();

        let device_ids: Vec<String> = conversations
            .iter()
            .map(|c| c.device_id.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        if let Ok(kv) = self.js.get_key_value("dm_device_heartbeat").await {
            for device_id in device_ids {
                match kv.get(&device_id).await {
                    Ok(Some(bytes)) => {
                        if let Ok(hb) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                            let last_seen = hb.get("last_seen")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());
                            result.insert(device_id, DeviceStatusDto { online: true, last_seen });
                        }
                    }
                    _ => {
                        result.insert(device_id, DeviceStatusDto { online: false, last_seen: None });
                    }
                }
            }
        }

        result
    }
}
