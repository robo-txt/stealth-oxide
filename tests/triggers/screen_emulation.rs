use std::time::Duration;

use anyhow::{Context, Result};
use chromiumoxide::cdp::browser_protocol::emulation::GetScreenInfosParams;
use serde_json::Value;
use tokio::time::timeout;

use super::common;
use common::TestBrowser as StealthBrowser;
use stealth_oxide::profiles::chrome_windows::chrome_windows;

#[tokio::test]
#[ignore = "requires a local Chromium process with working CDP sockets"]
async fn screen_patch_keeps_cdp_controlled_surfaces_consistent() -> Result<()> {
    let uses_native_screen = matches!(
        std::env::var("STEALTH_OXIDE_USE_NATIVE_SCREEN").as_deref(),
        Ok("1") | Ok("true")
    );
    let uses_headful = matches!(
        std::env::var("STEALTH_OXIDE_HEADFUL").as_deref(),
        Ok("1") | Ok("true")
    );
    let profile = chrome_windows();
    let expected_screen = profile.screen().clone();
    let browser = timeout(Duration::from_secs(20), StealthBrowser::launch(profile))
        .await
        .context("timed out while launching Chromium")??;

    let page = timeout(
        Duration::from_secs(20),
        browser.new_page(
            "data:text/html,<iframe id='probe' width='640' height='480' srcdoc='<p>probe</p>'></iframe>",
        ),
    )
    .await
    .context("timed out while creating and patching the page")??;

    let observed: Value = timeout(
        Duration::from_secs(20),
        page.inner().evaluate(
            r#"
            (() => {
                const iframeWindow = document.querySelector('#probe').contentWindow;

                const snapshot = (window) => ({
                    screenWidth: window.screen.width,
                    screenHeight: window.screen.height,
                    availWidth: window.screen.availWidth,
                    availHeight: window.screen.availHeight,
                    colorDepth: window.screen.colorDepth,
                    pixelDepth: window.screen.pixelDepth,
                    orientationType: window.screen.orientation?.type,
                    orientationAngle: window.screen.orientation?.angle,
                    devicePixelRatio: window.devicePixelRatio,
                    outerWidth: window.outerWidth,
                    outerHeight: window.outerHeight,
                    innerWidth: window.innerWidth,
                    innerHeight: window.innerHeight,
                    visualViewportWidth: window.visualViewport?.width,
                    visualViewportHeight: window.visualViewport?.height,
                    widthMediaMatches: window.matchMedia(`(device-width: ${window.screen.width}px)`).matches,
                    heightMediaMatches: window.matchMedia(`(device-height: ${window.screen.height}px)`).matches,
                    resolutionMediaMatches: window.matchMedia(`(resolution: ${window.devicePixelRatio}dppx)`).matches,
                    landscapeMediaMatches: window.matchMedia('(orientation: landscape)').matches,
                    portraitMediaMatches: window.matchMedia('(orientation: portrait)').matches
                });

                return {
                    top: snapshot(window),
                    iframe: snapshot(iframeWindow)
                };
            })()
            "#,
        ),
    )
    .await
    .context("timed out while reading browser-visible screen values")??
        .into_value()?;

    let native_screen_info = page
        .inner()
        .execute(GetScreenInfosParams::default())
        .await?;
    let native_screen = native_screen_info
        .screen_infos
        .iter()
        .into_iter()
        .find(|screen| screen.is_primary)
        .context("primary native screen was not reported")?;

    timeout(Duration::from_secs(20), browser.close())
        .await
        .context("timed out while closing Chromium")??;

    if uses_headful || uses_native_screen {
        // Native-screen mode preserves the host's physical display. CDP
        // cannot synthesize an OS window frame or replace that native screen.
        assert!(observed["top"]["screenWidth"].as_u64().unwrap_or(0) > 0);
        assert!(observed["top"]["screenHeight"].as_u64().unwrap_or(0) > 0);
        assert!(
            observed["top"]["availWidth"].as_u64().unwrap_or(0)
                <= observed["top"]["screenWidth"].as_u64().unwrap_or(0)
        );
        assert!(
            observed["top"]["availHeight"].as_u64().unwrap_or(0)
                <= observed["top"]["screenHeight"].as_u64().unwrap_or(0)
        );
    } else {
        assert_eq!(
            observed["top"]["screenWidth"].as_u64(),
            Some(u64::from(expected_screen.width))
        );
        assert_eq!(
            observed["top"]["screenHeight"].as_u64(),
            Some(u64::from(expected_screen.height))
        );
    }
    if uses_headful || uses_native_screen {
        assert!(observed["top"]["devicePixelRatio"].as_f64().unwrap_or(0.0) > 0.0);
    } else {
        assert_eq!(
            observed["top"]["devicePixelRatio"].as_f64(),
            Some(expected_screen.device_scale_factor)
        );
        assert_eq!(observed["top"]["widthMediaMatches"], true);
        assert_eq!(observed["top"]["heightMediaMatches"], true);
        assert_eq!(observed["top"]["resolutionMediaMatches"], true);
    }
    if uses_headful || uses_native_screen {
        assert!(observed["top"]["innerWidth"].as_u64().unwrap_or(0) > 0);
        assert!(observed["top"]["innerHeight"].as_u64().unwrap_or(0) > 0);
        let inner_width = observed["top"]["innerWidth"].as_f64().unwrap_or(0.0);
        let inner_height = observed["top"]["innerHeight"].as_f64().unwrap_or(0.0);
        let viewport_width = observed["top"]["visualViewportWidth"]
            .as_f64()
            .unwrap_or(0.0);
        let viewport_height = observed["top"]["visualViewportHeight"]
            .as_f64()
            .unwrap_or(0.0);
        assert!((viewport_width - inner_width).abs() <= 1.0);
        assert!((viewport_height - inner_height).abs() <= 1.0);
    } else {
        assert_eq!(observed["top"]["innerWidth"].as_u64(), Some(1920));
        assert_eq!(observed["top"]["innerHeight"].as_u64(), Some(1080));
        assert_eq!(observed["top"]["visualViewportWidth"].as_u64(), Some(1920));
        assert_eq!(observed["top"]["visualViewportHeight"].as_u64(), Some(1080));
    }
    assert_eq!(observed["top"]["orientationType"], "landscape-primary");
    assert_eq!(observed["top"]["orientationAngle"], 0);
    assert_eq!(observed["top"]["landscapeMediaMatches"], true);
    assert_eq!(observed["top"]["portraitMediaMatches"], false);

    assert!(observed["top"]["availWidth"].as_u64().unwrap_or(0) > 0);
    assert!(observed["top"]["availHeight"].as_u64().unwrap_or(0) > 0);
    assert!(
        observed["top"]["availWidth"].as_u64().unwrap_or(0)
            <= observed["top"]["screenWidth"].as_u64().unwrap_or(0)
    );
    assert!(
        observed["top"]["availHeight"].as_u64().unwrap_or(0)
            <= observed["top"]["screenHeight"].as_u64().unwrap_or(0)
    );
    if !uses_headful && !uses_native_screen {
        assert_eq!(native_screen.avail_width, i64::from(expected_screen.width));
        assert_eq!(
            native_screen.avail_height,
            i64::from(expected_screen.available_height)
        );
    }
    assert_eq!(observed["top"]["pixelDepth"], observed["top"]["colorDepth"]);
    assert!(observed["top"]["outerWidth"].as_u64().unwrap_or(0) > 0);
    assert!(observed["top"]["outerHeight"].as_u64().unwrap_or(0) > 0);

    if uses_headful || uses_native_screen {
        assert_eq!(
            observed["iframe"]["screenWidth"],
            observed["top"]["screenWidth"]
        );
        assert_eq!(
            observed["iframe"]["screenHeight"],
            observed["top"]["screenHeight"]
        );
    } else {
        assert_eq!(
            observed["iframe"]["screenWidth"].as_u64(),
            Some(u64::from(expected_screen.width))
        );
        assert_eq!(
            observed["iframe"]["screenHeight"].as_u64(),
            Some(u64::from(expected_screen.height))
        );
    }
    for property in [
        "availWidth",
        "availHeight",
        "colorDepth",
        "pixelDepth",
        "orientationType",
        "orientationAngle",
        "devicePixelRatio",
        "outerWidth",
        "outerHeight",
    ] {
        assert_eq!(observed["iframe"][property], observed["top"][property]);
    }
    assert_eq!(observed["iframe"]["innerWidth"].as_u64(), Some(640));
    assert_eq!(observed["iframe"]["innerHeight"].as_u64(), Some(480));
    assert_eq!(
        observed["iframe"]["visualViewportWidth"].as_u64(),
        Some(640)
    );
    assert_eq!(
        observed["iframe"]["visualViewportHeight"].as_u64(),
        Some(480)
    );
    assert_eq!(observed["iframe"]["widthMediaMatches"], true);
    assert_eq!(observed["iframe"]["heightMediaMatches"], true);
    assert_eq!(observed["iframe"]["resolutionMediaMatches"], true);
    assert_eq!(observed["iframe"]["landscapeMediaMatches"], true);
    assert_eq!(observed["iframe"]["portraitMediaMatches"], false);

    Ok(())
}
