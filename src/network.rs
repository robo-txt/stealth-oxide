//! Stable classification for Chromium network loading failures.

use std::collections::BTreeMap;

/// Broad, intentionally non-exhaustive families of loading failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum FailureCategory {
    /// The request was cancelled before completion.
    Cancelled,
    /// Chromium blocked the request due to browser policy, content blocking, or CORS.
    BrowserPolicy,
    /// DNS/name resolution failed.
    Dns,
    /// A proxy or CONNECT tunnel failed.
    Proxy,
    /// The peer connection was refused, reset, closed, or otherwise unavailable.
    Connection,
    /// TLS, certificate, or secure-channel negotiation failed.
    Tls,
    /// HTTP/2 framing or protocol negotiation failed.
    Http2,
    /// HTTP/3, QUIC, or QUIC transport negotiation failed.
    Http3,
    /// The request exceeded a network timeout.
    Timeout,
    /// Chromium reported a failure that does not fit a known family.
    Unknown,
}

impl std::fmt::Display for FailureCategory {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Cancelled => "cancelled",
            Self::BrowserPolicy => "browser-policy",
            Self::Dns => "dns",
            Self::Proxy => "proxy",
            Self::Connection => "connection",
            Self::Tls => "tls",
            Self::Http2 => "http2",
            Self::Http3 => "http3",
            Self::Timeout => "timeout",
            Self::Unknown => "unknown",
        };
        formatter.write_str(name)
    }
}

/// Classifies a Chromium `Network.loadingFailed` error without suppressing its name.
pub fn classify_failure(
    error_text: &str,
    cancelled: bool,
    blocked_reason: Option<&str>,
    cors_error: bool,
) -> FailureCategory {
    if cancelled {
        return FailureCategory::Cancelled;
    }
    if blocked_reason.is_some() || cors_error {
        return FailureCategory::BrowserPolicy;
    }

    let error = error_text.to_ascii_uppercase();
    let error = error.strip_prefix("NET::").unwrap_or(&error);
    if error.contains("NAME_NOT_RESOLVED")
        || error.contains("DNS_")
        || error.contains("ADDRESS_UNREACHABLE")
    {
        FailureCategory::Dns
    } else if error.contains("PROXY") || error.contains("TUNNEL") {
        FailureCategory::Proxy
    } else if error.contains("CERT_")
        || error.contains("SSL")
        || error.contains("TLS")
        || error.contains("BAD_SECURITY")
    {
        FailureCategory::Tls
    } else if error.contains("HTTP2") || error.contains("SPDY") {
        FailureCategory::Http2
    } else if error.contains("QUIC") || error.contains("HTTP3") {
        FailureCategory::Http3
    } else if error.contains("TIMED_OUT") || error.contains("TIMEOUT") {
        FailureCategory::Timeout
    } else if error.contains("CONNECTION_")
        || error.contains("CONNECTION_REFUSED")
        || error.contains("CONNECTION_RESET")
        || error.contains("CONNECTION_CLOSED")
        || error.contains("NETWORK_CHANGED")
    {
        FailureCategory::Connection
    } else {
        FailureCategory::Unknown
    }
}

/// Removes transport prefixes and detail text while retaining a useful error name.
pub fn sanitize_error_name(error_text: &str) -> String {
    let trimmed = error_text.trim();
    let name = trimmed
        .strip_prefix("net::")
        .or_else(|| trimmed.strip_prefix("NET::"))
        .unwrap_or(trimmed);
    name.split_whitespace()
        .next()
        .unwrap_or("UNKNOWN")
        .to_string()
}

/// Adds one failure to a category-count map.
pub fn increment_failure_count(counts: &mut BTreeMap<String, usize>, category: FailureCategory) {
    *counts.entry(category.to_string()).or_default() += 1;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_known_families() {
        assert_eq!(
            classify_failure("net::ERR_NAME_NOT_RESOLVED", false, None, false),
            FailureCategory::Dns
        );
        assert_eq!(
            classify_failure("net::ERR_PROXY_CONNECTION_FAILED", false, None, false),
            FailureCategory::Proxy
        );
        assert_eq!(
            classify_failure("net::ERR_CERT_AUTHORITY_INVALID", false, None, false),
            FailureCategory::Tls
        );
        assert_eq!(
            classify_failure("net::ERR_HTTP2_PROTOCOL_ERROR", false, None, false),
            FailureCategory::Http2
        );
        assert_eq!(
            classify_failure("net::ERR_QUIC_PROTOCOL_ERROR", false, None, false),
            FailureCategory::Http3
        );
        assert_eq!(
            classify_failure("net::ERR_TIMED_OUT", false, None, false),
            FailureCategory::Timeout
        );
    }

    #[test]
    fn policy_and_cancellation_take_precedence() {
        assert_eq!(
            classify_failure("net::ERR_FAILED", true, None, false),
            FailureCategory::Cancelled
        );
        assert_eq!(
            classify_failure("net::ERR_FAILED", false, Some("blockedByClient"), false),
            FailureCategory::BrowserPolicy
        );
        assert_eq!(
            classify_failure("net::ERR_FAILED", false, None, true),
            FailureCategory::BrowserPolicy
        );
    }

    #[test]
    fn unknowns_remain_observable() {
        assert_eq!(
            classify_failure("net::ERR_NEW_CHROMIUM_ERROR", false, None, false),
            FailureCategory::Unknown
        );
        assert_eq!(
            sanitize_error_name("net::ERR_FAILED details omitted"),
            "ERR_FAILED"
        );
    }
}
