use super::*;

/// Returns the built-in macOS Chrome desktop profile.
pub fn chrome_macos() -> BrowserProfile {
    BrowserProfile {
        name: "chrome-macos".to_string(),
        navigator: NavigatorProfile {
            // Chrome freezes the macOS token in the reduced user-agent string.
            user_agent: format!(
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{CHROME_VERSION} Safari/537.36"
            ),
            platform: "MacIntel".to_string(),
            languages: vec!["en-US".to_string(), "en".to_string()],
            client_hints: Some(UserAgentClientHintsProfile {
                brands: chrome_brands(false),
                full_version_list: chrome_brands(true),
                platform: "macOS".to_string(),
                platform_version: "15.6.1".to_string(),
                architecture: "x86".to_string(),
                bitness: "64".to_string(),
                model: "".to_string(),
                mobile: false,
                wow64: None,
                form_factors: Some(vec!["Desktop".to_string()]),
            }),
        },
        screen: desktop_screen(),
        device_environment: desktop_environment(),
        locale: us_eastern_locale(),
    }
}
