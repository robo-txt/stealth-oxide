//! Read-only host/browser environment observations and consistency checks.

/// Availability state for an observed browser surface.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Observation<T> {
    /// The browser does not expose this API.
    Unsupported,
    /// The API exists, but no usable value was available.
    Unavailable,
    /// The probe exceeded its caller-provided deadline.
    TimedOut,
    /// A value was observed without modifying the browser surface.
    Observed(T),
}

/// Severity for a read-only audit finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FindingSeverity {
    /// Informational observation or unsupported optional API.
    Info,
    /// A value deserves attention but is not inherently contradictory.
    Warning,
    /// Two observed surfaces contradict one another.
    Contradiction,
}

/// A stable, human-readable audit finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// Finding severity.
    pub severity: FindingSeverity,
    /// Stable finding identifier.
    pub code: &'static str,
    /// Short explanation without sensitive values.
    pub message: &'static str,
}

/// Returns whether a `navigator.deviceMemory` value is one of Chromium's buckets.
pub fn valid_device_memory_bucket(value: f64) -> bool {
    [0.25, 0.5, 1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 64.0]
        .iter()
        .any(|bucket| (value - bucket).abs() < f64::EPSILON)
}

/// Checks the optional heap-limit relationship without requiring either API.
pub fn check_device_memory(
    device_memory: &Observation<f64>,
    heap_limit: &Observation<f64>,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    if let Observation::Observed(memory) = device_memory {
        if !valid_device_memory_bucket(*memory) {
            findings.push(Finding {
                severity: FindingSeverity::Contradiction,
                code: "invalid-device-memory-bucket",
                message: "device memory is outside Chromium's documented bucket values",
            });
        }
        if let Observation::Observed(heap) = heap_limit {
            if *heap <= *memory {
                findings.push(Finding {
                    severity: FindingSeverity::Contradiction,
                    code: "heap-limit-not-greater",
                    message: "JavaScript heap limit is not greater than reported device memory",
                });
            }
        }
    }
    findings
}

/// Compares a page and worker value, treating unavailable optional APIs as evidence gaps.
pub fn compare_page_worker<T: PartialEq>(
    page: &Observation<T>,
    worker: &Observation<T>,
    code: &'static str,
    message: &'static str,
) -> Option<Finding> {
    match (page, worker) {
        (Observation::Observed(page), Observation::Observed(worker)) if page != worker => {
            Some(Finding {
                severity: FindingSeverity::Contradiction,
                code,
                message,
            })
        }
        _ => None,
    }
}

/// Compares a reported voice language to the configured primary language.
pub fn compare_voice_language(
    voice_language: &Observation<String>,
    configured_language: &str,
) -> Option<Finding> {
    let Observation::Observed(language) = voice_language else {
        return None;
    };
    let voice_prefix = language.split('-').next().unwrap_or(language);
    let configured_prefix = configured_language
        .split('-')
        .next()
        .unwrap_or(configured_language);
    (voice_prefix != configured_prefix).then_some(Finding {
        severity: FindingSeverity::Warning,
        code: "voice-language-locale-mismatch",
        message: "the default speech voice language differs from the configured locale",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_known_memory_buckets() {
        assert!(valid_device_memory_bucket(8.0));
        assert!(!valid_device_memory_bucket(6.0));
    }

    #[test]
    fn reports_memory_contradictions() {
        let findings =
            check_device_memory(&Observation::Observed(6.0), &Observation::Observed(4.0));
        assert_eq!(findings.len(), 2);
    }

    #[test]
    fn compares_only_observed_values() {
        assert!(
            compare_page_worker(
                &Observation::Observed("Linux"),
                &Observation::Observed("Windows"),
                "platform-mismatch",
                "page and worker platforms disagree",
            )
            .is_some()
        );
        assert!(
            compare_page_worker::<String>(
                &Observation::Unsupported,
                &Observation::Observed("Windows".into()),
                "platform-mismatch",
                "page and worker platforms disagree",
            )
            .is_none()
        );
    }

    #[test]
    fn compares_voice_language_by_language_family() {
        assert!(compare_voice_language(&Observation::Observed("en-GB".into()), "en-US").is_none());
        assert!(compare_voice_language(&Observation::Observed("fr-FR".into()), "en-US").is_some());
    }
}
