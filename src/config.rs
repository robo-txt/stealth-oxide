use anyhow::{Result, bail};

use crate::profiles::BrowserProfile;
use crate::profiles::chrome_linux::chrome_linux;
use crate::profiles::chrome_macos::chrome_macos;
use crate::profiles::chrome_windows::chrome_windows;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyConfig {
    pub server: String,
    pub username: Option<String>,
    pub password: Option<String>,
}

impl ProxyConfig {
    pub fn new(server: impl Into<String>) -> Result<Self> {
        let proxy = Self {
            server: server.into(),
            username: None,
            password: None,
        };
        proxy.validate()?;
        Ok(proxy)
    }

    pub fn parse(value: &str) -> Result<Self> {
        if !value.contains("://") {
            let fields = value.split(':').collect::<Vec<_>>();
            if fields.len() != 4 || fields.iter().any(|field| field.is_empty()) {
                bail!("scheme-less proxies must use host:port:username:password");
            }
            let proxy = Self {
                server: format!("http://{}:{}", fields[0], fields[1]),
                username: Some(fields[2].to_string()),
                password: Some(fields[3].to_string()),
            };
            proxy.validate()?;
            return Ok(proxy);
        }

        let mut parsed = url::Url::parse(value)
            .map_err(|error| anyhow::anyhow!("invalid proxy URL: {error}"))?;
        if !matches!(parsed.scheme(), "http" | "https" | "socks4" | "socks5") {
            bail!("proxy scheme must be http, https, socks4, or socks5");
        }
        if parsed.host().is_none() || parsed.port().is_none() {
            bail!("proxy URL must include a host and port");
        }
        if parsed.path() != "/" || parsed.query().is_some() || parsed.fragment().is_some() {
            bail!("proxy URL cannot contain a path, query, or fragment");
        }

        let username = (!parsed.username().is_empty()).then(|| parsed.username().to_string());
        let password = parsed.password().map(str::to_string);
        parsed
            .set_username("")
            .map_err(|_| anyhow::anyhow!("failed to remove proxy username"))?;
        parsed
            .set_password(None)
            .map_err(|_| anyhow::anyhow!("failed to remove proxy password"))?;
        let proxy = Self {
            server: parsed.as_str().trim_end_matches('/').to_string(),
            username,
            password,
        };
        proxy.validate()?;
        Ok(proxy)
    }

    pub fn credentials(mut self, username: impl Into<String>, password: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self.password = Some(password.into());
        self
    }

    pub fn label(&self) -> String {
        if self.username.is_some() {
            format!("{} (authenticated)", self.server)
        } else {
            self.server.clone()
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.server.chars().any(char::is_whitespace) {
            bail!("proxy server cannot contain whitespace");
        }
        if self.username.is_some() != self.password.is_some() {
            bail!("proxy username and password must be provided together");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformProfile {
    Linux,
    MacOS,
    Windows,
}

impl PlatformProfile {
    pub fn profile(self) -> BrowserProfile {
        match self {
            Self::Linux => chrome_linux(),
            Self::MacOS => chrome_macos(),
            Self::Windows => chrome_windows(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchSet {
    pub identity: bool,
    pub locale_and_timezone: bool,
    pub screen: bool,
    pub device_environment: bool,
}

impl PatchSet {
    pub fn all() -> Self {
        Self {
            identity: true,
            locale_and_timezone: true,
            screen: true,
            device_environment: true,
        }
    }

    pub fn none() -> Self {
        Self {
            identity: false,
            locale_and_timezone: false,
            screen: false,
            device_environment: false,
        }
    }

    pub fn identity(mut self, enabled: bool) -> Self {
        self.identity = enabled;
        self
    }

    pub fn locale_and_timezone(mut self, enabled: bool) -> Self {
        self.locale_and_timezone = enabled;
        self
    }

    pub fn screen(mut self, enabled: bool) -> Self {
        self.screen = enabled;
        self
    }

    pub fn device_environment(mut self, enabled: bool) -> Self {
        self.device_environment = enabled;
        self
    }
}

impl Default for PatchSet {
    fn default() -> Self {
        Self::all()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LaunchOptions {
    pub headful: bool,
    pub mesa: bool,
    pub speech_dispatcher: bool,
    pub proxy: Option<ProxyConfig>,
}

impl LaunchOptions {
    pub fn headful(mut self, enabled: bool) -> Self {
        self.headful = enabled;
        self
    }

    pub fn mesa(mut self, enabled: bool) -> Self {
        self.mesa = enabled;
        self
    }

    pub fn speech_dispatcher(mut self, enabled: bool) -> Self {
        self.speech_dispatcher = enabled;
        self
    }

    pub fn proxy(mut self, proxy: ProxyConfig) -> Self {
        self.proxy = Some(proxy);
        self
    }

    pub(crate) fn from_legacy_environment() -> Self {
        Self::default()
            .headful(env_enabled("STEALTH_OXIDE_HEADFUL"))
            .mesa(env_enabled("STEALTH_OXIDE_USE_MESA"))
            .speech_dispatcher(env_enabled("STEALTH_OXIDE_SPEECH_DISPATCHER"))
    }
}

#[derive(Debug, Clone)]
pub struct StealthConfig {
    pub profile: BrowserProfile,
    pub launch: LaunchOptions,
    pub patches: PatchSet,
}

impl StealthConfig {
    pub fn builder() -> StealthConfigBuilder {
        StealthConfigBuilder::default()
    }

    pub fn new(profile: BrowserProfile) -> Result<Self> {
        Self::builder().profile(profile).build()
    }

    pub fn validate(&self) -> Result<()> {
        validate_profile(&self.profile)?;
        if let Some(proxy) = &self.launch.proxy {
            proxy.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct StealthConfigBuilder {
    profile: Option<BrowserProfile>,
    launch: LaunchOptions,
    patches: PatchSet,
}

impl StealthConfigBuilder {
    pub fn platform(mut self, platform: PlatformProfile) -> Self {
        self.profile = Some(platform.profile());
        self
    }

    pub fn profile(mut self, profile: BrowserProfile) -> Self {
        self.profile = Some(profile);
        self
    }

    pub fn launch_options(mut self, launch: LaunchOptions) -> Self {
        self.launch = launch;
        self
    }

    pub fn patches(mut self, patches: PatchSet) -> Self {
        self.patches = patches;
        self
    }

    pub fn headful(mut self, enabled: bool) -> Self {
        self.launch.headful = enabled;
        self
    }

    pub fn mesa(mut self, enabled: bool) -> Self {
        self.launch.mesa = enabled;
        self
    }

    pub fn speech_dispatcher(mut self, enabled: bool) -> Self {
        self.launch.speech_dispatcher = enabled;
        self
    }

    pub fn proxy(mut self, proxy: ProxyConfig) -> Self {
        self.launch.proxy = Some(proxy);
        self
    }

    pub fn build(self) -> Result<StealthConfig> {
        let config = StealthConfig {
            profile: self
                .profile
                .ok_or_else(|| anyhow::anyhow!("a browser profile is required"))?,
            launch: self.launch,
            patches: self.patches,
        };
        config.validate()?;
        Ok(config)
    }
}

pub fn validate_profile(profile: &BrowserProfile) -> Result<()> {
    if profile.screen.width == 0 || profile.screen.height == 0 {
        bail!("screen dimensions must be greater than zero");
    }
    if profile.screen.available_width > profile.screen.width
        || profile.screen.available_height > profile.screen.height
    {
        bail!("available screen dimensions cannot exceed screen dimensions");
    }
    if !profile.screen.device_scale_factor.is_finite() || profile.screen.device_scale_factor <= 0.0
    {
        bail!("device scale factor must be finite and greater than zero");
    }
    if profile.navigator.languages.first() != Some(&profile.locale.locale) {
        bail!("the first navigator language must match the Intl locale");
    }
    if !profile.device_environment.touch_enabled && profile.device_environment.max_touch_points != 0
    {
        bail!("a non-touch profile must report zero maximum touch points");
    }

    if let Some(hints) = &profile.navigator.client_hints {
        let ua = &profile.navigator.user_agent;
        let platform_is_consistent = match hints.platform.as_str() {
            "Linux" => ua.contains("Linux") && profile.navigator.platform.contains("Linux"),
            "Windows" => ua.contains("Windows") && profile.navigator.platform == "Win32",
            "macOS" => ua.contains("Mac OS X") && profile.navigator.platform == "MacIntel",
            _ => true,
        };
        if !platform_is_consistent {
            bail!("user agent, navigator platform, and UA Client Hints platform disagree");
        }

        let ua_major = ua
            .split_whitespace()
            .find_map(|part| part.strip_prefix("Chrome/"))
            .and_then(|version| version.split('.').next());
        if let Some(ua_major) = ua_major {
            for brand in hints
                .brands
                .iter()
                .filter(|brand| brand.brand == "Chromium" || brand.brand == "Google Chrome")
            {
                if brand.version != ua_major {
                    bail!("Chrome user-agent and UA Client Hints major versions disagree");
                }
            }
        }
    }

    Ok(())
}

fn env_enabled(name: &str) -> bool {
    matches!(std::env::var(name).as_deref(), Ok("1") | Ok("true"))
}
