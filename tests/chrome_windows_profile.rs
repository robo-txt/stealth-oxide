use std::collections::HashSet;

use stealth_oxide::profiles::chrome_windows::chrome_windows;

fn chrome_major_version(user_agent: &str) -> &str {
    let chrome = user_agent
        .split_whitespace()
        .find(|part| part.starts_with("Chrome/"))
        .expect("user agent should contain a Chrome version");

    chrome
        .trim_start_matches("Chrome/")
        .split('.')
        .next()
        .expect("Chrome version should contain a major version")
}

#[test]
fn chrome_windows_has_client_hints() {
    let profile = chrome_windows();
    let client_hints = profile
        .navigator
        .client_hints
        .expect("Chrome profile should include user agent client hints");

    assert_eq!(client_hints.platform, "Windows");
    assert_eq!(client_hints.platform_version, "10.0.0");
    assert_eq!(client_hints.architecture, "x86");
    assert_eq!(client_hints.bitness, "64");
    assert_eq!(client_hints.model, "");
    assert!(!client_hints.mobile);
}

#[test]
fn chrome_windows_client_hints_match_user_agent() {
    let profile = chrome_windows();
    let client_hints = profile
        .navigator
        .client_hints
        .expect("Chrome profile should include user agent client hints");
    let major_version = chrome_major_version(&profile.navigator.user_agent);

    assert!(profile.navigator.user_agent.contains("Windows NT 10.0"));
    assert!(profile.navigator.user_agent.contains("Win64; x64"));
    assert_eq!(profile.navigator.platform, "Win32");

    let chrome_brand = client_hints
        .brands
        .iter()
        .find(|brand| brand.brand == "Google Chrome")
        .expect("client hints should include Google Chrome brand");

    assert_eq!(chrome_brand.version, major_version);
}

#[test]
fn chrome_windows_client_hint_brands_are_unique() {
    let profile = chrome_windows();
    let client_hints = profile
        .navigator
        .client_hints
        .expect("Chrome profile should include user agent client hints");

    let mut brands = HashSet::new();
    for brand in &client_hints.brands {
        assert!(
            brands.insert(&brand.brand),
            "duplicate client hint brand: {}",
            brand.brand
        );
    }

    assert_eq!(client_hints.brands.len(), 3);
}

#[test]
fn chrome_windows_full_version_list_matches_brands() {
    let profile = chrome_windows();
    let client_hints = profile
        .navigator
        .client_hints
        .expect("Chrome profile should include user agent client hints");

    let brand_names = client_hints
        .brands
        .iter()
        .map(|brand| brand.brand.as_str())
        .collect::<HashSet<_>>();

    let full_version_brand_names = client_hints
        .full_version_list
        .iter()
        .map(|brand| brand.brand.as_str())
        .collect::<HashSet<_>>();

    assert_eq!(full_version_brand_names, brand_names);

    for brand in &client_hints.full_version_list {
        assert!(
            brand.version.split('.').count() >= 4,
            "full version should include major/minor/build/patch: {}",
            brand.version
        );
    }
}
