//pub mod chrome_linux;
//pub mod chrome_macos;
pub mod chrome_windows;
//pub mod custom;

#[derive(Debug, Clone)]
pub struct BrowserProfile {
    pub name: String,
    pub navigator: NavigatorProfile,
    pub screen: ScreenProfile,
    pub device_environment: DeviceEnvironmentProfile,
    pub locale: LocaleProfile,
}

#[derive(Debug, Clone)]
pub struct NavigatorProfile {
    pub user_agent: String,
    pub platform: String,
    pub languages: Vec<String>,
    //pub vendor: String,
    pub client_hints: Option<UserAgentClientHintsProfile>,
}

#[derive(Debug, Clone)]
pub struct UserAgentClientHintsProfile {
    pub brands: Vec<BrandVersion>,
    pub full_version_list: Vec<BrandVersion>,
    pub platform: String,
    pub platform_version: String,
    pub architecture: String,
    pub bitness: String,
    pub model: String,
    pub mobile: bool,
}

#[derive(Debug, Clone)]
pub struct BrandVersion {
    pub brand: String,
    pub version: String,
}
#[derive(Debug, Clone)]
pub struct ScreenProfile {
    pub width: u32,
    pub height: u32,
    pub device_scale_factor: f64,
}

#[derive(Debug, Clone)]
pub struct DeviceEnvironmentProfile {
    pub reduced_motion: String,
    pub forced_colors: String,
    pub color_gamut: String,
    pub monochrome: String,
    pub touch_enabled: bool,
    pub max_touch_points: u32,
}

#[derive(Debug, Clone)]
pub struct LocaleProfile {
    pub locale: String,
    pub timezone: String,
}
