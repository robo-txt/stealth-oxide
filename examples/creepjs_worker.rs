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
                    const workerHeading = [...document.querySelectorAll('strong')]
                        .find(element => element.textContent?.trim() === 'Worker');
                    const workerColumn = workerHeading?.closest('.col-six');
                    const workerSibling = workerColumn?.nextElementSibling;
                    const rating = (selector, signals) => {
                        const text = document.querySelector(selector)?.textContent || '';
                        return {
                            percent: Number(text.match(/(\d+)%/)?.[1]),
                            signals: Object.fromEntries(signals.map(signal => [
                                signal,
                                text.includes(`${signal}: true`)
                            ]))
                        };
                    };
                    return {
                        worker: !workerColumn ? null : {
                            rejected: workerColumn.classList.contains('rejected') ||
                                workerSibling?.classList.contains('rejected'),
                            summary: workerColumn.innerText?.trim() || '',
                            identity: workerSibling?.innerText?.trim() || ''
                        },
                        headless: rating('.headless-rating', [
                            'webDriverIsOn', 'hasHeadlessUA', 'hasHeadlessWorkerUA'
                        ]),
                        likeHeadless: rating('.like-headless-rating', [
                            'hasSwiftShader'
                        ]),
                        stealth: rating('.stealth-rating', [
                            'hasBadWebGL'
                        ])
                    };
                })()
                "#,
            )
            .await?
            .into_value()?;

        let ready = report["worker"]["identity"]
            .as_str()
            .is_some_and(|text| text.contains("userAgent:"))
            && report["headless"]["percent"].as_u64().is_some();
        if ready {
            break report;
        }
        if Instant::now() >= deadline {
            bail!("timed out waiting for the CreepJS worker report");
        }
        sleep(Duration::from_millis(500)).await;
    };

    println!("Chromium: {version}");
    println!("{}", serde_json::to_string_pretty(&report)?);

    browser.close().await?;
    Ok(())
}
