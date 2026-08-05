use chromiumoxide::error::CdpError;

use crate::config::Patch;

/// Collection of consistency issues rejected by strict validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationErrors(Vec<ValidationIssue>);

impl ValidationErrors {
    /// Borrows every reported issue.
    pub fn issues(&self) -> &[ValidationIssue] {
        &self.0
    }

    /// Consumes the collection and returns its issues.
    pub fn into_issues(self) -> Vec<ValidationIssue> {
        self.0
    }
}

impl From<Vec<ValidationIssue>> for ValidationErrors {
    fn from(issues: Vec<ValidationIssue>) -> Self {
        Self(issues)
    }
}

impl std::fmt::Display for ValidationErrors {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (index, issue) in self.0.iter().enumerate() {
            if index > 0 {
                formatter.write_str("; ")?;
            }
            std::fmt::Display::fmt(issue, formatter)?;
        }
        Ok(())
    }
}

/// A known consistency or configuration problem.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ValidationIssue {
    /// A required text value is empty.
    #[error("{field} must not be empty")]
    EmptyValue {
        /// Stable field name.
        field: &'static str,
    },
    /// Screen dimensions are invalid.
    #[error("screen dimensions must be greater than zero")]
    InvalidScreenDimensions,
    /// Available work-area dimensions exceed the screen.
    #[error("available screen dimensions cannot exceed screen dimensions")]
    WorkAreaExceedsScreen,
    /// Device scale factor is invalid.
    #[error("device scale factor must be finite and greater than zero")]
    InvalidScaleFactor,
    /// Locale and primary navigator language disagree.
    #[error("the first navigator language must match the Intl locale")]
    LocaleLanguageMismatch,
    /// Touch state and point count disagree.
    #[error("touch configuration is contradictory")]
    TouchContradiction,
    /// User agent, navigator platform, and Client Hints platform disagree.
    #[error("user agent, navigator platform, and UA Client Hints platform disagree")]
    PlatformMismatch,
    /// User-agent and Client Hint Chrome versions disagree.
    #[error("Chrome user-agent and UA Client Hints versions disagree")]
    ChromeVersionMismatch,
    /// A Client Hint brand occurs more than once.
    #[error("duplicate UA Client Hint brand: {brand}")]
    DuplicateBrand {
        /// Duplicated brand name.
        brand: String,
    },
    /// Reduced and full UA Client Hint brand sets disagree.
    #[error("reduced and full UA Client Hint brand sets disagree")]
    ClientHintBrandSetMismatch,
    /// Required Chromium and Google Chrome brand entries are missing.
    #[error("UA Client Hints must include Chromium and Google Chrome brands")]
    MissingChromeBrands,
}

/// Error returned while validating a profile or applying a CDP patch.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A profile seed document contains an invalid value.
    #[cfg(feature = "seeding")]
    #[error("invalid profile seed: {message}")]
    InvalidSeed {
        /// Description of the invalid seed value.
        message: String,
    },

    /// A profile seed could not be decoded from JSON.
    #[cfg(feature = "seeding")]
    #[error("invalid profile seed JSON: {0}")]
    SeedJson(#[from] serde_json::Error),

    /// Strict validation rejected one or more known contradictions.
    #[error("stealth configuration failed validation: {issues}")]
    Validation {
        /// All issues discovered before patch execution.
        issues: ValidationErrors,
    },

    /// A typed CDP parameter builder rejected the supplied profile value.
    #[error("failed to build the {patch} CDP parameters: {message}")]
    InvalidPatchParameters {
        /// Stable name of the patch that failed.
        patch: &'static str,
        /// Error returned by the generated CDP builder.
        message: String,
    },

    /// Chromium rejected a CDP command while applying a patch.
    #[error("failed to apply the {patch} CDP patch: {source}")]
    Cdp {
        /// Stable name of the patch that failed.
        patch: &'static str,
        /// Original Chromiumoxide error.
        #[source]
        source: CdpError,
    },

    /// A patch failed after earlier operations may already have succeeded.
    #[error("failed to apply {patch:?} after applying {applied:?}: {source}")]
    Apply {
        /// Patch that failed.
        patch: Patch,
        /// Patches successfully applied before the failure.
        applied: Vec<Patch>,
        /// Original patch construction or Chromiumoxide error.
        #[source]
        source: Box<Error>,
    },
}

impl Error {
    #[cfg(feature = "seeding")]
    pub(crate) fn invalid_seed(message: impl Into<String>) -> Self {
        Self::InvalidSeed {
            message: message.into(),
        }
    }

    pub(crate) fn invalid_parameters(patch: &'static str, message: impl Into<String>) -> Self {
        Self::InvalidPatchParameters {
            patch,
            message: message.into(),
        }
    }

    pub(crate) fn cdp(patch: &'static str, source: CdpError) -> Self {
        Self::Cdp { patch, source }
    }
}

/// Result type returned by `stealth-oxide`.
pub type Result<T> = std::result::Result<T, Error>;
