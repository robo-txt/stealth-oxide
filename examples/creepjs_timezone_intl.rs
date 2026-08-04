use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde_json::Value;
use tokio::time::{Instant, sleep};

mod common;
use common::BrowserSession as StealthBrowser;
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
                    const readSection = title => {
                        const heading = [...document.querySelectorAll('strong')]
                            .find(element => element.textContent?.trim() === title);
                        const section = heading?.parentElement;
                        return !section ? null : {
                            text: section.innerText?.trim() || '',
                            rejected: section.classList.contains('rejected'),
                            lowerEntropy: !!section.querySelector('.bold-fail')
                        };
                    };
                    return {
                        timezone: readSection('Timezone'),
                        intl: readSection('Intl')
                    };
                })()
                "#,
            )
            .await?
            .into_value()?;

        let ready = report["timezone"]["text"]
            .as_str()
            .is_some_and(|text| !text.contains("Blocked") && text.lines().count() >= 4)
            && report["intl"]["text"]
                .as_str()
                .is_some_and(|text| !text.contains("Blocked") && text.lines().count() >= 4);
        if ready {
            break report;
        }
        if Instant::now() >= deadline {
            bail!("timed out waiting for the CreepJS Timezone and Intl reports");
        }
        sleep(Duration::from_millis(500)).await;
    };

    println!("Chromium: {version}");
    println!("{}", serde_json::to_string_pretty(&report)?);

    browser.close().await?;
    Ok(())
}
