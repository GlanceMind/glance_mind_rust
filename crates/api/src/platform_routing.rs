pub const PLATFORM_NAME_REDDIT: &str = "REDDIT";
pub const PLATFORM_NAME_TIKTOK: &str = "TIKTOK";
pub const PLATFORM_NAME_FACEBOOK: &str = "FACEBOOK";
pub const PLATFORM_NAME_INSTAGRAM: &str = "INSTAGRAM";
pub const PLATFORM_NAME_TWITTER: &str = "TWITTER";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportedPlatform {
    Reddit,
    Tiktok,
    Facebook,
    Instagram,
    Twitter,
}

impl SupportedPlatform {
    pub fn from_name(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "reddit" => Some(Self::Reddit),
            "tiktok" => Some(Self::Tiktok),
            "facebook" => Some(Self::Facebook),
            "instagram" => Some(Self::Instagram),
            "twitter" => Some(Self::Twitter),
            _ => None,
        }
    }

    pub fn as_db_name(self) -> &'static str {
        match self {
            Self::Reddit => PLATFORM_NAME_REDDIT,
            Self::Tiktok => PLATFORM_NAME_TIKTOK,
            Self::Facebook => PLATFORM_NAME_FACEBOOK,
            Self::Instagram => PLATFORM_NAME_INSTAGRAM,
            Self::Twitter => PLATFORM_NAME_TWITTER,
        }
    }

    pub fn as_api_name(self) -> &'static str {
        match self {
            Self::Reddit => "reddit",
            Self::Tiktok => "tiktok",
            Self::Facebook => "facebook",
            Self::Instagram => "instagram",
            Self::Twitter => "twitter",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SupportedPlatform;

    #[test]
    fn supported_platform_parsing_is_case_insensitive() {
        assert_eq!(
            SupportedPlatform::from_name("TIKTOK"),
            Some(SupportedPlatform::Tiktok)
        );
        assert_eq!(
            SupportedPlatform::from_name("facebook"),
            Some(SupportedPlatform::Facebook)
        );
        assert_eq!(
            SupportedPlatform::from_name(" Instagram "),
            Some(SupportedPlatform::Instagram)
        );
    }

    #[test]
    fn unsupported_platform_returns_none() {
        assert_eq!(SupportedPlatform::from_name("youtube"), None);
        assert_eq!(SupportedPlatform::from_name(""), None);
    }
}
