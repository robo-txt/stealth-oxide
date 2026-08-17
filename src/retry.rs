//! Safe, opt-in decisions for retrying top-level browser navigations.
//!
//! This module never sends a request. Chrome remains responsible for transport
//! retries, redirects, subresources, cache behavior, service workers, and
//! connection reuse. Callers may use a [`RetryDecision::RetryAfter`] result to
//! repeat an eligible top-level navigation on the same page and profile.

use std::time::{Duration, SystemTime};

/// Conservative request-method classification for automatic navigation retries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum NavigationMethod {
    /// HTTP `GET`.
    Get,
    /// HTTP `HEAD`.
    Head,
    /// HTTP `OPTIONS`.
    Options,
    /// Any method that is not automatically replayed.
    UnsafeOrUnknown,
}

impl NavigationMethod {
    /// Classifies a method without retaining the original text.
    pub fn classify(value: &str) -> Self {
        if value.eq_ignore_ascii_case("GET") {
            Self::Get
        } else if value.eq_ignore_ascii_case("HEAD") {
            Self::Head
        } else if value.eq_ignore_ascii_case("OPTIONS") {
            Self::Options
        } else {
            Self::UnsafeOrUnknown
        }
    }

    /// Returns whether the method is eligible for an automatic replay.
    pub const fn automatically_replayable(self) -> bool {
        !matches!(self, Self::UnsafeOrUnknown)
    }
}

/// High-level handling category for a main-document HTTP response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ResponseDisposition {
    /// Chrome should continue handling the response normally.
    BrowserManaged,
    /// The origin requires authorized credentials.
    AuthenticationRequired,
    /// Access was denied and must not be retried automatically.
    AccessDenied,
    /// The origin applied a rate limit.
    RateLimited,
    /// The origin is temporarily unavailable.
    TemporarilyUnavailable,
}

/// Classifies a main-document HTTP status without interpreting page content.
pub const fn classify_response(status: u16) -> ResponseDisposition {
    match status {
        401 => ResponseDisposition::AuthenticationRequired,
        403 => ResponseDisposition::AccessDenied,
        429 => ResponseDisposition::RateLimited,
        503 => ResponseDisposition::TemporarilyUnavailable,
        _ => ResponseDisposition::BrowserManaged,
    }
}

/// Reason an application-level navigation retry was declined.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum NoRetryReason {
    /// The response is not eligible for application-level retry handling.
    IneligibleStatus,
    /// The request method is unsafe or unknown.
    UnsafeOrUnknownMethod,
    /// The configured attempt budget has been exhausted.
    AttemptBudgetExhausted,
    /// The configured elapsed-time budget has been exhausted.
    TimeBudgetExhausted,
    /// The proposed delay would exceed the configured elapsed-time budget.
    DelayExceedsTimeBudget,
}

/// Decision returned by [`NavigationRetryPolicy::decide`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RetryDecision {
    /// Repeat the eligible top-level navigation after this delay.
    RetryAfter(Duration),
    /// Do not automatically repeat the navigation.
    DoNotRetry(NoRetryReason),
}

/// Bounded policy for eligible top-level navigation retries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NavigationRetryPolicy {
    max_attempts: u32,
    max_elapsed: Duration,
    base_delay: Duration,
    max_delay: Duration,
}

impl Default for NavigationRetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            max_elapsed: Duration::from_secs(120),
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(30),
        }
    }
}

impl NavigationRetryPolicy {
    /// Creates the conservative default policy.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the total number of permitted navigation attempts, including the first.
    pub const fn max_attempts(mut self, value: u32) -> Self {
        self.max_attempts = value;
        self
    }

    /// Sets the maximum elapsed time across all attempts and delays.
    pub const fn max_elapsed(mut self, value: Duration) -> Self {
        self.max_elapsed = value;
        self
    }

    /// Sets the initial exponential-backoff delay.
    pub const fn base_delay(mut self, value: Duration) -> Self {
        self.base_delay = value;
        self
    }

    /// Caps server-provided and locally calculated delays.
    pub const fn max_delay(mut self, value: Duration) -> Self {
        self.max_delay = value;
        self
    }

    /// Returns a retry decision for a completed top-level navigation.
    ///
    /// `attempt` is one-based and identifies the attempt that produced the
    /// response. `jitter` must be between `0.0` and `1.0`; values outside that
    /// range are clamped. Jitter is applied only to locally calculated backoff,
    /// never to an origin's `Retry-After` instruction.
    #[allow(clippy::too_many_arguments)]
    pub fn decide(
        &self,
        status: u16,
        method: NavigationMethod,
        retry_after: Option<&str>,
        attempt: u32,
        elapsed: Duration,
        now: SystemTime,
        jitter: f64,
    ) -> RetryDecision {
        if !matches!(status, 429 | 503) {
            return RetryDecision::DoNotRetry(NoRetryReason::IneligibleStatus);
        }
        if !method.automatically_replayable() {
            return RetryDecision::DoNotRetry(NoRetryReason::UnsafeOrUnknownMethod);
        }
        if attempt >= self.max_attempts {
            return RetryDecision::DoNotRetry(NoRetryReason::AttemptBudgetExhausted);
        }
        if elapsed >= self.max_elapsed {
            return RetryDecision::DoNotRetry(NoRetryReason::TimeBudgetExhausted);
        }

        let delay = retry_after
            .and_then(|value| parse_retry_after(value, now))
            .unwrap_or_else(|| self.backoff(attempt, jitter))
            .min(self.max_delay);
        if elapsed.saturating_add(delay) > self.max_elapsed {
            return RetryDecision::DoNotRetry(NoRetryReason::DelayExceedsTimeBudget);
        }
        RetryDecision::RetryAfter(delay)
    }

    fn backoff(&self, attempt: u32, jitter: f64) -> Duration {
        let exponent = attempt.saturating_sub(1).min(31);
        let nominal = self
            .base_delay
            .saturating_mul(1_u32.checked_shl(exponent).unwrap_or(u32::MAX))
            .min(self.max_delay);
        let factor = 0.5 + (jitter.clamp(0.0, 1.0) * 0.5);
        Duration::from_secs_f64(nominal.as_secs_f64() * factor)
    }
}

/// Parses either form of the HTTP `Retry-After` header.
///
/// Delta-seconds are measured from receipt. HTTP dates in the past produce a
/// zero delay. Invalid values return `None` so policy backoff can be used.
pub fn parse_retry_after(value: &str, now: SystemTime) -> Option<Duration> {
    let value = value.trim();
    if value.bytes().all(|byte| byte.is_ascii_digit()) && !value.is_empty() {
        return value.parse::<u64>().ok().map(Duration::from_secs);
    }
    let deadline = httpdate::parse_http_date(value).ok()?;
    Some(deadline.duration_since(now).unwrap_or(Duration::ZERO))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_only_conservative_methods_as_replayable() {
        assert_eq!(NavigationMethod::classify("GET"), NavigationMethod::Get);
        assert!(NavigationMethod::classify("head").automatically_replayable());
        for method in ["POST", "PUT", "PATCH", "DELETE", ""] {
            assert!(!NavigationMethod::classify(method).automatically_replayable());
        }
    }

    #[test]
    fn classifies_challenge_relevant_statuses() {
        assert_eq!(classify_response(302), ResponseDisposition::BrowserManaged);
        assert_eq!(
            classify_response(401),
            ResponseDisposition::AuthenticationRequired
        );
        assert_eq!(classify_response(403), ResponseDisposition::AccessDenied);
        assert_eq!(classify_response(429), ResponseDisposition::RateLimited);
        assert_eq!(
            classify_response(503),
            ResponseDisposition::TemporarilyUnavailable
        );
    }

    #[test]
    fn parses_delta_and_http_date_retry_after_values() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(784_111_777);
        assert_eq!(parse_retry_after("15", now), Some(Duration::from_secs(15)));
        assert_eq!(
            parse_retry_after("Sun, 06 Nov 1994 08:49:52 GMT", now),
            Some(Duration::from_secs(15))
        );
        assert_eq!(parse_retry_after("not-a-date", now), None);
    }

    #[test]
    fn honors_server_delay_and_never_retries_access_denied() {
        let policy = NavigationRetryPolicy::new();
        let now = SystemTime::UNIX_EPOCH;
        assert_eq!(
            policy.decide(
                429,
                NavigationMethod::Get,
                Some("9"),
                1,
                Duration::ZERO,
                now,
                0.0,
            ),
            RetryDecision::RetryAfter(Duration::from_secs(9))
        );
        assert_eq!(
            policy.decide(
                403,
                NavigationMethod::Get,
                None,
                1,
                Duration::ZERO,
                now,
                0.0,
            ),
            RetryDecision::DoNotRetry(NoRetryReason::IneligibleStatus)
        );
    }

    #[test]
    fn enforces_method_attempt_and_elapsed_budgets() {
        let policy = NavigationRetryPolicy::new()
            .max_attempts(2)
            .max_elapsed(Duration::from_secs(10));
        let now = SystemTime::UNIX_EPOCH;
        assert_eq!(
            policy.decide(
                503,
                NavigationMethod::UnsafeOrUnknown,
                None,
                1,
                Duration::ZERO,
                now,
                0.5
            ),
            RetryDecision::DoNotRetry(NoRetryReason::UnsafeOrUnknownMethod)
        );
        assert_eq!(
            policy.decide(
                503,
                NavigationMethod::Get,
                None,
                2,
                Duration::ZERO,
                now,
                0.5
            ),
            RetryDecision::DoNotRetry(NoRetryReason::AttemptBudgetExhausted)
        );
        assert_eq!(
            policy.decide(
                503,
                NavigationMethod::Get,
                Some("10"),
                1,
                Duration::from_secs(1),
                now,
                0.5,
            ),
            RetryDecision::DoNotRetry(NoRetryReason::DelayExceedsTimeBudget)
        );
    }
}
