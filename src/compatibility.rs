//! Browser runtime and profile-version compatibility inspection.

use crate::profiles::ProfileVersion;

/// Result of comparing a profile identity with a Chromium product string.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CompatibilityStatus {
    /// Runtime and profile use the same Chrome major version.
    Compatible {
        /// Shared Chrome major version.
        chrome_major: u32,
    },
    /// Runtime and profile advertise different Chrome major versions.
    MajorMismatch {
        /// Major version represented by the profile.
        profile_major: u32,
        /// Major version reported by the runtime.
        runtime_major: u32,
    },
    /// The custom profile does not declare browser version metadata.
    UnknownProfileVersion,
    /// The runtime product string does not contain a recognized version.
    UnknownRuntimeVersion,
}

/// Compares optional profile metadata with a CDP `Browser.getVersion` product string.
///
/// Recognized examples include `Chrome/150.0.7871.128`,
/// `HeadlessChrome/150.0.7871.128`, and `Chromium/150.0.7871.128`.
pub fn compare_browser_versions(
    profile: Option<&ProfileVersion>,
    runtime_product: &str,
) -> CompatibilityStatus {
    let Some(profile) = profile else {
        return CompatibilityStatus::UnknownProfileVersion;
    };
    let Some(runtime_major) = runtime_chrome_major(runtime_product) else {
        return CompatibilityStatus::UnknownRuntimeVersion;
    };

    if profile.chrome_major == runtime_major {
        CompatibilityStatus::Compatible {
            chrome_major: runtime_major,
        }
    } else {
        CompatibilityStatus::MajorMismatch {
            profile_major: profile.chrome_major,
            runtime_major,
        }
    }
}

fn runtime_chrome_major(product: &str) -> Option<u32> {
    let (name, version) = product.trim().split_once('/')?;
    if !matches!(name, "Chrome" | "HeadlessChrome" | "Chromium") {
        return None;
    }
    version.split('.').next()?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_supported_chromium_product_names() {
        for product in [
            "Chrome/150.0.7871.128",
            "HeadlessChrome/150.0.7871.128",
            "Chromium/150.0.7871.128",
        ] {
            assert_eq!(
                compare_browser_versions(Some(&ProfileVersion::built_in()), product),
                CompatibilityStatus::Compatible { chrome_major: 150 }
            );
        }
    }

    #[test]
    fn reports_mismatch_and_unknown_versions() {
        assert_eq!(
            compare_browser_versions(Some(&ProfileVersion::built_in()), "Chrome/152.0.0.0"),
            CompatibilityStatus::MajorMismatch {
                profile_major: 150,
                runtime_major: 152,
            }
        );
        assert_eq!(
            compare_browser_versions(Some(&ProfileVersion::built_in()), "unknown"),
            CompatibilityStatus::UnknownRuntimeVersion
        );
        assert_eq!(
            compare_browser_versions(None, "Chrome/150.0.7871.128"),
            CompatibilityStatus::UnknownProfileVersion
        );
    }
}
