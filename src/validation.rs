use std::collections::HashSet;

use crate::config::{PatchMode, StealthConfig};
use crate::error::{Error, Result, ValidationIssue};
use crate::profiles::{BrowserProfile, NavigatorProfile, ScreenConfig};

pub(crate) fn validate_config(config: &StealthConfig) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    if let PatchMode::Override(identity) = config.identity_mode() {
        validate_identity(identity, &mut issues);
    }
    if let PatchMode::Override(locale) = config.locale_mode() {
        if locale.trim().is_empty() {
            issues.push(ValidationIssue::EmptyValue { field: "locale" });
        }
        if let PatchMode::Override(identity) = config.identity_mode() {
            if identity.languages.first() != Some(locale) {
                issues.push(ValidationIssue::LocaleLanguageMismatch);
            }
        }
    }
    if let PatchMode::Override(timezone) = config.timezone_mode() {
        if timezone.trim().is_empty() {
            issues.push(ValidationIssue::EmptyValue { field: "timezone" });
        }
    }
    if let PatchMode::Override(screen) = config.screen_mode() {
        validate_screen(screen, &mut issues);
    }
    if let PatchMode::Override(touch) = config.touch_mode() {
        if (!touch.enabled && touch.max_touch_points != 0)
            || (touch.enabled && touch.max_touch_points == 0)
        {
            issues.push(ValidationIssue::TouchContradiction);
        }
    }
    if let PatchMode::Override(hardware_concurrency) = config.hardware_concurrency_mode()
        && *hardware_concurrency == 0
    {
        issues.push(ValidationIssue::InvalidHardwareConcurrency);
    }

    issues
}

fn validate_screen(screen: &ScreenConfig, issues: &mut Vec<ValidationIssue>) {
    if screen.width == 0 || screen.height == 0 {
        issues.push(ValidationIssue::InvalidScreenDimensions);
    }
    if !screen.device_scale_factor.is_finite() || screen.device_scale_factor <= 0.0 {
        issues.push(ValidationIssue::InvalidScaleFactor);
    }
}

fn validate_identity(identity: &NavigatorProfile, issues: &mut Vec<ValidationIssue>) {
    if identity.user_agent.trim().is_empty() {
        issues.push(ValidationIssue::EmptyValue {
            field: "user agent",
        });
    }
    if identity.platform.trim().is_empty() {
        issues.push(ValidationIssue::EmptyValue {
            field: "navigator platform",
        });
    }
    if identity.languages.is_empty()
        || identity
            .languages
            .iter()
            .any(|value| value.trim().is_empty())
    {
        issues.push(ValidationIssue::EmptyValue {
            field: "navigator languages",
        });
    }

    let Some(hints) = &identity.client_hints else {
        return;
    };
    let platform_is_consistent = match hints.platform.as_str() {
        "Linux" => identity.user_agent.contains("Linux") && identity.platform.contains("Linux"),
        "Windows" => identity.user_agent.contains("Windows") && identity.platform == "Win32",
        "macOS" => identity.user_agent.contains("Mac OS X") && identity.platform == "MacIntel",
        _ => false,
    };
    if !platform_is_consistent {
        issues.push(ValidationIssue::PlatformMismatch);
    }

    validate_brands(identity, hints, issues);
    validate_platform_metadata(hints, issues);
}

fn validate_platform_metadata(
    hints: &crate::profiles::UserAgentClientHintsProfile,
    issues: &mut Vec<ValidationIssue>,
) {
    let is_windows = hints.platform == "Windows";
    let is_x86_32 = hints.architecture == "x86" && hints.bitness == "32";
    if hints.wow64 == Some(true) && !(is_windows && is_x86_32) {
        issues.push(ValidationIssue::InvalidWow64);
    }
    if !matches!(hints.architecture.as_str(), "x86" | "arm" | "arm64")
        || !matches!(hints.bitness.as_str(), "32" | "64")
    {
        issues.push(ValidationIssue::InvalidArchitectureBitness);
    }
    if !hints.mobile && !hints.model.is_empty() {
        issues.push(ValidationIssue::MobileMetadataMismatch);
    }
    if let Some(form_factors) = &hints.form_factors {
        let has_mobile_factor = form_factors
            .iter()
            .any(|factor| matches!(factor.as_str(), "Mobile" | "Tablet"));
        let has_desktop_factor = form_factors.iter().any(|factor| factor == "Desktop");
        if hints.mobile != has_mobile_factor || (!hints.mobile && !has_desktop_factor) {
            issues.push(ValidationIssue::MobileMetadataMismatch);
        }
    }
    let version_valid = match hints.platform.as_str() {
        "Linux" => hints.platform_version.is_empty(),
        "Windows" | "macOS" => {
            let parts = hints.platform_version.split('.').collect::<Vec<_>>();
            parts.len() == 3
                && parts
                    .iter()
                    .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()))
        }
        _ => false,
    };
    if !version_valid {
        issues.push(ValidationIssue::InvalidPlatformVersion);
    }
}

fn validate_brands(
    identity: &NavigatorProfile,
    hints: &crate::profiles::UserAgentClientHintsProfile,
    issues: &mut Vec<ValidationIssue>,
) {
    let mut seen = HashSet::new();
    for brand in &hints.brands {
        if !seen.insert(&brand.brand) {
            issues.push(ValidationIssue::DuplicateBrand {
                brand: brand.brand.clone(),
            });
        }
    }
    seen.clear();
    for brand in &hints.full_version_list {
        if !seen.insert(&brand.brand) {
            issues.push(ValidationIssue::DuplicateBrand {
                brand: brand.brand.clone(),
            });
        }
    }

    let reduced_brands = hints
        .brands
        .iter()
        .map(|brand| brand.brand.as_str())
        .collect::<HashSet<_>>();
    let full_brands = hints
        .full_version_list
        .iter()
        .map(|brand| brand.brand.as_str())
        .collect::<HashSet<_>>();
    if reduced_brands != full_brands {
        issues.push(ValidationIssue::ClientHintBrandSetMismatch);
    }
    if !reduced_brands.contains("Chromium") || !reduced_brands.contains("Google Chrome") {
        issues.push(ValidationIssue::MissingChromeBrands);
    }

    let ua_version = identity
        .user_agent
        .split_whitespace()
        .find_map(|part| part.strip_prefix("Chrome/"));
    let versions_match = ua_version.is_some_and(|ua_version| {
        let ua_major = ua_version.split('.').next().unwrap_or_default();
        hints
            .brands
            .iter()
            .filter(|brand| brand.brand == "Chromium" || brand.brand == "Google Chrome")
            .all(|brand| brand.version == ua_major)
            && hints
                .full_version_list
                .iter()
                .filter(|brand| brand.brand == "Chromium" || brand.brand == "Google Chrome")
                .all(|brand| brand.version == ua_version)
    });
    if !versions_match {
        issues.push(ValidationIssue::ChromeVersionMismatch);
    }
}

/// Validates all relationships in an editable browser profile.
///
/// # Errors
///
/// Returns [`Error::Validation`] containing every discovered issue.
pub fn validate_profile(profile: &BrowserProfile) -> Result<()> {
    let mut issues = validate_config(&StealthConfig::from_profile(profile.clone()));
    if profile.screen.available_width > profile.screen.width
        || profile.screen.available_height > profile.screen.height
    {
        issues.push(ValidationIssue::WorkAreaExceedsScreen);
    }
    if issues.is_empty() {
        Ok(())
    } else {
        Err(Error::Validation {
            issues: issues.into(),
        })
    }
}
