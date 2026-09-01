use std::time::Duration;

use anyhow::{Context, Result};
use serde_json::Value;
use tokio::time::timeout;

use super::common;
use common::TestBrowser as StealthBrowser;
use stealth_oxide::StealthConfig;
use stealth_oxide::profiles::chrome_windows::chrome_windows;

#[tokio::test]
#[ignore = "requires a local Chromium process with working CDP sockets"]
async fn native_hardware_concurrency_override_reaches_page_and_iframe() -> Result<()> {
    let profile = chrome_windows();
    let native_baseline = matches!(
        std::env::var("STEALTH_OXIDE_HARDWARE_TEST_MODE").as_deref(),
        Ok("native")
    );
    let expected = profile.hardware().hardware_concurrency;
    let browser = timeout(
        Duration::from_secs(20),
        StealthBrowser::launch(profile.clone()),
    )
    .await
    .context("timed out while launching Chromium")??;
    let config = if native_baseline {
        StealthConfig::from_profile(profile)
    } else {
        StealthConfig::from_profile(profile).hardware_concurrency(expected)
    };
    let url = std::env::var("STEALTH_OXIDE_HARDWARE_TEST_URL").unwrap_or_else(|_| {
        "data:text/html,<iframe id='probe' srcdoc='<p>probe</p>'></iframe>".to_string()
    });
    let page = timeout(
        Duration::from_secs(20),
        browser.new_page_with_stealth(&url, &config),
    )
    .await
    .context("timed out while creating the patched page")??;

    let observed: Value = timeout(
        Duration::from_secs(20),
        page.inner().evaluate(
            r#"({
                top: navigator.hardwareConcurrency,
                iframe: document.querySelector('#probe')?.contentWindow?.navigator.hardwareConcurrency ?? null
            })"#,
        ),
    )
    .await
    .context("timed out while reading hardware concurrency")??
    .into_value()?;

    println!(
        "mode={}, navigation: status={:?}, final_url={:?}, redirects={:?}; observed={observed:#}",
        if native_baseline {
            "native"
        } else {
            "override"
        },
        page.navigation().status,
        page.navigation().final_url,
        page.navigation().redirect_statuses,
    );

    timeout(Duration::from_secs(20), browser.close())
        .await
        .context("timed out while closing Chromium")??;

    if !native_baseline {
        assert_eq!(observed["top"].as_u64(), Some(u64::from(expected)));
    }
    if !observed["iframe"].is_null() {
        assert_eq!(observed["iframe"], observed["top"]);
    }
    if let Ok(expected_status) = std::env::var("STEALTH_OXIDE_EXPECTED_STATUS") {
        let expected_statuses = expected_status
            .split(',')
            .map(str::trim)
            .map(str::parse::<i64>)
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let observed_status = page.navigation().status;
        assert!(
            observed_status.is_some_and(|status| expected_statuses.contains(&status)),
            "observed status {observed_status:?} was not in the expected set {expected_statuses:?}"
        );
    }
    Ok(())
}

#[tokio::test]
#[ignore = "requires a local Chromium process with working CDP sockets"]
async fn profile_helper_applies_hardware_before_the_first_document_script() -> Result<()> {
    let profile = chrome_windows();
    let expected = profile.hardware().hardware_concurrency;
    let browser = timeout(
        Duration::from_secs(20),
        StealthBrowser::launch(profile.clone()),
    )
    .await
    .context("timed out while launching Chromium")??;
    let config = StealthConfig::from_profile(profile).hardware_concurrency(expected);
    let url =
        "data:text/html,<script>document.title=String(navigator.hardwareConcurrency)</script>";
    let page = timeout(
        Duration::from_secs(20),
        browser.new_page_with_profile_helper(url, &config),
    )
    .await
    .context("timed out while creating the profile-helper page")??;
    let observed = page.get_title().await?.unwrap_or_default();

    timeout(Duration::from_secs(20), browser.close())
        .await
        .context("timed out while closing Chromium")??;

    assert_eq!(observed, expected.to_string());
    Ok(())
}
