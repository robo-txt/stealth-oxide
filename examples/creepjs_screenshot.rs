use std::time::Duration;

use anyhow::{Result, bail};
use chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat;
use chromiumoxide::page::ScreenshotParams;
use chromiumoxide::{Browser, BrowserConfig};
use futures::StreamExt;
use serde_json::Value;
use stealth_oxide::{Patch, PlatformProfile, StealthConfig};
use tokio::time::{Instant, sleep};

const CREEPJS_URL: &str = "https://abrahamjuliot.github.io/creepjs/";
const OUTPUT: &str = "docs/images/creepjs-full-page.png";

#[tokio::main]
async fn main() -> Result<()> {
    let profile = PlatformProfile::Windows.profile();
    let browser_config = BrowserConfig::builder()
        .hide()
        .with_head()
        .arg(("user-agent", profile.navigator().user_agent.as_str()))
        .arg(("use-gl", "angle"))
        .arg(("use-angle", "gl"))
        .arg("ignore-gpu-blocklist")
        .arg("enable-gpu-rasterization")
        .build()
        .map_err(anyhow::Error::msg)?;
    let (mut browser, mut handler) = Browser::launch(browser_config).await?;

    tokio::spawn(async move {
        while let Some(event) = handler.next().await {
            if let Err(error) = event {
                eprintln!("chromiumoxide handler error: {error:?}");
            }
        }
    });

    let page = browser.new_page("about:blank").await?;
    StealthConfig::from_profile(profile)
        .use_native(Patch::Screen)
        .apply(&page)
        .await?;
    page.goto(CREEPJS_URL).await?;

    wait_for_complete_report(&page).await?;
    redact_publish_sensitive_values(&page).await?;
    page.save_screenshot(
        ScreenshotParams::builder()
            .format(CaptureScreenshotFormat::Png)
            .full_page(true)
            .build(),
        OUTPUT,
    )
    .await?;

    println!("saved {OUTPUT}");
    browser.close().await?;
    Ok(())
}

async fn redact_publish_sensitive_values(page: &chromiumoxide::Page) -> Result<()> {
    page.evaluate(
        r#"
        (() => {
            for (const element of document.querySelectorAll('*')) {
                if (element.childElementCount !== 0) continue;
                const text = element.textContent?.trim() || '';
                if (text.startsWith('FP ID:')) element.textContent = 'FP ID: [redacted]';
                if (text.startsWith('Fuzzy:')) element.textContent = 'Fuzzy: [redacted]';
            }
            const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
            while (walker.nextNode()) {
                const node = walker.currentNode;
                node.textContent = node.textContent
                    .replace(/\b(?:\d{1,3}\.){3}\d{1,3}\b/g, '[IP redacted]')
                    .replace(/\b[a-f0-9]{32,}\b/gi, '[fingerprint redacted]');
            }
        })()
        "#,
    )
    .await?;
    Ok(())
}

async fn wait_for_complete_report(page: &chromiumoxide::Page) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(120);
    let mut stable_height = None;
    let mut stable_polls = 0;

    loop {
        let state: Value = page
            .evaluate(
                r#"
                (() => {
                    const sectionReady = title => {
                        const heading = [...document.querySelectorAll('strong')]
                            .find(element => element.textContent?.trim() === title);
                        const root = heading?.closest('[class*="col-"]');
                        return (root?.innerText?.trim().length || 0) > title.length;
                    };
                    const hasPercent = selector => /\d+%/.test(
                        document.querySelector(selector)?.textContent || ''
                    );
                    const height = Math.max(
                        document.body?.scrollHeight || 0,
                        document.documentElement?.scrollHeight || 0
                    );
                    return {
                        ready: document.readyState === 'complete',
                        hasRatings: hasPercent('.headless-rating') &&
                            hasPercent('.stealth-rating'),
                        hasReport: sectionReady('Canvas 2d') &&
                            sectionReady('WebGL') && sectionReady('Status'),
                        height
                    };
                })()
                "#,
            )
            .await?
            .into_value()?;

        let height = state["height"].as_u64();
        if height.is_some() && height == stable_height {
            stable_polls += 1;
        } else {
            stable_height = height;
            stable_polls = 0;
        }

        if state["ready"] == true
            && state["hasRatings"] == true
            && state["hasReport"] == true
            && stable_polls >= 4
        {
            // Allow late fonts and decoded images one final moment to paint.
            sleep(Duration::from_secs(2)).await;
            return Ok(());
        }

        if Instant::now() >= deadline {
            bail!("timed out waiting for the complete CreepJS report");
        }
        sleep(Duration::from_millis(500)).await;
    }
}
