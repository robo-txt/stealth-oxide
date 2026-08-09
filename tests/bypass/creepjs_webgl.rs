use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde_json::Value;
use tokio::time::{Instant, sleep};

use super::common;
use common::TestBrowser as StealthBrowser;
use stealth_oxide::profiles::chrome_windows::chrome_windows;

const CREEPJS_URL: &str = "https://abrahamjuliot.github.io/creepjs/";

#[tokio::test]
#[ignore = "requires Chromium, network access, and the desktop container"]
async fn creepjs_webgl_report() -> Result<()> {
    let browser = StealthBrowser::launch(chrome_windows()).await?;
    let version = browser.version().await?;
    let system_info = browser.system_info().await?;
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
                    const readSection = title => {
                        const heading = [...document.querySelectorAll('strong')]
                            .find(element => element.textContent?.trim() === title);
                        const section = heading?.parentElement;
                        return !section ? null : {
                            text: section.innerText?.trim() || '',
                            rejected: section.classList.contains('rejected')
                        };
                    };
                    const readRating = (selector, keys) => {
                        const text = document.querySelector(selector)?.textContent || '';
                        return {
                            rating: Number(text.match(/(\d+)%/)?.[1]),
                            signals: Object.fromEntries(keys.map(key => [
                                key,
                                text.includes(`${key}: true`)
                            ]))
                        };
                    };
                    return {
                        webgl: readSection('WebGL'),
                        worker: readSection('Worker'),
                        headless: readRating('.headless-rating', [
                            'webDriverIsOn', 'hasHeadlessUA', 'hasHeadlessWorkerUA'
                        ]),
                        likeHeadless: readRating('.like-headless-rating', [
                            'hasSwiftShader'
                        ]),
                        stealth: readRating('.stealth-rating', [
                            'hasBadWebGL'
                        ])
                    };
                })()
                "#,
            )
            .await?
            .into_value()?;

        let ready = report["webgl"]["text"].as_str().is_some_and(|text| {
            !text.contains("Blocked") && !text.contains("unsupported") && text.lines().count() >= 4
        }) && report["headless"]["rating"].as_u64().is_some();
        if ready {
            break report;
        }
        if Instant::now() >= deadline {
            bail!("timed out waiting for the CreepJS WebGL report");
        }
        sleep(Duration::from_millis(500)).await;
    };

    println!("Chromium: {version}");
    println!("CDP GPU info: {:#?}", system_info.gpu);
    println!("{}", serde_json::to_string_pretty(&report)?);

    browser.close().await?;
    Ok(())
}
