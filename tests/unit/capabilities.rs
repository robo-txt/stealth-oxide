use stealth_oxide::{
    CapabilityExpectation, NativeCapability, NativeCapabilityObservation, PlatformProfile,
};

#[test]
fn linux_desktop_capabilities_are_not_expected() {
    let expectations = PlatformProfile::Linux.native_capability_expectations();

    assert_eq!(expectations.web_share, CapabilityExpectation::NotExpected);
    assert_eq!(
        expectations.contacts_manager,
        CapabilityExpectation::NotExpected
    );
    assert_eq!(
        expectations.content_index,
        CapabilityExpectation::NotExpected
    );
    assert_eq!(
        expectations.network_information_downlink_max,
        CapabilityExpectation::NotExpected
    );
}

#[test]
fn native_windows_and_macos_expect_web_share_only() {
    for platform in [PlatformProfile::Windows, PlatformProfile::MacOS] {
        let expectations = platform.native_capability_expectations();

        assert_eq!(expectations.web_share, CapabilityExpectation::Expected);
        assert_eq!(
            expectations.contacts_manager,
            CapabilityExpectation::NotExpected
        );
        assert_eq!(
            expectations.content_index,
            CapabilityExpectation::NotExpected
        );
        assert_eq!(
            expectations.network_information_downlink_max,
            CapabilityExpectation::NotExpected
        );
    }
}

#[test]
fn capability_comparison_reports_only_native_mismatches() {
    let expectations = PlatformProfile::Windows.native_capability_expectations();
    let observed = NativeCapabilityObservation::new(false, false, false, false);
    let mismatches = expectations.mismatches(observed);

    assert_eq!(mismatches.len(), 1);
    assert_eq!(mismatches[0].capability, NativeCapability::WebShare);
    assert_eq!(mismatches[0].expectation, CapabilityExpectation::Expected);
    assert!(!mismatches[0].observed);
}

#[test]
fn linux_observation_matches_desktop_absence() {
    let expectations = PlatformProfile::Linux.native_capability_expectations();
    let observed = NativeCapabilityObservation::new(false, false, false, false);

    assert!(expectations.mismatches(observed).is_empty());
}
