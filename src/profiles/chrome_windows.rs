use super::*;

pub fn chrome_windows() -> BrowserProfile {
    BrowserProfile {
        name: "chrome-windows".to_string(),
        navigator: NavigatorProfile {
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36"
                .to_string(),
            platform: "Win32".to_string(),
            languages: vec!["en-US".to_string(), "en".to_string()],
            hardware_concurrency: Some(8),
            device_memory: Some(8),
                    client_hints: Some(UserAgentClientHintsProfile{
            brands: vec![
                BrandVersion {
                    brand: "Chromium".to_string(),
                    version: "125".to_string(),
                },
                BrandVersion {
                    brand:"Chromium".to_string(),
                    version:"125".to_string(),
                },
                BrandVersion {
                    brand: "Not.A/Brand".to_string(),
                    version: "24".to_string(),
                },
            ],
            full_version_list: vec![
                BrandVersion {
                    brand: "Chromium".to_string(),
                    version: "125.0.0.0".to_string(),
                },
                BrandVersion {
                    brand: "Google Chrome".to_string(),
                    version: "125.0.0.0".to_string(),
                },
                BrandVersion {
                    brand: "Not.A/Brand".to_string(),
                    version: "24.0.0.0".to_string(),
                },
            ],
            platform: "Windows".to_string(),
            platform_version: "10.0.0".to_string(),
            architectuer: "x86".to_string(),
            bitness: "64".to_string(),
            model:"".to_string(),
            mobile: false,         
            })
        },
        screen: ScreenProfile {
            width: 1920,
            height: 1080,
            avail_width: 1920,
            avail_height: 1040,
            color_depth: 24,
            pixel_depth: 24,
            device_scale_factor: 1.0,
        },
        locale: LocaleProfile {
            locale: "en-US".to_string(),
            timezone: "America/New_York".to_string(),
        },
        webgl: Some(WebGLProfile{
            vendor: "Google Inc. (NVIDIA)".to_string(),
            renderer:
                "ANGLE (NVIDIA, NVIDIA GeForce GTX 1660 SUPER Direct3D11 vs_5_0 ps_5_0, D3D11)"
                    .to_string(),
        }),
    }
}
