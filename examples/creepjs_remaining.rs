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
                    const section = title => {
                        const heading = [...document.querySelectorAll('strong')]
                            .find(element => element.textContent?.trim() === title);
                        const root = heading?.closest('[class*="col-"]');
                        return !root ? null : {
                            rejected: root.classList.contains('rejected'),
                            lies: root.classList.contains('lies') || !!root.querySelector('.lies'),
                            text: root.innerText?.trim() || ''
                        };
                    };
                    const liesText = document.body.innerText.match(/lies \(\d+\)/)?.[0] || null;
                    const rating = selector => Number(
                        (document.querySelector(selector)?.textContent || '')
                            .match(/(\d+)%/)?.[1]
                    );
                    return {
                        canvas: section('Canvas 2d'),
                        domRect: section('DOMRect'),
                        svg: section('SVGRect'),
                        resistance: section('Resistance'),
                        math: section('Math'),
                        engineErrors: section('Error'),
                        status: section('Status'),
                        prototypeLies: liesText,
                        headless: rating('.headless-rating'),
                        likeHeadless: rating('.like-headless-rating'),
                        stealth: rating('.stealth-rating')
                    };
                })()
                "#,
            )
            .await?
            .into_value()?;

        if report["canvas"]["text"]
            .as_str()
            .is_some_and(|text| !text.is_empty())
            && report["status"]["text"]
                .as_str()
                .is_some_and(|text| !text.is_empty())
            && report["headless"].as_u64().is_some()
        {
            break report;
        }
        if Instant::now() >= deadline {
            bail!("timed out waiting for remaining CreepJS results");
        }
        sleep(Duration::from_millis(500)).await;
    };

    println!("Chromium: {version}");
    println!("{}", serde_json::to_string_pretty(&report)?);
    browser.close().await?;
    Ok(())
}
