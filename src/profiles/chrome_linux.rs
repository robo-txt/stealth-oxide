use super::*;

pub fn chrome_linux() -> BrowserProfile {
    BrowserProfile {
        name: "chrome-linux".to_string(),
        navigator: NavigatorProfile {
            user_agent: "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/151.0.7922.71 Safari/537.36"
                .to_string(),
            platform: "Linux x86_64".to_string(),
            languages: vec!["en-US".to_string(), "en".to_string()],
            client_hints: Some(UserAgentClientHintsProfile {
                brands: brands(false),
                full_version_list: brands(true),
                platform: "Linux".to_string(),
                platform_version: "".to_string(),
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

fn desktop_screen() -> ScreenProfile {
    ScreenProfile {
        width: 1920,
        height: 1080,
        available_width: 1920,
        available_height: 1040,
        device_scale_factor: 1.0,
    }
}

fn desktop_environment() -> DeviceEnvironmentProfile {
    DeviceEnvironmentProfile {
        color_scheme: "dark".to_string(),
        reduced_motion: "no-preference".to_string(),
        forced_colors: "none".to_string(),
        color_gamut: "srgb".to_string(),
        monochrome: "0".to_string(),
        touch_enabled: false,
        max_touch_points: 0,
    }
}

fn us_eastern_locale() -> LocaleProfile {
    LocaleProfile {
        locale: "en-US".to_string(),
        timezone: "America/New_York".to_string(),
    }
}
