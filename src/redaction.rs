//! Shared privacy rules for diagnostic reports.

use std::collections::BTreeMap;

/// Maximum length for a diagnostic string.
pub const MAX_STRING_LENGTH: usize = 512;
/// Maximum number of entries retained in a diagnostic collection.
pub const MAX_COLLECTION_ITEMS: usize = 100;

/// Bounds a diagnostic string without exposing unbounded response/error text.
pub fn bounded(value: &str) -> String {
    value.chars().take(MAX_STRING_LENGTH).collect()
}

/// Removes user information, query parameters, and fragments from an HTTP URL.
pub fn url(value: &str) -> String {
    let Ok(mut parsed) = url::Url::parse(value) else {
        return "<non-url>".to_string();
    };
    if !matches!(parsed.scheme(), "http" | "https") {
        return format!("<{}-url>", parsed.scheme());
    }
    let _ = parsed.set_username("");
    let _ = parsed.set_password(None);
    parsed.set_query(None);
    parsed.set_fragment(None);
    bounded(parsed.as_str())
}

/// Keeps only explicitly safe response headers and bounds their values.
pub fn safe_headers(value: &serde_json::Value) -> BTreeMap<String, String> {
    const ALLOWED: &[&str] = &[
        "accept-ch",
        "cache-control",
        "cf-mitigated",
        "cf-ray",
        "content-type",
        "critical-ch",
        "location",
        "server",
        "x-akamai-transformed",
        "x-cache",
        "x-datadome",
        "x-px",
        "x-sucuri-id",
    ];
    value
        .as_object()
        .into_iter()
        .flatten()
        .filter(|(name, _)| {
            ALLOWED
                .iter()
                .any(|allowed| name.eq_ignore_ascii_case(allowed))
        })
        .filter_map(|(name, value)| {
            value.as_str().map(|value| {
                let value = if name.eq_ignore_ascii_case("location") {
                    url(value)
                } else {
                    bounded(value)
                };
                (name.to_ascii_lowercase(), value)
            })
        })
        .take(MAX_COLLECTION_ITEMS)
        .collect()
}

/// Returns a cookie's name and metadata without ever returning its value.
pub fn cookie_metadata(name: &str, domain: &str, path: &str) -> (String, String, String) {
    (bounded(name), bounded(domain), bounded(path))
}

/// Compares two sensitive identities without placing either value in a report.
pub fn same_identity(left: &str, right: &str) -> bool {
    left == right
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn strips_url_secrets() {
        let safe = url("https://user:password@example.com/path?token=secret#fragment");
        assert_eq!(safe, "https://example.com/path");
        assert!(!safe.contains("password"));
        assert!(!safe.contains("secret"));
    }

    #[test]
    fn allowlists_headers_and_bounds_values() {
        let headers = safe_headers(&json!({
            "Authorization": "Bearer secret",
            "Content-Type": "text/html",
            "Location": "https://example.com/?token=secret",
        }));
        assert!(!headers.contains_key("authorization"));
        assert_eq!(headers["content-type"], "text/html");
        assert_eq!(headers["location"], "https://example.com/");
    }

    #[test]
    fn cookie_metadata_excludes_values() {
        let (name, domain, path) = cookie_metadata("session", "example.com", "/");
        assert_eq!(
            (name, domain, path),
            ("session".into(), "example.com".into(), "/".into())
        );
    }
}
