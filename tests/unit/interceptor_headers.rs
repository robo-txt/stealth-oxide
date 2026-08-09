#![cfg(feature = "interceptor")]

use stealth_oxide::interceptor::{Header, HeaderPolicy};

#[test]
fn replaces_case_insensitively_without_reordering_other_headers() {
    let policy = HeaderPolicy::builder()
        .set_header("X-Test-Scenario", "checkout")
        .build()
        .unwrap();
    let original = vec![
        Header::new("Accept", "application/json"),
        Header::new("x-test-scenario", "old"),
        Header::new("User-Agent", "Chromium"),
    ];

    let result = policy.apply(&original).unwrap();

    assert_eq!(result[0], original[0]);
    assert_eq!(result[1], Header::new("x-test-scenario", "checkout"));
    assert_eq!(result[2], original[2]);
}

#[test]
fn appends_new_headers_in_policy_order() {
    let policy = HeaderPolicy::builder()
        .set_header("x-first", "one")
        .set_header("x-second", "two")
        .build()
        .unwrap();

    let result = policy.apply(&[Header::new("accept", "*/*")]).unwrap();

    assert_eq!(
        result,
        vec![
            Header::new("accept", "*/*"),
            Header::new("x-first", "one"),
            Header::new("x-second", "two"),
        ]
    );
}

#[test]
fn repeated_configured_name_uses_the_last_value_without_duplication() {
    let policy = HeaderPolicy::builder()
        .set_header("X-Test", "one")
        .set_header("x-test", "two")
        .build()
        .unwrap();

    let result = policy.apply(&[]).unwrap();

    assert_eq!(result, vec![Header::new("x-test", "two")]);
}

#[test]
fn rejects_pseudo_hop_by_hop_framing_and_credential_headers() {
    let denied = [
        ":authority",
        "Connection",
        "CONTENT-LENGTH",
        "host",
        "keep-alive",
        "proxy-authenticate",
        "proxy-authorization",
        "te",
        "trailer",
        "transfer-encoding",
        "upgrade",
        "Authorization",
        "Cookie",
        "Set-Cookie",
    ];

    for name in denied {
        let result = HeaderPolicy::builder().set_header(name, "value").build();
        assert!(result.is_err(), "{name} should be rejected");
    }
}

#[test]
fn rejects_invalid_names_and_unsafe_values() {
    for name in ["", "bad name", "bad/name", "nön-ascii"] {
        assert!(
            HeaderPolicy::builder()
                .set_header(name, "value")
                .build()
                .is_err(),
            "{name:?} should be rejected"
        );
    }

    for value in ["line\rbreak", "line\nbreak", "nul\0byte", "escape\u{1b}"] {
        assert!(
            HeaderPolicy::builder()
                .set_header("x-test", value)
                .build()
                .is_err(),
            "{value:?} should be rejected"
        );
    }
}

#[test]
fn duplicate_exposed_target_fails_open_at_the_caller_boundary() {
    let policy = HeaderPolicy::builder()
        .set_header("x-test", "new")
        .build()
        .unwrap();
    let original = vec![Header::new("X-Test", "one"), Header::new("x-test", "two")];

    assert!(policy.apply(&original).is_err());
    assert_eq!(original[0].value(), "one");
    assert_eq!(original[1].name(), "x-test");
}
