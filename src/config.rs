use crate::error::ValidationIssue;
use crate::profiles::chrome_linux::chrome_linux;
use crate::profiles::chrome_macos::chrome_macos;
use crate::profiles::chrome_windows::chrome_windows;
use crate::profiles::{
    BrowserProfile, ColorGamut, ColorScheme, ForcedColors, MediaFeaturesConfig, NavigatorProfile,
    ReducedMotion, ScreenConfig, TouchConfig, UserAgentClientHintsProfile,
};

/// Built-in desktop profile selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformProfile {
    /// Chrome running on Linux.
    Linux,
    /// Chrome running on macOS.
    MacOS,
    /// Chrome running on Windows.
    Windows,
}

/// An independently selectable CDP patch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Patch {
    /// User agent, navigator platform, languages, and UA Client Hints.
    Identity,
    /// Intl locale override.
    Locale,
    /// IANA timezone override.
    Timezone,
    /// Screen and viewport metrics.
    Screen,
    /// CSS media feature overrides.
    MediaFeatures,
    /// Touch capability emulation.
    Touch,
    /// Native logical-processor-count override.
    HardwareConcurrency,
}

/// Controls how one patch obtains its value.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum PatchMode<T> {
    /// Preserve the value already supplied by Chromium or the host environment.
    Native,
    /// Apply the supplied override through CDP.
    Override(T),
    /// Exclude the patch from the application plan.
    Disabled,
}

/// Non-generic state of a configured patch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatchState {
    /// Chromium's existing value is preserved.
    Native,
    /// A configured value will be applied.
    Override,
    /// The patch is excluded from the plan.
    Disabled,
}

/// Controls how cross-surface contradictions are handled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConsistencyPolicy {
    /// Reject known contradictions before executing any CDP command.
    #[default]
    Strict,
    /// Apply valid CDP values and include consistency issues in the report.
    Warn,
    /// Apply exactly what the caller requested, subject only to CDP requirements.
    Permissive,
}

impl PlatformProfile {
    /// Builds the selected profile.
    pub fn profile(self) -> BrowserProfile {
        match self {
            Self::Linux => chrome_linux(),
            Self::MacOS => chrome_macos(),
            Self::Windows => chrome_windows(),
        }
    }
}

/// Fully configurable patch selection and override values.
#[derive(Debug, Clone, PartialEq)]
pub struct StealthConfig {
    identity: PatchMode<NavigatorProfile>,
    locale: PatchMode<String>,
    timezone: PatchMode<String>,
    screen: PatchMode<ScreenConfig>,
    media_features: PatchMode<MediaFeaturesConfig>,
    touch: PatchMode<TouchConfig>,
    hardware_concurrency: PatchMode<u32>,
    policy: ConsistencyPolicy,
    defaults: BrowserProfile,
}

impl StealthConfig {
    /// Creates a configuration with every patch disabled.
    pub fn none() -> Self {
        let defaults = PlatformProfile::Linux.profile();
        Self {
            identity: PatchMode::Disabled,
            locale: PatchMode::Disabled,
            timezone: PatchMode::Disabled,
            screen: PatchMode::Disabled,
            media_features: PatchMode::Disabled,
            touch: PatchMode::Disabled,
            hardware_concurrency: PatchMode::Disabled,
            policy: ConsistencyPolicy::Strict,
            defaults,
        }
    }

    /// Creates a complete override configuration from a built-in platform preset.
    pub fn for_platform(platform: PlatformProfile) -> Self {
        Self::from_profile(platform.profile())
    }

    /// Creates a complete override configuration from an editable profile.
    pub fn from_profile(profile: BrowserProfile) -> Self {
        Self {
            identity: PatchMode::Override(profile.navigator.clone()),
            locale: PatchMode::Override(profile.locale.locale.clone()),
            timezone: PatchMode::Override(profile.locale.timezone.clone()),
            screen: PatchMode::Override((&profile.screen).into()),
            media_features: PatchMode::Override((&profile.device_environment).into()),
            touch: PatchMode::Override((&profile.device_environment).into()),
            // Keep the host value by default. This experimental CDP override is
            // opt-in because a profile should not silently contradict Chromium.
            hardware_concurrency: PatchMode::Native,
            policy: ConsistencyPolicy::Strict,
            defaults: profile,
        }
    }

    /// Selects strict, warning, or permissive consistency handling.
    pub const fn consistency_policy(mut self, policy: ConsistencyPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Enables a patch using the selected platform's default value.
    pub fn enable(mut self, patch: Patch) -> Self {
        match patch {
            Patch::Identity => self.identity = PatchMode::Override(self.defaults.navigator.clone()),
            Patch::Locale => self.locale = PatchMode::Override(self.defaults.locale.locale.clone()),
            Patch::Timezone => {
                self.timezone = PatchMode::Override(self.defaults.locale.timezone.clone());
            }
            Patch::Screen => self.screen = PatchMode::Override((&self.defaults.screen).into()),
            Patch::MediaFeatures => {
                self.media_features =
                    PatchMode::Override((&self.defaults.device_environment).into());
            }
            Patch::Touch => {
                self.touch = PatchMode::Override((&self.defaults.device_environment).into());
            }
            Patch::HardwareConcurrency => {
                self.hardware_concurrency =
                    PatchMode::Override(self.defaults.hardware.hardware_concurrency);
            }
        }
        self
    }

    /// Disables a patch completely.
    pub fn disable(mut self, patch: Patch) -> Self {
        match patch {
            Patch::Identity => self.identity = PatchMode::Disabled,
            Patch::Locale => self.locale = PatchMode::Disabled,
            Patch::Timezone => self.timezone = PatchMode::Disabled,
            Patch::Screen => self.screen = PatchMode::Disabled,
            Patch::MediaFeatures => self.media_features = PatchMode::Disabled,
            Patch::Touch => self.touch = PatchMode::Disabled,
            Patch::HardwareConcurrency => self.hardware_concurrency = PatchMode::Disabled,
        }
        self
    }

    /// Preserves Chromium's native value for a patch.
    pub fn use_native(mut self, patch: Patch) -> Self {
        match patch {
            Patch::Identity => self.identity = PatchMode::Native,
            Patch::Locale => self.locale = PatchMode::Native,
            Patch::Timezone => self.timezone = PatchMode::Native,
            Patch::Screen => self.screen = PatchMode::Native,
            Patch::MediaFeatures => self.media_features = PatchMode::Native,
            Patch::Touch => self.touch = PatchMode::Native,
            Patch::HardwareConcurrency => self.hardware_concurrency = PatchMode::Native,
        }
        self
    }

    /// Replaces the complete identity override.
    pub fn identity(mut self, identity: NavigatorProfile) -> Self {
        self.identity = PatchMode::Override(identity);
        self
    }

    /// Sets the complete identity patch mode.
    pub fn identity_patch(mut self, mode: PatchMode<NavigatorProfile>) -> Self {
        self.identity = mode;
        self
    }

    /// Replaces UA Client Hints and enables identity patching.
    pub fn client_hints(mut self, hints: Option<UserAgentClientHintsProfile>) -> Self {
        self.identity_value_mut().client_hints = hints;
        self
    }

    /// Overrides the user-agent string and enables identity patching.
    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.identity_value_mut().user_agent = user_agent.into();
        self
    }

    /// Overrides navigator platform and enables identity patching.
    pub fn navigator_platform(mut self, platform: impl Into<String>) -> Self {
        self.identity_value_mut().platform = platform.into();
        self
    }

    /// Overrides navigator languages and enables identity patching.
    pub fn languages<I, S>(mut self, languages: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.identity_value_mut().languages = languages.into_iter().map(Into::into).collect();
        self
    }

    /// Overrides the Intl locale and enables locale patching.
    pub fn locale(mut self, locale: impl Into<String>) -> Self {
        self.locale = PatchMode::Override(locale.into());
        self
    }

    /// Sets the complete locale patch mode.
    pub fn locale_patch(mut self, mode: PatchMode<String>) -> Self {
        self.locale = mode;
        self
    }

    /// Overrides the IANA timezone and enables timezone patching.
    pub fn timezone(mut self, timezone: impl Into<String>) -> Self {
        self.timezone = PatchMode::Override(timezone.into());
        self
    }

    /// Sets the complete timezone patch mode.
    pub fn timezone_patch(mut self, mode: PatchMode<String>) -> Self {
        self.timezone = mode;
        self
    }

    /// Replaces the complete screen override.
    pub fn screen(mut self, screen: ScreenConfig) -> Self {
        self.screen = PatchMode::Override(screen);
        self
    }

    /// Sets the complete screen patch mode.
    pub fn screen_patch(mut self, mode: PatchMode<ScreenConfig>) -> Self {
        self.screen = mode;
        self
    }

    /// Overrides total screen dimensions and enables screen patching.
    pub fn screen_size(mut self, width: u32, height: u32) -> Self {
        let screen = self.screen_value_mut();
        screen.width = width;
        screen.height = height;
        self
    }

    /// Overrides the device scale factor and enables screen patching.
    pub fn device_scale_factor(mut self, factor: f64) -> Self {
        self.screen_value_mut().device_scale_factor = factor;
        self
    }

    /// Replaces all media-feature overrides.
    pub fn media_features(mut self, media: MediaFeaturesConfig) -> Self {
        self.media_features = PatchMode::Override(media);
        self
    }

    /// Sets the complete media-feature patch mode.
    pub fn media_features_patch(mut self, mode: PatchMode<MediaFeaturesConfig>) -> Self {
        self.media_features = mode;
        self
    }

    /// Overrides `prefers-color-scheme` and enables media-feature patching.
    pub fn color_scheme(mut self, value: ColorScheme) -> Self {
        self.media_features_value_mut().color_scheme = value;
        self
    }

    /// Overrides `prefers-reduced-motion` and enables media-feature patching.
    pub fn reduced_motion(mut self, value: ReducedMotion) -> Self {
        self.media_features_value_mut().reduced_motion = value;
        self
    }

    /// Overrides `forced-colors` and enables media-feature patching.
    pub fn forced_colors(mut self, value: ForcedColors) -> Self {
        self.media_features_value_mut().forced_colors = value;
        self
    }

    /// Overrides `color-gamut` and enables media-feature patching.
    pub fn color_gamut(mut self, value: ColorGamut) -> Self {
        self.media_features_value_mut().color_gamut = value;
        self
    }

    /// Replaces the touch override.
    pub fn touch(mut self, enabled: bool, max_touch_points: u32) -> Self {
        self.touch = PatchMode::Override(TouchConfig {
            enabled,
            max_touch_points,
        });
        self
    }

    /// Sets the complete touch patch mode.
    pub fn touch_patch(mut self, mode: PatchMode<TouchConfig>) -> Self {
        self.touch = mode;
        self
    }

    /// Overrides `navigator.hardwareConcurrency` through Chromium's native CDP
    /// emulation command. No script is injected into the site realm.
    pub fn hardware_concurrency(mut self, value: u32) -> Self {
        self.hardware_concurrency = PatchMode::Override(value);
        self
    }

    /// Sets the complete hardware-concurrency patch mode.
    pub fn hardware_concurrency_patch(mut self, mode: PatchMode<u32>) -> Self {
        self.hardware_concurrency = mode;
        self
    }

    /// Returns the selected consistency policy.
    pub const fn policy(&self) -> ConsistencyPolicy {
        self.policy
    }

    /// Returns the current state of an individual patch.
    pub fn patch_state(&self, patch: Patch) -> PatchState {
        match patch {
            Patch::Identity => state(&self.identity),
            Patch::Locale => state(&self.locale),
            Patch::Timezone => state(&self.timezone),
            Patch::Screen => state(&self.screen),
            Patch::MediaFeatures => state(&self.media_features),
            Patch::Touch => state(&self.touch),
            Patch::HardwareConcurrency => state(&self.hardware_concurrency),
        }
    }

    /// Builds the deterministic patch order and validates it without a browser.
    pub fn plan(&self) -> PatchPlan {
        let operations = [
            Patch::Locale,
            Patch::Timezone,
            Patch::Identity,
            Patch::HardwareConcurrency,
            Patch::Screen,
            Patch::MediaFeatures,
            Patch::Touch,
        ]
        .into_iter()
        .map(|patch| (patch, self.patch_state(patch)))
        .collect();
        PatchPlan {
            operations,
            issues: crate::validation::validate_config(self),
        }
    }

    /// Returns every known consistency issue without applying any patch.
    pub fn validation_issues(&self) -> Vec<ValidationIssue> {
        self.plan().issues
    }

    /// Returns the configured identity override, if identity is in override mode.
    pub fn identity_override(&self) -> Option<&NavigatorProfile> {
        override_value(&self.identity)
    }

    /// Returns the configured locale override, if locale is in override mode.
    pub fn locale_override(&self) -> Option<&str> {
        override_value(&self.locale).map(String::as_str)
    }

    /// Returns the configured timezone override, if timezone is in override mode.
    pub fn timezone_override(&self) -> Option<&str> {
        override_value(&self.timezone).map(String::as_str)
    }

    /// Returns the configured screen override, if screen is in override mode.
    pub fn screen_override(&self) -> Option<&ScreenConfig> {
        override_value(&self.screen)
    }

    /// Returns the configured media-feature override, if enabled.
    pub fn media_features_override(&self) -> Option<&MediaFeaturesConfig> {
        override_value(&self.media_features)
    }

    /// Returns the configured touch override, if enabled.
    pub fn touch_override(&self) -> Option<&TouchConfig> {
        override_value(&self.touch)
    }

    /// Returns the configured hardware-concurrency override, if enabled.
    pub fn hardware_concurrency_override(&self) -> Option<u32> {
        override_value(&self.hardware_concurrency).copied()
    }

    pub(crate) fn identity_mode(&self) -> &PatchMode<NavigatorProfile> {
        &self.identity
    }

    pub(crate) fn locale_mode(&self) -> &PatchMode<String> {
        &self.locale
    }

    pub(crate) fn timezone_mode(&self) -> &PatchMode<String> {
        &self.timezone
    }

    pub(crate) fn screen_mode(&self) -> &PatchMode<ScreenConfig> {
        &self.screen
    }

    pub(crate) fn media_features_mode(&self) -> &PatchMode<MediaFeaturesConfig> {
        &self.media_features
    }

    pub(crate) fn touch_mode(&self) -> &PatchMode<TouchConfig> {
        &self.touch
    }

    pub(crate) fn hardware_concurrency_mode(&self) -> &PatchMode<u32> {
        &self.hardware_concurrency
    }

    fn identity_value_mut(&mut self) -> &mut NavigatorProfile {
        if !matches!(self.identity, PatchMode::Override(_)) {
            self.identity = PatchMode::Override(self.defaults.navigator.clone());
        }
        let PatchMode::Override(value) = &mut self.identity else {
            unreachable!("identity was replaced with an override")
        };
        value
    }

    fn screen_value_mut(&mut self) -> &mut ScreenConfig {
        if !matches!(self.screen, PatchMode::Override(_)) {
            self.screen = PatchMode::Override((&self.defaults.screen).into());
        }
        let PatchMode::Override(value) = &mut self.screen else {
            unreachable!("screen was replaced with an override")
        };
        value
    }

    fn media_features_value_mut(&mut self) -> &mut MediaFeaturesConfig {
        if !matches!(self.media_features, PatchMode::Override(_)) {
            self.media_features = PatchMode::Override((&self.defaults.device_environment).into());
        }
        let PatchMode::Override(value) = &mut self.media_features else {
            unreachable!("media features were replaced with an override")
        };
        value
    }
}

fn state<T>(mode: &PatchMode<T>) -> PatchState {
    match mode {
        PatchMode::Native => PatchState::Native,
        PatchMode::Override(_) => PatchState::Override,
        PatchMode::Disabled => PatchState::Disabled,
    }
}

fn override_value<T>(mode: &PatchMode<T>) -> Option<&T> {
    match mode {
        PatchMode::Override(value) => Some(value),
        PatchMode::Native | PatchMode::Disabled => None,
    }
}

/// Result of applying a stealth configuration.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ApplyReport {
    pub(crate) applied: Vec<Patch>,
    pub(crate) skipped: Vec<Patch>,
    pub(crate) native: Vec<Patch>,
    pub(crate) warnings: Vec<ValidationIssue>,
}

/// Deterministic patch order and validation result prepared without a browser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchPlan {
    operations: Vec<(Patch, PatchState)>,
    issues: Vec<ValidationIssue>,
}

impl PatchPlan {
    /// Ordered patch states that will be processed by [`StealthConfig::apply`].
    pub fn operations(&self) -> &[(Patch, PatchState)] {
        &self.operations
    }

    /// Known consistency issues in this plan.
    pub fn issues(&self) -> &[ValidationIssue] {
        &self.issues
    }
}

impl ApplyReport {
    /// Patches whose CDP commands completed successfully.
    pub fn applied(&self) -> &[Patch] {
        &self.applied
    }

    /// Patches explicitly disabled by the caller.
    pub fn skipped(&self) -> &[Patch] {
        &self.skipped
    }

    /// Patches configured to preserve Chromium's native values.
    pub fn native(&self) -> &[Patch] {
        &self.native
    }

    /// Non-fatal consistency issues produced in warning mode.
    pub fn warnings(&self) -> &[ValidationIssue] {
        &self.warnings
    }
}
