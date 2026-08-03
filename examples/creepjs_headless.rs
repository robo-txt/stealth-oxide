use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde_json::Value;
use tokio::time::{Instant, sleep};

use stealth_oxide::browser::StealthBrowser;
use stealth_oxide::profiles::chrome_windows::chrome_windows;

const CREEPJS_URL: &str = "https://abrahamjuliot.github.io/creepjs/";

#[tokio::main]
async fn main() -> Result<()> {
    let browser = StealthBrowser::launch(chrome_windows()).await?;
    let version = browser.version().await?;
    let page = browser
        .new_page(CREEPJS_URL)
        .await
        .context("failed to open CreepJS")?;

    let deadline = Instant::now() + Duration::from_secs(60);
    let report = loop {
        let report: Value = page
            .inner()
            .evaluate(
                r#"
                (() => {
                    const read = (selector, keys) => {
                        const text = document.querySelector(selector)?.textContent || '';
                        const rating = Number(text.match(/(\d+)%/)?.[1]);
                        return {
                            rating,
                            signals: Object.fromEntries(keys.map(key => [
                                key,
                                text.includes(`${key}: true`)
                            ]))
                        };
                    };
                    return {
                        likeHeadless: read('.like-headless-rating', [
                            'noChrome', 'hasPermissionsBug', 'noPlugins',
                            'noMimeTypes', 'notificationIsDenied',
                            'hasKnownBgColor', 'prefersLightColor',
                            'uaDataIsBlank', 'pdfIsDisabled', 'noTaskbar',
                            'hasVvpScreenRes', 'hasSwiftShader', 'noWebShare',
                            'noContentIndex', 'noContactsManager', 'noDownlinkMax'
                        ]),
                        headless: read('.headless-rating', [
                            'webDriverIsOn', 'hasHeadlessUA', 'hasHeadlessWorkerUA'
                        ]),
                        stealth: read('.stealth-rating', [
                            'hasIframeProxy', 'hasHighChromeIndex',
                            'hasBadChromeRuntime', 'hasToStringProxy', 'hasBadWebGL'
                        ])
                    };
                })()
                "#,
            )
            .await?
            .into_value()?;

        if report["headless"]["rating"].as_u64().is_some() {
            break report;
        }
        if Instant::now() >= deadline {
            bail!("timed out waiting for the CreepJS Headless report");
        }
        sleep(Duration::from_millis(500)).await;
    };

    println!("Chromium: {version}");
    println!("{}", serde_json::to_string_pretty(&report)?);

    browser.close().await?;
    Ok(())
}
