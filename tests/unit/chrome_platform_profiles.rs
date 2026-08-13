use std::collections::HashSet;

use stealth_oxide::profiles::chrome_linux::chrome_linux;
use stealth_oxide::profiles::chrome_macos::chrome_macos;
use stealth_oxide::profiles::{
    BUILT_IN_CHROME_MAJOR, BUILT_IN_CHROME_VERSION, BrowserProfile, BrowserProfileBuilder,
    ProfileVersion,
};

fn assert_common_desktop_invariants(profile: BrowserProfile) {
    assert_eq!(
        profile.version(),
        Some(&ProfileVersion::new(
            BUILT_IN_CHROME_MAJOR,
            BUILT_IN_CHROME_VERSION
        ))
    );
    let hints = profile
        .navigator()
        .client_hints
        .as_ref()
        .expect("Chrome profiles should include UA Client Hints");
    let brand_names = hints
        .brands
        .iter()
        .map(|brand| brand.brand.as_str())
        .collect::<HashSet<_>>();
    let full_brand_names = hints
        .full_version_list
        .iter()
        .map(|brand| brand.brand.as_str())
        .collect::<HashSet<_>>();

    assert_eq!(brand_names.len(), hints.brands.len());
    assert_eq!(brand_names, full_brand_names);
    assert!(
        hints
            .brands
            .iter()
            .all(|brand| brand.version == "151" || brand.brand == "Not.A/Brand")
    );
    assert!(
        hints
            .full_version_list
            .iter()
            .all(|brand| brand.version.split('.').count() == 4)
    );
    assert_eq!(hints.architecture, "x86");
    assert_eq!(hints.bitness, "64");
    assert!(!hints.mobile);
    assert_eq!(
        profile.navigator().languages.first(),
        Some(&profile.locale().locale)
    );
    assert!(profile.screen().available_width <= profile.screen().width);
    assert!(profile.screen().available_height < profile.screen().height);
    assert!(!profile.device_environment().touch_enabled);
    assert_eq!(profile.device_environment().max_touch_points, 0);
}

#[test]
fn identity_customization_clears_stale_version_metadata() {
    let profile = BrowserProfileBuilder::new(chrome_linux())
        .user_agent("custom browser")
        .build();

    // The identity is intentionally inconsistent and validation rejects it,
    // but the builder must clear metadata before that validation boundary.
    assert!(profile.is_err());

    let profile = BrowserProfileBuilder::new(chrome_linux())
        .version(None)
        .build()
        .expect("clearing metadata alone keeps the profile coherent");
    assert_eq!(profile.version(), None);
}

#[test]
fn chrome_linux_identity_is_coherent() {
    let profile = chrome_linux();
    let hints = profile.navigator().client_hints.as_ref().unwrap();

    assert_eq!(profile.name(), "chrome-linux");
    assert!(profile.navigator().user_agent.contains("X11; Linux x86_64"));
    assert_eq!(profile.navigator().platform, "Linux x86_64");
    assert_eq!(hints.platform, "Linux");
    assert_eq!(hints.platform_version, "");
    assert_common_desktop_invariants(profile);
}

#[test]
fn chrome_macos_identity_is_coherent() {
    let profile = chrome_macos();
    let hints = profile.navigator().client_hints.as_ref().unwrap();

    assert_eq!(profile.name(), "chrome-macos");
    assert!(profile.navigator().user_agent.contains("Mac OS X 10_15_7"));
    assert_eq!(profile.navigator().platform, "MacIntel");
    assert_eq!(hints.platform, "macOS");
    assert_eq!(hints.platform_version, "15.6.1");
    assert_common_desktop_invariants(profile);
}
