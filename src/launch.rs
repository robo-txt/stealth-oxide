//! Chrome startup values that must be configured before CDP is available.

use serde_json::{Value, json};

use crate::profiles::BrowserProfile;

/// Browser-native language settings for a persistent Chrome profile.
///
/// Apply the preference patch to a stopped browser profile before launch and
/// pass [`Self::chrome_argument`] to Chromiumoxide's `BrowserConfigBuilder`.
/// The type performs no filesystem writes and does not overwrite an existing
/// Preferences file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChromeLanguageConfig {
    application_locale: String,
    selected_languages: Vec<String>,
}

impl ChromeLanguageConfig {
    /// Derives startup language settings from a validated browser profile.
    pub fn from_profile(profile: &BrowserProfile) -> Self {
        Self {
            application_locale: profile.locale().locale.clone(),
            selected_languages: profile.navigator().languages.clone(),
        }
    }

    /// Returns Chrome's native `--lang=<locale>` launch argument.
    pub fn chrome_argument(&self) -> (&'static str, &str) {
        ("lang", &self.application_locale)
    }

    /// Returns the primary application locale.
    pub fn application_locale(&self) -> &str {
        &self.application_locale
    }

    /// Returns the ordered user-selected languages.
    pub fn selected_languages(&self) -> &[String] {
        &self.selected_languages
    }

    /// Returns a minimal object to merge into the profile's `Preferences` JSON
    /// before Chrome starts.
    ///
    /// `intl.accept_languages` is intentionally absent. Chromium documents it
    /// as a derived preference that should be updated from
    /// `intl.selected_languages` rather than written directly.
    pub fn preference_patch(&self) -> Value {
        json!({
            "intl": {
                "app_locale": self.application_locale,
                "selected_languages": self.selected_languages.join(",")
            }
        })
    }

    /// Returns the ordered value used by the target-scoped CDP fallback.
    pub fn cdp_accept_language(&self) -> String {
        self.selected_languages.join(",")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PlatformProfile;

    #[test]
    fn derives_native_language_values_without_setting_derived_pref() {
        let config = ChromeLanguageConfig::from_profile(&PlatformProfile::Linux.profile());

        assert_eq!(config.chrome_argument(), ("lang", "en-US"));
        assert_eq!(config.cdp_accept_language(), "en-US,en");
        assert_eq!(
            config.preference_patch()["intl"]["selected_languages"],
            "en-US,en"
        );
        assert!(config.preference_patch()["intl"]["accept_languages"].is_null());
    }
}
