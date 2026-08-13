use std::time::Instant;

use anyhow::Result;
use chromiumoxide::{Browser, BrowserConfig};
use futures::StreamExt;
use stealth_oxide::{PlatformProfile, StealthConfig};

const TARGET_URL: &str = "https://example.com";

#[tokio::main]
async fn main() -> Result<()> {
    let browser_config = BrowserConfig::builder()
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
    let report = StealthConfig::for_platform(PlatformProfile::Windows)
        .apply(&page)
        .await?;

    let navigation_started = Instant::now();
    page.goto(TARGET_URL).await?;
    let navigation_time = navigation_started.elapsed();

    let title = page.get_title().await?.unwrap_or_default();
    let content_size = page.content().await?.len();

    println!("page title: {title}");
    println!("content size: {content_size} bytes");
    println!("navigation time: {navigation_time:.2?}");
    println!("applied patches: {:?}", report.applied());
    browser.close().await?;
    Ok(())
}
