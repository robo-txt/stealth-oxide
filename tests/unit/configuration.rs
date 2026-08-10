use stealth_oxide::profiles::chrome_linux::chrome_linux;
use stealth_oxide::profiles::chrome_windows::chrome_windows;
use stealth_oxide::{
    ApplyReport, BrowserProfileBuilder, ColorScheme, ConsistencyPolicy, Error, Patch, PatchMode,
    PatchState, PlatformProfile, StealthConfig,
};

#[test]
fn selects_a_typed_linux_profile() {
    let profile = PlatformProfile::Linux.profile();
    assert_eq!(profile.name(), "chrome-linux");
}

#[test]
fn none_disables_every_patch() {
    let config = StealthConfig::none();
    for patch in [
        Patch::Identity,
        Patch::Locale,
        Patch::Timezone,
        Patch::Screen,
        Patch::MediaFeatures,
        Patch::Touch,
    ] {
        assert_eq!(config.patch_state(patch), PatchState::Disabled);
    }
}

#[test]
fn value_overrides_enable_their_patch() {
    let config = StealthConfig::none()
        .locale_patch(PatchMode::Override("en-CA".to_string()))
        .timezone("America/Toronto")
        .screen_size(2560, 1440)
        .device_scale_factor(1.25)
        .color_scheme(ColorScheme::Dark)
        .touch(false, 0);

    assert_eq!(config.locale_override(), Some("en-CA"));
    assert_eq!(config.timezone_override(), Some("America/Toronto"));
    assert_eq!(config.screen_override().unwrap().width, 2560);
    assert_eq!(config.screen_override().unwrap().device_scale_factor, 1.25);
    assert_eq!(
        config.media_features_override().unwrap().color_scheme,
        ColorScheme::Dark
    );
    assert!(!config.touch_override().unwrap().enabled);
}

#[test]
fn apply_report_defaults_to_no_operations() {
    let report = ApplyReport::default();
    assert!(report.applied().is_empty());
    assert!(report.skipped().is_empty());
    assert!(report.native().is_empty());
    assert!(report.warnings().is_empty());
}

#[test]
fn patch_plan_order_is_deterministic() {
    let config = StealthConfig::for_platform(PlatformProfile::Linux);
    let patches = config
        .plan()
        .operations()
        .iter()
        .map(|(patch, _)| *patch)
        .collect::<Vec<_>>();

    assert_eq!(
        patches,
        vec![
            Patch::Locale,
            Patch::Timezone,
            Patch::Identity,
            Patch::Screen,
            Patch::MediaFeatures,
            Patch::Touch,
        ]
    );
}

#[test]
fn public_configuration_types_remain_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<StealthConfig>();
    assert_send_sync::<ApplyReport>();
    assert_send_sync::<Error>();
}

#[test]
fn supports_playwright_style_patch_selection() {
    let config = StealthConfig::none()
        .enable(Patch::Identity)
        .enable(Patch::Timezone)
        .use_native(Patch::Screen)
        .disable(Patch::Touch)
        .consistency_policy(ConsistencyPolicy::Warn);

    assert_eq!(config.policy(), ConsistencyPolicy::Warn);
    assert_eq!(config.patch_state(Patch::Identity), PatchState::Override);
    assert_eq!(config.patch_state(Patch::Screen), PatchState::Native);
    assert_eq!(config.patch_state(Patch::Touch), PatchState::Disabled);
}

#[test]
fn permissive_configuration_accepts_intentional_identity_mismatches() {
    let config = StealthConfig::for_platform(PlatformProfile::Linux)
        .navigator_platform("Win32")
        .consistency_policy(ConsistencyPolicy::Permissive);

    assert_eq!(config.policy(), ConsistencyPolicy::Permissive);
    assert!(!config.validation_issues().is_empty());
}

#[test]
fn disabled_patches_are_excluded_from_validation() {
    let config = StealthConfig::for_platform(PlatformProfile::Linux)
        .navigator_platform("Win32")
        .disable(Patch::Identity);

    assert!(config.validation_issues().is_empty());
}

#[test]
fn detects_touch_contradictions_before_launching_chromium() {
    let config = StealthConfig::none().touch(true, 0);

    assert_eq!(config.validation_issues().len(), 1);
}

#[test]
fn rejects_cross_platform_identity_contradictions() {
    let profile = BrowserProfileBuilder::new(chrome_linux())
        .navigator_platform("Win32")
        .build()
        .unwrap_err();

    assert!(matches!(profile, Error::Validation { .. }));
    assert!(profile.to_string().contains("disagree"));
}

#[test]
fn rejects_locale_language_contradictions() {
    let profile = BrowserProfileBuilder::new(chrome_windows())
        .languages(["en-US", "en"])
        .locale("fr-FR")
        .languages(["en-US", "en"])
        .build()
        .unwrap_err();

    assert!(profile.to_string().contains("first navigator language"));
}

#[test]
fn customizes_a_preset_without_breaking_coupled_locale_fields() -> stealth_oxide::Result<()> {
    let profile = BrowserProfileBuilder::new(chrome_linux())
        .locale("en-CA")
        .timezone("America/Toronto")
        .screen(2560, 1440)
        .available_screen(2560, 1400)
        .device_scale_factor(1.25)
        .build()?;

    assert_eq!(profile.locale().locale, "en-CA");
    assert_eq!(profile.navigator().languages[0], "en-CA");
    assert_eq!(profile.locale().timezone, "America/Toronto");
    assert_eq!(profile.screen().device_scale_factor, 1.25);
    Ok(())
}
