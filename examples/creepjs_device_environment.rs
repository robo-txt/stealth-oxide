use std::time::Duration;

use anyhow::{Context, Result, bail};
use chromiumoxide::cdp::browser_protocol::emulation::ClearDeviceMetricsOverrideParams;
use serde_json::Value;
use tokio::time::{Instant, sleep};

use stealth_oxide::browser::StealthBrowser;
use stealth_oxide::profiles::chrome_windows::chrome_windows;

const CREEPJS_URL: &str = "https://abrahamjuliot.github.io/creepjs/";

#[tokio::main]
async fn main() -> Result<()> {
    let browser = StealthBrowser::launch(chrome_windows()).await?;
    let page = browser
        .new_page(CREEPJS_URL)
        .await
        .context("failed to open CreepJS")?;
    if matches!(
        std::env::var("STEALTH_OXIDE_CLEAR_SCREEN_OVERRIDE").as_deref(),
        Ok("1") | Ok("true")
    ) {
        page.inner()
            .execute(ClearDeviceMetricsOverrideParams {})
            .await?;
        page.goto(CREEPJS_URL).await?;
    }
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
                            text: root.innerText?.trim() || ''
                        };
                    };
                    const rating = selector => Number(
                        (document.querySelector(selector)?.textContent || '')
                            .match(/(\d+)%/)?.[1]
                    );
                    return {
                        screen: section('Screen'),
                        cssMedia: section('CSS Media Queries'),
                        css: section('CSS'),
                        fonts: section('Fonts'),
                        speech: section('Speech'),
                        dimensions: {
                            screenWidth: screen.width,
                            screenHeight: screen.height,
                            availWidth: screen.availWidth,
                            availHeight: screen.availHeight,
                            outerWidth,
                            outerHeight,
                            innerWidth,
                            innerHeight
                        },
                        headless: rating('.headless-rating'),
                        likeHeadless: rating('.like-headless-rating'),
                        stealth: rating('.stealth-rating')
                    };
                })()
                "#,
            )
            .await?
            .into_value()?;

        if report["screen"]["text"]
            .as_str()
            .is_some_and(|text| !text.is_empty())
            && report["cssMedia"]["text"]
                .as_str()
                .is_some_and(|text| !text.is_empty())
            && report["headless"].as_u64().is_some()
        {
            break report;
        }
        if Instant::now() >= deadline {
            bail!("timed out waiting for CreepJS device-environment results");
        }
        sleep(Duration::from_millis(500)).await;
    };

    println!("{}", serde_json::to_string_pretty(&report)?);
    browser.close().await?;
    Ok(())
}
