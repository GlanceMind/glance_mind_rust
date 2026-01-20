use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionType {
    AiAnalyze,     // AI analysis
    VideoGenerate, // Video generation
    ScanPost,      // Scan post
}

impl ActionType {
    /// Convert to database format string (uppercase + underscore)
    pub fn as_str(&self) -> &'static str {
        match self {
            ActionType::AiAnalyze => "AI_ANALYZE",
            ActionType::VideoGenerate => "VIDEO_GENERATE",
            ActionType::ScanPost => "SCAN_POST",
        }
    }
}

/// Implement Display trait for convenient string conversion
impl fmt::Display for ActionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Implement FromStr trait for parsing from database string
impl FromStr for ActionType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "AI_ANALYZE" => Ok(ActionType::AiAnalyze),
            "VIDEO_GENERATE" => Ok(ActionType::VideoGenerate),
            "SCAN_POST" => Ok(ActionType::ScanPost),
            _ => Err(format!("Unknown action type: {}", s)),
        }
    }
}
