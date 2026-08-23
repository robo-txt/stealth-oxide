use super::*;

/// Returns the built-in Linux Chrome desktop profile.
pub fn chrome_linux() -> BrowserProfile {
    BrowserProfile {
        name: "chrome-linux".to_string(),
        navigator: NavigatorProfile {
            user_agent: format!(
                "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{CHROME_VERSION} Safari/537.36"
            ),
            platform: "Linux x86_64".to_string(),
            languages: vec!["en-US".to_string(), "en".to_string()],
            client_hints: Some(UserAgentClientHintsProfile {
                brands: chrome_brands(false),
                full_version_list: chrome_brands(true),
                platform: "Linux".to_string(),
                platform_version: "".to_string(),
                architecture: "x86".to_string(),
                bitness: "64".to_string(),
                model: "".to_string(),
                mobile: false,
                wow64: None,
                form_factors: Some(vec!["Desktop".to_string()]),
            }),
        },
        screen: super::desktop_screen(),
        device_environment: super::desktop_environment(),
        locale: super::us_eastern_locale(),
        hardware: HardwareProfile::new(8),
        version: Some(ProfileVersion::built_in()),
    }
}
