#![cfg(feature = "seeding")]

use serde_json::json;
use stealth_oxide::{CookieSeed, IndexedDbSeed, OriginSeed, ProfileSeed};

#[test]
fn creates_three_default_seed_documents() {
    let seeds = ProfileSeed::defaults_for("https://example.com/path").unwrap();

    assert_eq!(seeds.len(), 3);
    assert_eq!(
        seeds.iter().map(|seed| seed.cookies.len()).sum::<usize>(),
        1
    );
    assert_eq!(
        seeds
            .iter()
            .flat_map(|seed| &seed.origins)
            .map(|origin| origin.local_storage.len())
            .sum::<usize>(),
        2
    );
    assert_eq!(
        seeds
            .iter()
            .flat_map(|seed| &seed.origins)
            .map(|origin| origin.indexed_db.len())
            .sum::<usize>(),
        1
    );
}

#[test]
fn builds_and_validates_custom_seed_data() {
    let seed = ProfileSeed::new()
        .cookie(
            CookieSeed::new("test-session", "value", "https://app.example.test/")
                .secure(true)
                .http_only(true),
        )
        .origin(
            OriginSeed::new("https://app.example.test")
                .local_storage("theme", "dark")
                .indexed_db(IndexedDbSeed::new(
                    "test-state",
                    "settings",
                    json!("onboarding"),
                    json!({ "complete": true }),
                )),
        );

    seed.validate().unwrap();
}

#[test]
fn decodes_multiple_seed_documents_independently() {
    let first = ProfileSeed::from_json(
        r#"{"cookies":[{"name":"a","value":"1","url":"https://example.com/"}]}"#,
    )
    .unwrap();
    let second = ProfileSeed::from_json(
        r#"{"origins":[{"origin":"https://example.com","localStorage":{"theme":"dark"}}]}"#,
    )
    .unwrap();

    assert_eq!(first.cookies.len(), 1);
    assert_eq!(second.origins[0].local_storage.len(), 1);
}

#[test]
fn rejects_non_origin_storage_urls() {
    let seed = ProfileSeed::new().origin(OriginSeed::new("https://example.com/path"));

    assert!(seed.validate().is_err());
}
