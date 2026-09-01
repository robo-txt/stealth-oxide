use std::time::Duration;

use anyhow::{Context, Result};
use serde_json::Value;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::time::timeout;
use url::Url;

use super::common;
use common::TestBrowser as StealthBrowser;
use stealth_oxide::profiles::chrome_windows::chrome_windows;
use stealth_oxide::{GeolocationConfig, PermissionOverride, PermissionSetting, StealthConfig};

async fn loopback_page() -> Result<(String, String)> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
    let address = listener.local_addr()?;
    let body = r#"<!doctype html><title>native geolocation probe</title><p>probe</p>"#;
    tokio::spawn(async move {
        if let Ok((mut stream, _)) = listener.accept().await {
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(response.as_bytes()).await;
        }
    });
    let origin = format!("http://{address}");
    Ok((format!("{origin}/"), origin))
}

async fn test_target() -> Result<(String, String)> {
    if let Ok(url) = std::env::var("STEALTH_OXIDE_PERMISSION_TEST_URL") {
        let parsed = Url::parse(&url).context("invalid permission test URL")?;
        anyhow::ensure!(
            matches!(parsed.scheme(), "http" | "https") && parsed.host().is_some(),
            "permission test URL must be an HTTP(S) URL with a host"
        );
        let origin = parsed.origin().ascii_serialization();
        return Ok((url, origin));
    }
    loopback_page().await
}

#[tokio::test]
#[ignore = "requires a local Chromium process with working CDP sockets"]
async fn native_permission_and_geolocation_controls_match() -> Result<()> {
    let profile = chrome_windows();
    let (url, origin) = test_target().await?;
    let browser = timeout(Duration::from_secs(20), StealthBrowser::launch(profile))
        .await
        .context("timed out while launching Chromium")??;
    let config = StealthConfig::from_profile(chrome_windows())
        .permission(PermissionOverride::for_origin(
            "geolocation",
            PermissionSetting::Granted,
            &origin,
        ))
        .geolocation(GeolocationConfig::position(40.7128, -74.006, 25.0));
    browser.apply_browser_stealth(&config).await?;
    let page_config = config.use_native(stealth_oxide::Patch::Permissions);
    let page = timeout(
        Duration::from_secs(20),
        browser.new_page_with_stealth(&url, &page_config),
    )
    .await
    .context("timed out while creating the configured page")??;
    // Some cross-origin navigation paths reset target-scoped geolocation;
    // reapply after the final document is active before probing it.
    page_config.apply(page.inner()).await?;

    let observed: Value = timeout(
        Duration::from_secs(20),
        page.inner().evaluate(
            r#"(async () => {
                const permission = await navigator.permissions.query({ name: 'geolocation' });
                const position = await new Promise(resolve => {
                    navigator.geolocation.getCurrentPosition(
                        value => resolve({
                            ok: true,
                            latitude: value.coords.latitude,
                            longitude: value.coords.longitude,
                            accuracy: value.coords.accuracy
                        }),
                        error => resolve({ ok: false, code: error.code, message: error.message }),
                        { maximumAge: 0, timeout: 10000 }
                    );
                });
                return { permission: permission.state, position };
            })()"#,
        ),
    )
    .await
    .context("timed out while reading native permission and geolocation")??
    .into_value()?;

    println!(
        "url={url}, navigation: status={:?}, final_url={:?}, redirects={:?}; native permission/geolocation observed: {observed:#}",
        page.navigation().status,
        page.navigation().final_url,
        page.navigation().redirect_statuses,
    );

    timeout(Duration::from_secs(20), browser.close())
        .await
        .context("timed out while closing Chromium")??;

    assert_eq!(observed["permission"], "granted");
    assert_eq!(observed["position"]["ok"], true);
    assert_eq!(observed["position"]["latitude"].as_f64(), Some(40.7128));
    assert_eq!(observed["position"]["longitude"].as_f64(), Some(-74.006));
    assert_eq!(observed["position"]["accuracy"].as_f64(), Some(25.0));
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
