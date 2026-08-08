#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

mod config;
/// Read-only environment observations and consistency checks.
pub mod environment;
mod error;
#[cfg(feature = "interceptor")]
pub mod interceptor;
/// Typed categories for observed Chromium network failures.
pub mod network;
mod patches;
/// Browser profile types, builders, and built-in platform presets.
pub mod profiles;
/// Shared privacy and diagnostic redaction helpers.
pub mod redaction;
#[cfg(feature = "seeding")]
mod seeding;
/// Read-only frame and target topology classification helpers.
pub mod topology;
mod validation;

use chromiumoxide::Page;

pub use config::{
    ApplyReport, ConsistencyPolicy, Patch, PatchMode, PatchPlan, PatchState, PlatformProfile,
    StealthConfig,
};
pub use environment::{
    Finding, FindingSeverity, Observation, check_device_memory, compare_page_worker,
    compare_voice_language, valid_device_memory_bucket,
};
pub use error::{Error, Result, ValidationErrors, ValidationIssue};
pub use network::{
    FailureCategory, classify_failure, increment_failure_count, sanitize_error_name,
};
pub use profiles::{
    BrowserProfile, BrowserProfileBuilder, ColorGamut, ColorScheme, ForcedColors, IdentityConfig,
    MediaFeaturesConfig, ReducedMotion, ScreenConfig, TouchConfig,
};
#[cfg(feature = "seeding")]
pub use seeding::{
    CookieSeed, IndexedDbSeed, OriginSeed, ProfileSeed, SeedReport, apply_profile_seeds,
};
pub use topology::{Coverage, ResourceScope, classify_resource, sanitize_initiator_origin};
pub use validation::validate_profile;

impl StealthConfig {
    /// Validates and applies this configuration to an existing Chromiumoxide page.
    ///
    /// Call this while the page is still `about:blank`, before site scripts run.
    /// Disabled and native patches do not issue CDP commands and are recorded in
    /// the returned report.
    ///
    /// # Errors
    ///
    /// Strict mode returns [`Error::Validation`] before issuing a CDP command.
    /// A later CDP failure can leave earlier patches applied; [`Error::Apply`]
    /// records those completed patches.
    pub async fn apply(&self, page: &Page) -> Result<ApplyReport> {
        let plan = self.plan();
        let issues = plan.issues().to_vec();
        if self.policy() == ConsistencyPolicy::Strict && !issues.is_empty() {
            return Err(Error::Validation {
                issues: issues.into(),
            });
        }

        let mut report = ApplyReport::default();
        if self.policy() == ConsistencyPolicy::Warn {
            report.warnings = issues;
        }

        apply_mode(self.locale_mode(), Patch::Locale, &mut report, |value| {
            patches::locale::apply(page, value)
        })
        .await?;
        apply_mode(
            self.timezone_mode(),
            Patch::Timezone,
            &mut report,
            |value| patches::timezone::apply(page, value),
        )
        .await?;
        apply_mode(
            self.identity_mode(),
            Patch::Identity,
            &mut report,
            |value| patches::identity::apply(page, value),
        )
        .await?;
        apply_mode(self.screen_mode(), Patch::Screen, &mut report, |value| {
            patches::screen::apply(page, value)
        })
        .await?;
        apply_mode(
            self.media_features_mode(),
            Patch::MediaFeatures,
            &mut report,
            |value| patches::media_features::apply(page, value),
        )
        .await?;
        apply_mode(self.touch_mode(), Patch::Touch, &mut report, |value| {
            patches::touch::apply(page, value)
        })
        .await?;

        Ok(report)
    }
}

async fn apply_mode<'a, T, F, Fut>(
    mode: &'a PatchMode<T>,
    patch: Patch,
    report: &mut ApplyReport,
    apply: F,
) -> Result<()>
where
    F: FnOnce(&'a T) -> Fut,
    Fut: std::future::Future<Output = Result<()>>,
{
    match mode {
        PatchMode::Native => report.native.push(patch),
        PatchMode::Disabled => report.skipped.push(patch),
        PatchMode::Override(value) => match apply(value).await {
            Ok(()) => report.applied.push(patch),
            Err(source) => {
                return Err(Error::Apply {
                    patch,
                    applied: report.applied.clone(),
                    source: Box::new(source),
                });
            }
        },
    }
    Ok(())
}
