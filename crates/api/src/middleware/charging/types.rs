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
            "VIDEO_GENERATE" | "VIDEO" => Ok(ActionType::VideoGenerate),
            "SCAN_POST" => Ok(ActionType::ScanPost),
            _ => Err(format!("Unknown action type: {}", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_str_round_trips_through_from_str() {
        for action in [ActionType::AiAnalyze, ActionType::VideoGenerate, ActionType::ScanPost] {
            let parsed: ActionType = action.as_str().parse().unwrap();
            assert_eq!(parsed, action);
        }
    }

    #[test]
    fn from_str_is_case_insensitive() {
        assert_eq!("ai_analyze".parse::<ActionType>().unwrap(), ActionType::AiAnalyze);
        assert_eq!("Scan_Post".parse::<ActionType>().unwrap(), ActionType::ScanPost);
        assert_eq!("video_generate".parse::<ActionType>().unwrap(), ActionType::VideoGenerate);
    }

    #[test]
    fn legacy_video_alias_maps_to_video_generate() {
        assert_eq!("VIDEO".parse::<ActionType>().unwrap(), ActionType::VideoGenerate);
        assert_eq!("video".parse::<ActionType>().unwrap(), ActionType::VideoGenerate);
    }

    #[test]
    fn unknown_action_type_returns_error() {
        assert!("UNKNOWN".parse::<ActionType>().is_err());
        assert!("".parse::<ActionType>().is_err());
    }

    #[test]
    fn display_matches_as_str() {
        for action in [ActionType::AiAnalyze, ActionType::VideoGenerate, ActionType::ScanPost] {
            assert_eq!(format!("{}", action), action.as_str());
        }
    }
}
