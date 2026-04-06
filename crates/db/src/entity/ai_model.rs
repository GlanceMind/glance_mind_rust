use crate::schema::gm_ai_models;
use bigdecimal::BigDecimal;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Selectable, Identifiable, Serialize, Deserialize, Debug, Clone)]
#[diesel(table_name = gm_ai_models)]
pub struct AiModel {
    pub id: i32,
    pub name: String,
    pub provider: String,
    pub model_key: String,
    pub cost_multiplier: BigDecimal,
    pub is_active: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
    pub model_type: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelType {
    Chat,
    Video,
    Image,
}

#[allow(dead_code)]
impl ModelType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ModelType::Chat => "chat",
            ModelType::Video => "video",
            ModelType::Image => "image",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "chat" => Some(ModelType::Chat),
            "video" => Some(ModelType::Video),
            "image" => Some(ModelType::Image),
            _ => None,
        }
    }
}

impl std::fmt::Display for ModelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Insertable, AsChangeset)]
#[diesel(table_name = gm_ai_models)]
pub struct NewAiModel {
    pub name: String,
    pub provider: String,
    pub model_key: String,
    pub cost_multiplier: BigDecimal,
    pub is_active: Option<bool>,
    pub model_type: String,
}
