use super::*;

/// Returns the built-in Windows Chrome desktop profile.
pub fn chrome_windows() -> BrowserProfile {
    BrowserProfile {
        name: "chrome-windows".to_string(),
        navigator: NavigatorProfile {
            user_agent: format!(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{CHROME_VERSION} Safari/537.36"
            ),
            platform: "Win32".to_string(),
            languages: vec!["en-US".to_string(), "en".to_string()],
            client_hints: Some(UserAgentClientHintsProfile {
                brands: chrome_brands(false),
                full_version_list: chrome_brands(true),
                platform: "Windows".to_string(),
                platform_version: "10.0.0".to_string(),
                architecture: "x86".to_string(),
                bitness: "64".to_string(),
                model: "".to_string(),
                mobile: false,
            }),
        },
        screen: desktop_screen(),
        device_environment: desktop_environment(),
        locale: us_eastern_locale(),
    }
}
