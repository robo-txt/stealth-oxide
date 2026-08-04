use stealth_oxide::BrowserProfileBuilder;
use stealth_oxide::config::{
    PatchSet, PlatformProfile, ProxyConfig, StealthConfig, validate_profile,
};
use stealth_oxide::profiles::chrome_linux::chrome_linux;
use stealth_oxide::profiles::chrome_windows::chrome_windows;

#[test]
fn builds_a_typed_linux_configuration() -> anyhow::Result<()> {
    let config = StealthConfig::builder()
        .platform(PlatformProfile::Linux)
        .headful(true)
        .mesa(true)
        .patches(PatchSet::all().device_environment(false))
        .build()?;

    assert_eq!(config.profile.name, "chrome-linux");
    assert!(config.launch.headful);
    assert!(config.launch.mesa);
    assert!(!config.patches.device_environment);
    Ok(())
}

#[test]
fn rejects_cross_platform_identity_contradictions() {
    let mut profile = chrome_linux();
    profile.navigator.platform = "Win32".to_string();

    let error = validate_profile(&profile).unwrap_err();
    assert!(error.to_string().contains("disagree"));
}

#[test]
fn rejects_locale_language_contradictions() {
    let mut profile = chrome_windows();
    profile.locale.locale = "fr-FR".to_string();

    let error = validate_profile(&profile).unwrap_err();
    assert!(error.to_string().contains("first navigator language"));
}

#[test]
fn requires_a_profile() {
    assert!(StealthConfig::builder().build().is_err());
}

#[test]
fn customizes_a_preset_without_breaking_coupled_locale_fields() -> anyhow::Result<()> {
    let profile = BrowserProfileBuilder::new(chrome_linux())
        .locale("en-CA")
        .timezone("America/Toronto")
        .screen(2560, 1440)
        .available_screen(2560, 1400)
        .device_scale_factor(1.25)
        .build()?;

    assert_eq!(profile.locale.locale, "en-CA");
    assert_eq!(profile.navigator.languages[0], "en-CA");
    assert_eq!(profile.locale.timezone, "America/Toronto");
    assert_eq!(profile.screen.device_scale_factor, 1.25);
    Ok(())
}

#[test]
fn parses_authenticated_proxy_without_exposing_credentials() -> anyhow::Result<()> {
    let proxy = ProxyConfig::parse("http://alice:secret@127.0.0.1:8080")?;

    assert_eq!(proxy.server, "http://127.0.0.1:8080");
    assert_eq!(proxy.username.as_deref(), Some("alice"));
    assert_eq!(proxy.password.as_deref(), Some("secret"));
    assert!(!proxy.label().contains("secret"));
    Ok(())
}

#[test]
fn rejects_proxy_without_an_explicit_port() {
    assert!(ProxyConfig::parse("http://proxy.example").is_err());
}

#[test]
fn parses_webshare_four_field_proxy_format() -> anyhow::Result<()> {
    let proxy = ProxyConfig::parse("proxy.example:8080:alice:secret")?;

    assert_eq!(proxy.server, "http://proxy.example:8080");
    assert_eq!(proxy.username.as_deref(), Some("alice"));
    assert_eq!(proxy.password.as_deref(), Some("secret"));
    Ok(())
}
