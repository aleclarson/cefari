use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum CefariTarget {
    Desktop,
    Ios,
    Android,
}

impl CefariTarget {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Desktop => "desktop",
            Self::Ios => "ios",
            Self::Android => "android",
        }
    }
}

impl fmt::Display for CefariTarget {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for CefariTarget {
    type Err = UnknownCefariTarget;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "desktop" => Ok(Self::Desktop),
            "ios" => Ok(Self::Ios),
            "android" => Ok(Self::Android),
            _ => Err(UnknownCefariTarget {
                value: value.to_owned(),
            }),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct UnknownCefariTarget {
    value: String,
}

impl fmt::Display for UnknownCefariTarget {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "unknown Cefari target `{}`", self.value)
    }
}

impl std::error::Error for UnknownCefariTarget {}

#[derive(Debug, Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum PlatformSupport {
    Portable,
    HostSpecific,
    DesktopOnly,
    MobileOnly,
    Deferred,
}

impl PlatformSupport {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Portable => "portable",
            Self::HostSpecific => "hostSpecific",
            Self::DesktopOnly => "desktopOnly",
            Self::MobileOnly => "mobileOnly",
            Self::Deferred => "deferred",
        }
    }
}

impl fmt::Display for PlatformSupport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for PlatformSupport {
    type Err = UnknownPlatformSupport;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "portable" => Ok(Self::Portable),
            "hostSpecific" => Ok(Self::HostSpecific),
            "desktopOnly" => Ok(Self::DesktopOnly),
            "mobileOnly" => Ok(Self::MobileOnly),
            "deferred" => Ok(Self::Deferred),
            _ => Err(UnknownPlatformSupport {
                value: value.to_owned(),
            }),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct UnknownPlatformSupport {
    value: String,
}

impl fmt::Display for UnknownPlatformSupport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "unknown platform support `{}`", self.value)
    }
}

impl std::error::Error for UnknownPlatformSupport {}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::{CefariTarget, PlatformSupport};

    #[test]
    fn parses_target_names() {
        assert_eq!(CefariTarget::from_str("desktop"), Ok(CefariTarget::Desktop));
        assert_eq!(CefariTarget::from_str("ios"), Ok(CefariTarget::Ios));
        assert_eq!(CefariTarget::from_str("android"), Ok(CefariTarget::Android));
        assert!(CefariTarget::from_str("web").is_err());
    }

    #[test]
    fn serializes_target_names() {
        assert_eq!(
            serde_json::to_string(&CefariTarget::Desktop).expect("target should serialize"),
            r#""desktop""#
        );
        assert_eq!(
            serde_json::from_str::<CefariTarget>(r#""ios""#).expect("target should deserialize"),
            CefariTarget::Ios
        );
    }

    #[test]
    fn parses_platform_support_names() {
        assert_eq!(
            PlatformSupport::from_str("portable"),
            Ok(PlatformSupport::Portable)
        );
        assert_eq!(
            PlatformSupport::from_str("hostSpecific"),
            Ok(PlatformSupport::HostSpecific)
        );
        assert_eq!(
            PlatformSupport::from_str("desktopOnly"),
            Ok(PlatformSupport::DesktopOnly)
        );
        assert_eq!(
            PlatformSupport::from_str("mobileOnly"),
            Ok(PlatformSupport::MobileOnly)
        );
        assert_eq!(
            PlatformSupport::from_str("deferred"),
            Ok(PlatformSupport::Deferred)
        );
        assert!(PlatformSupport::from_str("desktop").is_err());
    }

    #[test]
    fn serializes_platform_support_names() {
        assert_eq!(
            serde_json::to_string(&PlatformSupport::HostSpecific)
                .expect("support should serialize"),
            r#""hostSpecific""#
        );
        assert_eq!(
            serde_json::from_str::<PlatformSupport>(r#""desktopOnly""#)
                .expect("support should deserialize"),
            PlatformSupport::DesktopOnly
        );
    }
}
