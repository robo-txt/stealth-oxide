/// Built-in Linux Chrome profile.
pub mod chrome_linux;
/// Built-in macOS Chrome profile.
pub mod chrome_macos;
/// Built-in Windows Chrome profile.
pub mod chrome_windows;

/// Full Chrome version modeled by the built-in profiles.
pub const BUILT_IN_CHROME_VERSION: &str = "151.0.7922.138";
/// Chrome major version modeled by the built-in profiles.
pub const BUILT_IN_CHROME_MAJOR: u32 = 151;

const CHROME_VERSION: &str = BUILT_IN_CHROME_VERSION;
const CHROME_MAJOR: &str = "151";

pub(crate) fn chrome_brands(full: bool) -> Vec<BrandVersion> {
    let chrome_version = if full { CHROME_VERSION } else { CHROME_MAJOR };
    let grease_version = if full { "24.0.0.0" } else { "24" };
    vec![
        BrandVersion {
            brand: "Chromium".to_string(),
            version: chrome_version.to_string(),
        },
        BrandVersion {
            brand: "Google Chrome".to_string(),
            version: chrome_version.to_string(),
        },
        BrandVersion {
            brand: "Not.A/Brand".to_string(),
            version: grease_version.to_string(),
        },
    ]
}

pub(crate) fn desktop_screen() -> ScreenProfile {
    ScreenProfile {
        width: 1920,
        height: 1080,
        available_width: 1920,
        available_height: 1040,
        device_scale_factor: 1.0,
    }
}

pub(crate) fn desktop_environment() -> DeviceEnvironmentProfile {
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

pub(crate) fn us_eastern_locale() -> LocaleProfile {
    LocaleProfile {
        locale: "en-US".to_string(),
        timezone: "America/New_York".to_string(),
    }
}

/// Hardware characteristics that can be modeled without changing the site realm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HardwareProfile {
    /// Logical processor count exposed through Chromium's native override.
    pub hardware_concurrency: u32,
}

/// CPU-rendered Docker GPU identities commonly seen on Windows and macOS.
///
/// These presets describe the identity exposed by ANGLE. They do not provide
/// hardware acceleration; the runtime remains Mesa/LLVMpipe when the preset's
/// launch environment is applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GpuPreset {
    /// Windows laptop Intel UHD 620.
    WindowsIntelUhd620,
    /// Windows laptop Intel Iris Xe.
    WindowsIntelIrisXe,
    /// Windows desktop NVIDIA GeForce GTX 1650.
    WindowsNvidiaGtx1650,
    /// Windows desktop NVIDIA GeForce RTX 3060.
    WindowsNvidiaRtx3060,
    /// Windows desktop AMD Radeon RX 580.
    WindowsAmdRadeonRx580,
    /// macOS Intel Iris Plus 645.
    MacosIntelIrisPlus645,
    /// macOS AMD Radeon Pro 5500M.
    MacosAmdRadeonPro5500m,
    /// Apple M1 integrated GPU.
    MacosAppleM1,
    /// Apple M2 integrated GPU.
    MacosAppleM2,
}

impl GpuPreset {
    /// Returns the stable catalog identifier.
    pub const fn name(self) -> &'static str {
        match self {
            Self::WindowsIntelUhd620 => "windows-intel-uhd-620",
            Self::WindowsIntelIrisXe => "windows-intel-iris-xe",
            Self::WindowsNvidiaGtx1650 => "windows-nvidia-gtx-1650",
            Self::WindowsNvidiaRtx3060 => "windows-nvidia-rtx-3060",
            Self::WindowsAmdRadeonRx580 => "windows-amd-radeon-rx-580",
            Self::MacosIntelIrisPlus645 => "macos-intel-iris-plus-645",
            Self::MacosAmdRadeonPro5500m => "macos-amd-radeon-pro-5500m",
            Self::MacosAppleM1 => "macos-apple-m1",
            Self::MacosAppleM2 => "macos-apple-m2",
        }
    }

    /// Returns the ANGLE/Mesa process settings for this identity.
    pub const fn runtime(self) -> GpuRuntimeProfile {
        match self {
            Self::WindowsIntelUhd620 => GpuRuntimeProfile {
                angle_vendor: "Intel",
                angle_renderer: "Intel(R) UHD Graphics 620",
            },
            Self::WindowsIntelIrisXe => GpuRuntimeProfile {
                angle_vendor: "Intel",
                angle_renderer: "Intel(R) Iris(R) Xe Graphics",
            },
            Self::WindowsNvidiaGtx1650 => GpuRuntimeProfile {
                angle_vendor: "NVIDIA Corporation",
                angle_renderer: "NVIDIA GeForce GTX 1650",
            },
            Self::WindowsNvidiaRtx3060 => GpuRuntimeProfile {
                angle_vendor: "NVIDIA Corporation",
                angle_renderer: "NVIDIA GeForce RTX 3060",
            },
            Self::WindowsAmdRadeonRx580 => GpuRuntimeProfile {
                angle_vendor: "AMD",
                angle_renderer: "AMD Radeon RX 580",
            },
            Self::MacosIntelIrisPlus645 => GpuRuntimeProfile {
                angle_vendor: "Intel",
                angle_renderer: "Intel Iris Plus Graphics 645",
            },
            Self::MacosAmdRadeonPro5500m => GpuRuntimeProfile {
                angle_vendor: "AMD",
                angle_renderer: "AMD Radeon Pro 5500M",
            },
            Self::MacosAppleM1 => GpuRuntimeProfile {
                angle_vendor: "Apple Inc.",
                angle_renderer: "Apple M1",
            },
            Self::MacosAppleM2 => GpuRuntimeProfile {
                angle_vendor: "Apple Inc.",
                angle_renderer: "Apple M2",
            },
        }
    }

    /// Returns the JavaScript-visible GPU identity associated with this preset.
    ///
    /// This is an opt-in JavaScript patch and is separate from the native
    /// Docker environment returned by [`Self::runtime`].
    pub fn webgl_profile(self) -> GpuProfile {
        let runtime = self.runtime();
        GpuProfile::new(runtime.angle_vendor, runtime.angle_renderer)
    }
}

/// Native process identity settings for CPU-rendered Docker Chromium.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GpuRuntimeProfile {
    /// Value supplied to ANGLE_GL_VENDOR.
    pub angle_vendor: &'static str,
    /// Value supplied to ANGLE_GL_RENDERER.
    pub angle_renderer: &'static str,
}

impl GpuRuntimeProfile {
    /// Returns environment variables for a Mesa/LLVMpipe Docker process.
    ///
    /// ANGLE_GL_VERSION is intentionally omitted so Chromium keeps its native
    /// version and capability surface.
    pub fn docker_process_envs(self) -> [(String, String); 4] {
        [
            ("LIBGL_ALWAYS_SOFTWARE".into(), "true".into()),
            ("MESA_LOADER_DRIVER_OVERRIDE".into(), "llvmpipe".into()),
            ("ANGLE_GL_VENDOR".into(), self.angle_vendor.into()),
            ("ANGLE_GL_RENDERER".into(), self.angle_renderer.into()),
        ]
    }
}

/// Permission setting understood by Chromium's native Browser domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionSetting {
    /// Allow the permission without prompting.
    Granted,
    /// Deny the permission without prompting.
    Denied,
    /// Restore the browser's prompt/default behavior.
    Prompt,
}

/// Origin-scoped native permission override.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionOverride {
    /// Chromium permission name, such as `geolocation` or `notifications`.
    pub name: String,
    /// Native permission setting.
    pub setting: PermissionSetting,
    /// Embedding origin. `None` applies to all origins in the browser context.
    pub origin: Option<String>,
    /// Optional embedded origin for embedded-origin policy decisions.
    pub embedded_origin: Option<String>,
}

impl PermissionOverride {
    /// Creates an origin-scoped permission override.
    pub fn for_origin(
        name: impl Into<String>,
        setting: PermissionSetting,
        origin: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            setting,
            origin: Some(origin.into()),
            embedded_origin: None,
        }
    }

    /// Creates a browser-context-wide permission override.
    pub fn all_origins(name: impl Into<String>, setting: PermissionSetting) -> Self {
        Self {
            name: name.into(),
            setting,
            origin: None,
            embedded_origin: None,
        }
    }

    /// Adds an embedded-origin restriction to the override.
    pub fn embedded_origin(mut self, origin: impl Into<String>) -> Self {
        self.embedded_origin = Some(origin.into());
        self
    }
}

/// Native geolocation position or unavailable-state override.
#[derive(Debug, Clone, PartialEq)]
pub struct GeolocationConfig {
    /// Mock latitude, or `None` to emulate an unavailable position.
    pub latitude: Option<f64>,
    /// Mock longitude, or `None` to emulate an unavailable position.
    pub longitude: Option<f64>,
    /// Mock accuracy in meters, or `None` to emulate an unavailable position.
    pub accuracy: Option<f64>,
    /// Optional mock altitude in meters.
    pub altitude: Option<f64>,
    /// Optional mock altitude accuracy in meters.
    pub altitude_accuracy: Option<f64>,
    /// Optional mock heading in degrees.
    pub heading: Option<f64>,
    /// Optional mock speed in meters per second.
    pub speed: Option<f64>,
}

impl GeolocationConfig {
    /// Creates a complete native position override.
    pub const fn position(latitude: f64, longitude: f64, accuracy: f64) -> Self {
        Self {
            latitude: Some(latitude),
            longitude: Some(longitude),
            accuracy: Some(accuracy),
            altitude: None,
            altitude_accuracy: None,
            heading: None,
            speed: None,
        }
    }

    /// Creates the native position-unavailable override.
    pub const fn unavailable() -> Self {
        Self {
            latitude: None,
            longitude: None,
            accuracy: None,
            altitude: None,
            altitude_accuracy: None,
            heading: None,
            speed: None,
        }
    }

    /// Adds optional altitude data.
    pub const fn altitude(mut self, value: f64) -> Self {
        self.altitude = Some(value);
        self
    }

    /// Adds optional altitude accuracy data.
    pub const fn altitude_accuracy(mut self, value: f64) -> Self {
        self.altitude_accuracy = Some(value);
        self
    }

    /// Adds optional heading data.
    pub const fn heading(mut self, value: f64) -> Self {
        self.heading = Some(value);
        self
    }

    /// Adds optional speed data.
    pub const fn speed(mut self, value: f64) -> Self {
        self.speed = Some(value);
        self
    }
}

impl HardwareProfile {
    /// Creates a hardware profile with the supplied logical processor count.
    pub const fn new(hardware_concurrency: u32) -> Self {
        Self {
            hardware_concurrency,
        }
    }
}

/// JavaScript-visible WebGL identity for an opt-in GPU surface override.
///
/// This experimental profile changes WebGL string queries while leaving
/// Chromium's native renderer in place. Numeric limits, extensions, shader
/// precision, and pixel output are intentionally not represented yet.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct GpuProfile {
    /// Value returned by `gl.getParameter(gl.VENDOR)`.
    pub vendor: String,
    /// Value returned by `gl.getParameter(gl.RENDERER)`.
    pub renderer: String,
    /// Value returned for `UNMASKED_VENDOR_WEBGL`.
    pub unmasked_vendor: String,
    /// Value returned for `UNMASKED_RENDERER_WEBGL`.
    pub unmasked_renderer: String,
    /// Value returned by `gl.getParameter(gl.VERSION)`.
    pub version: String,
    /// Value returned by `gl.getParameter(gl.SHADING_LANGUAGE_VERSION)`.
    pub shading_language_version: String,
}

impl GpuProfile {
    /// Creates a profile with Chromium's standard masked WebGL strings.
    pub fn new(unmasked_vendor: impl Into<String>, unmasked_renderer: impl Into<String>) -> Self {
        Self {
            vendor: "WebKit".to_string(),
            renderer: "WebKit WebGL".to_string(),
            unmasked_vendor: unmasked_vendor.into(),
            unmasked_renderer: unmasked_renderer.into(),
            version: "WebGL 1.0 (OpenGL ES 2.0 Chromium)".to_string(),
            shading_language_version: "WebGL GLSL ES 1.0 (OpenGL ES GLSL ES 1.0 Chromium)"
                .to_string(),
        }
    }

    /// Returns the AMD Renoir/Mesa identity observed on the development host.
    pub fn mesa_amd_renoir() -> Self {
        Self::new(
            "Google Inc. (AMD)",
            "ANGLE (AMD, AMD Radeon Graphics (radeonsi renoir ACO), OpenGL ES 3.2)",
        )
    }

    /// Replaces the masked WebGL vendor and renderer strings.
    pub fn masked_strings(
        mut self,
        vendor: impl Into<String>,
        renderer: impl Into<String>,
    ) -> Self {
        self.vendor = vendor.into();
        self.renderer = renderer.into();
        self
    }

    /// Replaces the WebGL and GLSL version strings.
    pub fn version_strings(
        mut self,
        version: impl Into<String>,
        shading_language_version: impl Into<String>,
    ) -> Self {
        self.version = version.into();
        self.shading_language_version = shading_language_version.into();
        self
    }
}

/// Coherent inputs consumed by the supported CDP patch groups.
#[derive(Debug, Clone, PartialEq)]
pub struct BrowserProfile {
    /// Human-readable profile name.
    pub(crate) name: String,
    /// Navigator and HTTP identity values.
    pub(crate) navigator: NavigatorProfile,
    /// Screen and viewport values.
    pub(crate) screen: ScreenProfile,
    /// Media preferences and touch capabilities.
    pub(crate) device_environment: DeviceEnvironmentProfile,
    /// Locale and timezone values.
    pub(crate) locale: LocaleProfile,
    /// Hardware characteristics used by opt-in native overrides.
    pub(crate) hardware: HardwareProfile,
    /// Browser version represented by this profile, when known.
    pub(crate) version: Option<ProfileVersion>,
}

impl BrowserProfile {
    /// Human-readable preset name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Navigator and HTTP identity defaults.
    pub fn navigator(&self) -> &NavigatorProfile {
        &self.navigator
    }

    /// Screen and viewport defaults.
    pub fn screen(&self) -> &ScreenProfile {
        &self.screen
    }

    /// Media-feature and touch defaults.
    pub fn device_environment(&self) -> &DeviceEnvironmentProfile {
        &self.device_environment
    }

    /// Locale and timezone defaults.
    pub fn locale(&self) -> &LocaleProfile {
        &self.locale
    }

    /// Hardware characteristics used by opt-in native overrides.
    pub const fn hardware(&self) -> HardwareProfile {
        self.hardware
    }

    /// Browser version represented by this profile.
    ///
    /// Built-in profiles always provide this metadata. A customized profile
    /// returns `None` after its user agent or Client Hints are replaced unless
    /// the caller supplies updated metadata through [`BrowserProfileBuilder`].
    pub const fn version(&self) -> Option<&ProfileVersion> {
        self.version.as_ref()
    }
}

/// Chrome version metadata attached to a browser profile.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ProfileVersion {
    /// Chrome major version.
    pub chrome_major: u32,
    /// Full dotted Chrome version.
    pub chrome_version: String,
}

impl ProfileVersion {
    /// Creates explicit profile version metadata.
    pub fn new(chrome_major: u32, chrome_version: impl Into<String>) -> Self {
        Self {
            chrome_major,
            chrome_version: chrome_version.into(),
        }
    }

    /// Version metadata used by all built-in profiles.
    pub fn built_in() -> Self {
        Self::new(BUILT_IN_CHROME_MAJOR, BUILT_IN_CHROME_VERSION)
    }

    /// Parses a Chrome, Chromium, or HeadlessChrome product string.
    pub fn from_product(product: &str) -> Option<Self> {
        let (name, version) = product.trim().split_once('/')?;
        if !matches!(name, "Chrome" | "HeadlessChrome" | "Chromium") {
            return None;
        }
        let major = version.split('.').next()?.parse().ok()?;
        Some(Self::new(major, version))
    }
}

/// Navigator identity and User-Agent Client Hint values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigatorProfile {
    /// Full user-agent string.
    pub user_agent: String,
    /// Value exposed as `navigator.platform` where CDP supports it.
    pub platform: String,
    /// Ordered navigator languages, with the primary locale first.
    pub languages: Vec<String>,
    /// Optional structured User-Agent Client Hint metadata.
    pub client_hints: Option<UserAgentClientHintsProfile>,
}

/// Identity override consumed by the identity patch.
pub type IdentityConfig = NavigatorProfile;

/// Structured User-Agent Client Hint metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserAgentClientHintsProfile {
    /// Reduced brand and major-version entries.
    pub brands: Vec<BrandVersion>,
    /// Brand entries containing full versions.
    pub full_version_list: Vec<BrandVersion>,
    /// Client Hint operating-system name.
    pub platform: String,
    /// Client Hint operating-system version.
    pub platform_version: String,
    /// Client Hint CPU architecture.
    pub architecture: String,
    /// Client Hint architecture bitness.
    pub bitness: String,
    /// Device model, empty for the desktop profiles.
    pub model: String,
    /// Whether the identity represents a mobile browser.
    pub mobile: bool,
    /// Optional Windows-on-Windows 64-bit marker.
    pub wow64: Option<bool>,
    /// Optional high-entropy form-factor values such as `Desktop` or `Mobile`.
    pub form_factors: Option<Vec<String>>,
}

/// A User-Agent Client Hint brand/version pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrandVersion {
    /// Brand name.
    pub brand: String,
    /// Major or full brand version.
    pub version: String,
}
/// Screen dimensions and device scale factor.
#[derive(Debug, Clone, PartialEq)]
pub struct ScreenProfile {
    /// Total screen width in CSS pixels.
    pub width: u32,
    /// Total screen height in CSS pixels.
    pub height: u32,
    /// Available screen width in CSS pixels.
    pub available_width: u32,
    /// Available screen height in CSS pixels.
    pub available_height: u32,
    /// Ratio between device and CSS pixels.
    pub device_scale_factor: f64,
}

/// Screen values that can be applied through the supported CDP command.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ScreenConfig {
    /// Screen width in CSS pixels.
    pub width: u32,
    /// Screen height in CSS pixels.
    pub height: u32,
    /// Available screen width in CSS pixels.
    pub available_width: u32,
    /// Available screen height in CSS pixels.
    pub available_height: u32,
    /// Ratio between device and CSS pixels.
    pub device_scale_factor: f64,
}

impl ScreenConfig {
    /// Creates an override with the supplied dimensions and scale factor.
    pub const fn new(width: u32, height: u32, device_scale_factor: f64) -> Self {
        Self {
            width,
            height,
            available_width: width,
            available_height: height,
            device_scale_factor,
        }
    }

    /// Replaces screen dimensions.
    pub const fn dimensions(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Replaces the available work-area dimensions.
    pub const fn available_dimensions(mut self, width: u32, height: u32) -> Self {
        self.available_width = width;
        self.available_height = height;
        self
    }

    /// Replaces the device scale factor.
    pub const fn scale_factor(mut self, value: f64) -> Self {
        self.device_scale_factor = value;
        self
    }
}

impl From<&ScreenProfile> for ScreenConfig {
    fn from(profile: &ScreenProfile) -> Self {
        Self {
            width: profile.width,
            height: profile.height,
            available_width: profile.available_width,
            available_height: profile.available_height,
            device_scale_factor: profile.device_scale_factor,
        }
    }
}

/// Media preferences and input capabilities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceEnvironmentProfile {
    /// `prefers-color-scheme` value.
    pub color_scheme: String,
    /// `prefers-reduced-motion` value.
    pub reduced_motion: String,
    /// `forced-colors` value.
    pub forced_colors: String,
    /// `color-gamut` value.
    pub color_gamut: String,
    /// `monochrome` media feature value.
    pub monochrome: String,
    /// Whether touch emulation is enabled.
    pub touch_enabled: bool,
    /// Maximum simultaneous touch points.
    pub max_touch_points: u32,
}

/// Supported `prefers-color-scheme` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorScheme {
    /// Use a light color palette.
    Light,
    /// Use a dark color palette.
    Dark,
    /// Express no color-scheme preference.
    #[default]
    NoPreference,
}

impl ColorScheme {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
            Self::NoPreference => "no-preference",
        }
    }
}

/// Supported `prefers-reduced-motion` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReducedMotion {
    /// Request reduced motion.
    Reduce,
    /// Express no reduced-motion preference.
    #[default]
    NoPreference,
}

impl ReducedMotion {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Reduce => "reduce",
            Self::NoPreference => "no-preference",
        }
    }
}

/// Supported `forced-colors` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ForcedColors {
    /// Enable forced colors.
    Active,
    /// Disable forced colors.
    #[default]
    None,
}

impl ForcedColors {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::None => "none",
        }
    }
}

/// Supported `color-gamut` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorGamut {
    /// Standard RGB gamut.
    #[default]
    Srgb,
    /// Display P3 gamut.
    P3,
    /// Rec. 2020 gamut.
    Rec2020,
}

impl ColorGamut {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Srgb => "srgb",
            Self::P3 => "p3",
            Self::Rec2020 => "rec2020",
        }
    }
}

/// Typed CSS media-feature overrides.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MediaFeaturesConfig {
    /// Preferred color scheme.
    pub color_scheme: ColorScheme,
    /// Preferred motion level.
    pub reduced_motion: ReducedMotion,
    /// Forced-colors state.
    pub forced_colors: ForcedColors,
    /// Reported color gamut.
    pub color_gamut: ColorGamut,
    /// Reported monochrome bit depth.
    pub monochrome: u32,
}

impl MediaFeaturesConfig {
    /// Creates desktop media-feature defaults.
    pub const fn desktop() -> Self {
        Self {
            color_scheme: ColorScheme::NoPreference,
            reduced_motion: ReducedMotion::NoPreference,
            forced_colors: ForcedColors::None,
            color_gamut: ColorGamut::Srgb,
            monochrome: 0,
        }
    }

    /// Replaces the color-scheme preference.
    pub const fn color_scheme(mut self, value: ColorScheme) -> Self {
        self.color_scheme = value;
        self
    }

    /// Replaces the reduced-motion preference.
    pub const fn reduced_motion(mut self, value: ReducedMotion) -> Self {
        self.reduced_motion = value;
        self
    }

    /// Replaces the forced-colors preference.
    pub const fn forced_colors(mut self, value: ForcedColors) -> Self {
        self.forced_colors = value;
        self
    }

    /// Replaces the color-gamut preference.
    pub const fn color_gamut(mut self, value: ColorGamut) -> Self {
        self.color_gamut = value;
        self
    }

    /// Replaces the monochrome bit depth.
    pub const fn monochrome(mut self, value: u32) -> Self {
        self.monochrome = value;
        self
    }
}

/// Typed touch-emulation override.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct TouchConfig {
    /// Whether touch input is enabled.
    pub enabled: bool,
    /// Maximum simultaneous touch points.
    pub max_touch_points: u32,
}

impl TouchConfig {
    /// Creates a touch override.
    pub const fn new(enabled: bool, max_touch_points: u32) -> Self {
        Self {
            enabled,
            max_touch_points,
        }
    }
}

impl From<&DeviceEnvironmentProfile> for MediaFeaturesConfig {
    fn from(profile: &DeviceEnvironmentProfile) -> Self {
        Self {
            color_scheme: match profile.color_scheme.as_str() {
                "light" => ColorScheme::Light,
                "dark" => ColorScheme::Dark,
                _ => ColorScheme::NoPreference,
            },
            reduced_motion: if profile.reduced_motion == "reduce" {
                ReducedMotion::Reduce
            } else {
                ReducedMotion::NoPreference
            },
            forced_colors: if profile.forced_colors == "active" {
                ForcedColors::Active
            } else {
                ForcedColors::None
            },
            color_gamut: match profile.color_gamut.as_str() {
                "p3" => ColorGamut::P3,
                "rec2020" => ColorGamut::Rec2020,
                _ => ColorGamut::Srgb,
            },
            monochrome: profile.monochrome.parse().unwrap_or_default(),
        }
    }
}

impl From<&DeviceEnvironmentProfile> for TouchConfig {
    fn from(profile: &DeviceEnvironmentProfile) -> Self {
        Self {
            enabled: profile.touch_enabled,
            max_touch_points: profile.max_touch_points,
        }
    }
}

/// Intl locale and IANA timezone identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocaleProfile {
    /// BCP 47 locale identifier.
    pub locale: String,
    /// IANA timezone identifier.
    pub timezone: String,
}

/// Builder that customizes a preset and validates it on completion.
#[derive(Debug, Clone)]
pub struct BrowserProfileBuilder {
    profile: BrowserProfile,
}

impl BrowserProfileBuilder {
    /// Starts from an existing coherent profile.
    pub fn new(profile: BrowserProfile) -> Self {
        Self { profile }
    }

    /// Replaces the human-readable profile name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.profile.name = name.into();
        self
    }

    /// Replaces the user-agent string.
    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.profile.navigator.user_agent = user_agent.into();
        self.profile.version = None;
        self
    }

    /// Replaces the navigator platform token.
    pub fn navigator_platform(mut self, platform: impl Into<String>) -> Self {
        self.profile.navigator.platform = platform.into();
        self
    }

    /// Replaces structured UA Client Hints or removes them.
    pub fn client_hints(mut self, hints: Option<UserAgentClientHintsProfile>) -> Self {
        self.profile.navigator.client_hints = hints;
        self.profile.version = None;
        self
    }

    /// Supplies version metadata for a customized browser identity.
    pub fn version(mut self, version: Option<ProfileVersion>) -> Self {
        self.profile.version = version;
        self
    }

    /// Updates Chrome version tokens in the UA and Client Hints metadata.
    ///
    /// This is useful when a typed platform profile is used with a real
    /// Chromium binary whose version differs from the profile's default.
    /// Operating-system identity and host-native surfaces remain unchanged.
    pub fn chrome_version(mut self, version: ProfileVersion) -> Self {
        replace_user_agent_chrome_version(
            &mut self.profile.navigator.user_agent,
            &version.chrome_version,
        );
        if let Some(client_hints) = &mut self.profile.navigator.client_hints {
            for brand in &mut client_hints.brands {
                if matches!(brand.brand.as_str(), "Chromium" | "Google Chrome") {
                    brand.version = version.chrome_major.to_string();
                }
            }
            for brand in &mut client_hints.full_version_list {
                if matches!(brand.brand.as_str(), "Chromium" | "Google Chrome") {
                    brand.version = version.chrome_version.clone();
                }
            }
        }
        self.profile.version = Some(version);
        self
    }

    /// Replaces the ordered navigator language list.
    pub fn languages<I, S>(mut self, languages: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.profile.navigator.languages = languages.into_iter().map(Into::into).collect();
        self
    }

    /// Sets the Intl locale and synchronizes the primary navigator language.
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

    /// Sets the IANA timezone identifier.
    pub fn timezone(mut self, timezone: impl Into<String>) -> Self {
        self.profile.locale.timezone = timezone.into();
        self
    }

    /// Sets the logical processor count for an opt-in native override.
    pub fn hardware_concurrency(mut self, value: u32) -> Self {
        self.profile.hardware.hardware_concurrency = value;
        self
    }

    /// Sets total screen dimensions.
    pub fn screen(mut self, width: u32, height: u32) -> Self {
        self.profile.screen.width = width;
        self.profile.screen.height = height;
        self
    }

    /// Sets available work-area dimensions.
    pub fn available_screen(mut self, width: u32, height: u32) -> Self {
        self.profile.screen.available_width = width;
        self.profile.screen.available_height = height;
        self
    }

    /// Sets the device scale factor.
    pub fn device_scale_factor(mut self, factor: f64) -> Self {
        self.profile.screen.device_scale_factor = factor;
        self
    }

    /// Sets the preferred color scheme.
    pub fn color_scheme(mut self, scheme: ColorScheme) -> Self {
        self.profile.device_environment.color_scheme = scheme.as_str().to_string();
        self
    }

    /// Sets the reduced-motion preference.
    pub fn reduced_motion(mut self, preference: ReducedMotion) -> Self {
        self.profile.device_environment.reduced_motion = preference.as_str().to_string();
        self
    }

    /// Sets the forced-colors preference.
    pub fn forced_colors(mut self, value: ForcedColors) -> Self {
        self.profile.device_environment.forced_colors = value.as_str().to_string();
        self
    }

    /// Sets the color-gamut preference.
    pub fn color_gamut(mut self, value: ColorGamut) -> Self {
        self.profile.device_environment.color_gamut = value.as_str().to_string();
        self
    }

    /// Sets the monochrome bit depth.
    pub fn monochrome(mut self, value: u32) -> Self {
        self.profile.device_environment.monochrome = value.to_string();
        self
    }

    /// Configures touch support and maximum touch points.
    pub fn touch(mut self, enabled: bool, max_touch_points: u32) -> Self {
        self.profile.device_environment.touch_enabled = enabled;
        self.profile.device_environment.max_touch_points = max_touch_points;
        self
    }

    /// Validates and returns the customized profile.
    pub fn build(self) -> crate::Result<BrowserProfile> {
        crate::validation::validate_profile(&self.profile)?;
        Ok(self.profile)
    }
}

fn replace_user_agent_chrome_version(user_agent: &mut String, version: &str) {
    let Some(marker_start) = user_agent.find("Chrome/") else {
        return;
    };
    let version_start = marker_start + "Chrome/".len();
    let version_end = user_agent[version_start..]
        .find(char::is_whitespace)
        .map(|offset| version_start + offset)
        .unwrap_or(user_agent.len());
    user_agent.replace_range(version_start..version_end, version);
}
