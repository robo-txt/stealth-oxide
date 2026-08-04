use super::*;

pub fn chrome_macos() -> BrowserProfile {
    BrowserProfile {
        name: "chrome-macos".to_string(),
        navigator: NavigatorProfile {
            // Chrome freezes the macOS token in the reduced user-agent string.
            user_agent: "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/151.0.7922.71 Safari/537.36"
                .to_string(),
            platform: "MacIntel".to_string(),
            languages: vec!["en-US".to_string(), "en".to_string()],
            client_hints: Some(UserAgentClientHintsProfile {
                brands: brands(false),
                full_version_list: brands(true),
                platform: "macOS".to_string(),
                platform_version: "15.6.1".to_string(),
                architecture: "x86".to_string(),
                bitness: "64".to_string(),
                model: "".to_string(),
                mobile: false,
            }),
        },
        screen: ScreenProfile {
            width: 1920,
            height: 1080,
            available_width: 1920,
            available_height: 1040,
            device_scale_factor: 1.0,
        },
        device_environment: DeviceEnvironmentProfile {
            color_scheme: "dark".to_string(),
            reduced_motion: "no-preference".to_string(),
            forced_colors: "none".to_string(),
            color_gamut: "srgb".to_string(),
            monochrome: "0".to_string(),
            touch_enabled: false,
            max_touch_points: 0,
        },
        locale: LocaleProfile {
            locale: "en-US".to_string(),
            timezone: "America/New_York".to_string(),
        },
    }
}

fn brands(full: bool) -> Vec<BrandVersion> {
    vec![
        BrandVersion {
            brand: "Chromium".to_string(),
            version: if full { "151.0.7922.71" } else { "151" }.to_string(),
        },
        BrandVersion {
            brand: "Google Chrome".to_string(),
            version: if full { "151.0.7922.71" } else { "151" }.to_string(),
        },
        BrandVersion {
            brand: "Not.A/Brand".to_string(),
            version: if full { "24.0.0.0" } else { "24" }.to_string(),
        },
    ]
}
