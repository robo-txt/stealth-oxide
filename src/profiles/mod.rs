pub mod chrome_linux;
pub mod chrome_macos;
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
    pub available_width: u32,
    pub available_height: u32,
    pub device_scale_factor: f64,
}

#[derive(Debug, Clone)]
pub struct DeviceEnvironmentProfile {
    pub color_scheme: String,
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

#[derive(Debug, Clone)]
pub struct BrowserProfileBuilder {
    profile: BrowserProfile,
}

impl BrowserProfileBuilder {
    pub fn new(profile: BrowserProfile) -> Self {
        Self { profile }
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.profile.name = name.into();
        self
    }

    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.profile.navigator.user_agent = user_agent.into();
        self
    }

    pub fn navigator_platform(mut self, platform: impl Into<String>) -> Self {
        self.profile.navigator.platform = platform.into();
        self
    }

    pub fn languages<I, S>(mut self, languages: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.profile.navigator.languages = languages.into_iter().map(Into::into).collect();
        self
    }

    pub fn locale(mut self, locale: impl Into<String>) -> Self {
        let locale = locale.into();
        self.profile.locale.locale = locale.clone();
        if self.profile.navigator.languages.is_empty() {
            self.profile.navigator.languages.push(locale);
        } else {
            self.profile.navigator.languages[0] = locale;
        }
        self
    }

    pub fn timezone(mut self, timezone: impl Into<String>) -> Self {
        self.profile.locale.timezone = timezone.into();
        self
    }

    pub fn screen(mut self, width: u32, height: u32) -> Self {
        self.profile.screen.width = width;
        self.profile.screen.height = height;
        self
    }

    pub fn available_screen(mut self, width: u32, height: u32) -> Self {
        self.profile.screen.available_width = width;
        self.profile.screen.available_height = height;
        self
    }

    pub fn device_scale_factor(mut self, factor: f64) -> Self {
        self.profile.screen.device_scale_factor = factor;
        self
    }

    pub fn color_scheme(mut self, scheme: impl Into<String>) -> Self {
        self.profile.device_environment.color_scheme = scheme.into();
        self
    }

    pub fn reduced_motion(mut self, preference: impl Into<String>) -> Self {
        self.profile.device_environment.reduced_motion = preference.into();
        self
    }

    pub fn touch(mut self, enabled: bool, max_touch_points: u32) -> Self {
        self.profile.device_environment.touch_enabled = enabled;
        self.profile.device_environment.max_touch_points = max_touch_points;
        self
    }

    pub fn build(self) -> anyhow::Result<BrowserProfile> {
        crate::config::validate_profile(&self.profile)?;
        Ok(self.profile)
    }
}
