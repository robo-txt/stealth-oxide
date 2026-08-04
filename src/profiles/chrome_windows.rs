use super::*;

pub fn chrome_windows() -> BrowserProfile {
    BrowserProfile {
        name: "chrome-windows".to_string(),
        navigator: NavigatorProfile {
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/151.0.7922.71 Safari/537.36"
                .to_string(),
            platform: "Win32".to_string(),
            languages: vec!["en-US".to_string(), "en".to_string()],
            //vendor:"Google Inc.".to_string(),
            client_hints: Some(UserAgentClientHintsProfile{
                brands: vec![
                    BrandVersion {
                        brand: "Chromium".to_string(),
                        version: "151".to_string(),
                    },
                    BrandVersion {
                        brand:"Google Chrome".to_string(),
                        version:"151".to_string(),
                    },
                    BrandVersion {
                        brand: "Not.A/Brand".to_string(),
                        version: "24".to_string(),
                    },
                ],
                full_version_list: vec![
                    BrandVersion {
                        brand: "Chromium".to_string(),
                        version: "151.0.7922.71".to_string(),
                    },
                    BrandVersion {
                        brand: "Google Chrome".to_string(),
                        version: "151.0.7922.71".to_string(),
                    },
                    BrandVersion {
                        brand: "Not.A/Brand".to_string(),
                        version: "24.0.0.0".to_string(),
                    },
                ],
                platform: "Windows".to_string(),
                platform_version: "10.0.0".to_string(),
                architecture: "x86".to_string(),
                bitness: "64".to_string(),
                model:"".to_string(),
                mobile: false,
                })
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
