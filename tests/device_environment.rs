use std::time::Duration;

use anyhow::{Context, Result};
use serde_json::Value;
use tokio::time::timeout;

use stealth_oxide::browser::StealthBrowser;
use stealth_oxide::profiles::chrome_windows::chrome_windows;

#[tokio::test]
#[ignore = "requires a local Chromium process with working CDP sockets"]
async fn device_media_and_touch_match_the_desktop_profile() -> Result<()> {
    let profile = chrome_windows();
    let expected = profile.device_environment.clone();
    let browser = timeout(Duration::from_secs(20), StealthBrowser::launch(profile))
        .await
        .context("timed out while launching Chromium")??;
    let page = timeout(
        Duration::from_secs(20),
        browser.new_page("data:text/html,<iframe id='probe' srcdoc='<p>probe</p>'></iframe>"),
    )
    .await
    .context("timed out while creating the patched page")??;

    let observed: Value = timeout(
        Duration::from_secs(20),
        page.inner().evaluate(
            r#"
            (() => {
                const snapshot = window => {
                    const document = window.document;
                    const style = document.createElement('style');
                    style.textContent = `
                        @media (prefers-color-scheme: light) { body { --scheme: light; } }
                        @media (prefers-color-scheme: dark) { body { --scheme: dark; } }
                        @media (prefers-reduced-motion: no-preference) { body { --motion: no-preference; } }
                        @media (prefers-reduced-motion: reduce) { body { --motion: reduce; } }
                        @media (forced-colors: none) { body { --forced: none; } }
                        @media (forced-colors: active) { body { --forced: active; } }
                        @media (pointer: fine) { body { --pointer: fine; } }
                        @media (pointer: coarse) { body { --pointer: coarse; } }
                        @media (hover: hover) { body { --hover: hover; } }
                        @media (hover: none) { body { --hover: none; } }
                    `;
                    document.head.appendChild(style);
                    const computed = window.getComputedStyle(document.body);
                    return {
                        colorScheme: window.matchMedia('(prefers-color-scheme: light)').matches ? 'light' :
                            window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : null,
                        reducedMotion: window.matchMedia('(prefers-reduced-motion: no-preference)').matches ? 'no-preference' :
                            window.matchMedia('(prefers-reduced-motion: reduce)').matches ? 'reduce' : null,
                        forcedColors: window.matchMedia('(forced-colors: none)').matches ? 'none' :
                            window.matchMedia('(forced-colors: active)').matches ? 'active' : null,
                        colorGamut: window.matchMedia('(color-gamut: rec2020)').matches ? 'rec2020' :
                            window.matchMedia('(color-gamut: p3)').matches ? 'p3' :
                            window.matchMedia('(color-gamut: srgb)').matches ? 'srgb' : null,
                        monochrome: window.matchMedia('(monochrome)').matches ? 'monochrome' :
                            window.matchMedia('(monochrome: 0)').matches ? '0' : null,
                        hover: window.matchMedia('(hover: hover)').matches ? 'hover' : 'none',
                        anyHover: window.matchMedia('(any-hover: hover)').matches ? 'hover' : 'none',
                        pointer: window.matchMedia('(pointer: fine)').matches ? 'fine' :
                            window.matchMedia('(pointer: coarse)').matches ? 'coarse' : 'none',
                        anyPointer: window.matchMedia('(any-pointer: fine)').matches ? 'fine' :
                            window.matchMedia('(any-pointer: coarse)').matches ? 'coarse' : 'none',
                        maxTouchPoints: window.navigator.maxTouchPoints,
                        hasTouchEvent: 'ontouchstart' in window,
                        css: {
                            colorScheme: computed.getPropertyValue('--scheme').trim(),
                            reducedMotion: computed.getPropertyValue('--motion').trim(),
                            forcedColors: computed.getPropertyValue('--forced').trim(),
                            pointer: computed.getPropertyValue('--pointer').trim(),
                            hover: computed.getPropertyValue('--hover').trim()
                        },
                        fonts: {
                            segoeUi: document.fonts.check('12px "Segoe UI"'),
                            arial: document.fonts.check('12px Arial'),
                            timesNewRoman: document.fonts.check('12px "Times New Roman"')
                        }
                    };
                };
                return {
                    top: snapshot(window),
                    iframe: snapshot(document.querySelector('#probe').contentWindow)
                };
            })()
            "#,
        ),
    )
    .await
    .context("timed out while reading device-environment surfaces")??
    .into_value()?;

    println!("observed device environment: {observed:#}");

    timeout(Duration::from_secs(20), browser.close())
        .await
        .context("timed out while closing Chromium")??;

    for realm in ["top", "iframe"] {
        assert_eq!(observed[realm]["reducedMotion"], expected.reduced_motion);
        assert_eq!(observed[realm]["forcedColors"], expected.forced_colors);
        assert_eq!(observed[realm]["colorGamut"], expected.color_gamut);
        assert_eq!(observed[realm]["monochrome"], expected.monochrome);
        assert_eq!(
            observed[realm]["maxTouchPoints"].as_u64(),
            Some(u64::from(expected.max_touch_points))
        );
        assert_eq!(observed[realm]["hasTouchEvent"], expected.touch_enabled);
        assert_eq!(
            observed[realm]["css"]["reducedMotion"],
            expected.reduced_motion
        );
        assert_eq!(
            observed[realm]["css"]["forcedColors"],
            expected.forced_colors
        );
    }

    assert_eq!(observed["iframe"]["hover"], observed["top"]["hover"]);
    assert_eq!(observed["iframe"]["anyHover"], observed["top"]["anyHover"]);
    assert_eq!(observed["iframe"]["pointer"], observed["top"]["pointer"]);
    assert_eq!(
        observed["iframe"]["anyPointer"],
        observed["top"]["anyPointer"]
    );

    Ok(())
}
