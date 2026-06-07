//pub mod chrome_linux;
//pub mod chrome_macos;
pub mod chrome_windows;
//pub mod custom;

#[derive(Debug, Clone)]
pub struct BrowserProfile {
    pub name: String,
    pub navigator: NavigatorProfile,
    pub screen: ScreenProfile,
    pub locale: LocaleProfile,
    pub webgl: Option<WebGLProfile>,
}

#[derive(Debug, Clone)]
pub struct NavigatorProfile {
    pub user_agent: String,
    pub platform: String,
    pub languages: Vec<String>,
    pub hardware_concurrency: Option<u32>,
    pub device_memory: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct ScreenProfile {
    pub width: u32,
    pub height: u32,
    pub avail_width: u32,
    pub avail_height: u32,
    pub color_depth: u32,
    pub pixel_depth: u32,
    pub device_scale_factor: f64,
}

#[derive(Debug, Clone)]
pub struct LocaleProfile {
    pub locale: String,
    pub timezone: String,
}

#[derive(Debug, Clone)]
pub struct WebGLProfile {
    pub vendor: String,
    pub renderer: String,
}
