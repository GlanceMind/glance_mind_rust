//! Entity definitions for GlanceMind database tables.
//!
//! This module contains all Diesel ORM entity definitions that map to database tables.

pub mod agent;
pub mod ai_chat;
pub mod ai_model;
pub mod aipub;
pub mod campaign;
pub mod crawler;
pub mod dm_auto_reply;
pub mod email_verification;
pub mod login_log;
pub mod material;
pub mod material_folder;
pub mod notification;
pub mod novel;
pub mod oauth;
pub mod patrol;
pub mod platform;
pub mod pricing_rule;
pub mod promo_code;
pub mod referral;
pub mod region;
pub mod social_account;
pub mod social_group;
pub mod template;
pub mod upload_task;
pub mod user;
pub mod user_wallet;
pub mod video;
pub mod video_case;
pub mod wallet_transaction;

// Re-export all entities for convenience
pub use agent::*;
pub use ai_chat::*;
pub use ai_model::*;
pub use aipub::*;
pub use campaign::*;
pub use crawler::*;
pub use dm_auto_reply::*;
pub use email_verification::*;
pub use login_log::*;
pub use material::*;
pub use material_folder::*;
pub use notification::*;
pub use novel::*;
pub use oauth::*;
pub use patrol::*;
pub use platform::*;
pub use pricing_rule::*;
pub use promo_code::*;
pub use referral::*;
pub use region::*;
pub use social_account::*;
pub use social_group::*;
pub use template::*;
pub use upload_task::*;
pub use user::*;
pub use user_wallet::*;
pub use video::*;
pub use video_case::*;
pub use wallet_transaction::*;
