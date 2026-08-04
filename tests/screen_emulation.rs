use std::time::Duration;

use anyhow::{Context, Result};
use serde_json::Value;
use tokio::time::timeout;

mod common;
use common::TestBrowser as StealthBrowser;
use stealth_oxide::profiles::chrome_windows::chrome_windows;

#[tokio::test]
#[ignore = "requires a local Chromium process with working CDP sockets"]
async fn screen_patch_keeps_cdp_controlled_surfaces_consistent() -> Result<()> {
    let uses_native_screen = matches!(
        std::env::var("STEALTH_OXIDE_USE_NATIVE_SCREEN").as_deref(),
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

    timeout(Duration::from_secs(20), browser.close())
        .await
        .context("timed out while closing Chromium")??;

    assert_eq!(
        observed["top"]["screenWidth"].as_u64(),
        Some(u64::from(expected_screen.width))
    );
    assert_eq!(
        observed["top"]["screenHeight"].as_u64(),
        Some(u64::from(expected_screen.height))
    );
    assert_eq!(
        observed["top"]["devicePixelRatio"].as_f64(),
        Some(expected_screen.device_scale_factor)
    );
    assert_eq!(observed["top"]["widthMediaMatches"], true);
    assert_eq!(observed["top"]["heightMediaMatches"], true);
    assert_eq!(observed["top"]["resolutionMediaMatches"], true);
    if uses_native_screen {
        assert!(observed["top"]["innerWidth"].as_u64().unwrap_or(0) > 0);
        assert!(observed["top"]["innerHeight"].as_u64().unwrap_or(0) > 0);
        assert_eq!(
            observed["top"]["visualViewportWidth"],
            observed["top"]["innerWidth"]
        );
        assert_eq!(
            observed["top"]["visualViewportHeight"],
            observed["top"]["innerHeight"]
        );
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

    if uses_native_screen {
        assert_eq!(
            observed["top"]["availWidth"].as_u64(),
            Some(u64::from(expected_screen.available_width))
        );
        assert_eq!(
            observed["top"]["availHeight"].as_u64(),
            Some(u64::from(expected_screen.available_height))
        );
    } else {
        // CDP's device-metrics override has no separate available-area input.
        assert_eq!(
            observed["top"]["availWidth"],
            observed["top"]["screenWidth"]
        );
        assert_eq!(
            observed["top"]["availHeight"],
            observed["top"]["screenHeight"]
        );
    }
    assert_eq!(observed["top"]["pixelDepth"], observed["top"]["colorDepth"]);
    assert!(observed["top"]["outerWidth"].as_u64().unwrap_or(0) > 0);
    assert!(observed["top"]["outerHeight"].as_u64().unwrap_or(0) > 0);

    assert_eq!(
        observed["iframe"]["screenWidth"].as_u64(),
        Some(u64::from(expected_screen.width))
    );
    assert_eq!(
        observed["iframe"]["screenHeight"].as_u64(),
        Some(u64::from(expected_screen.height))
    );
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
