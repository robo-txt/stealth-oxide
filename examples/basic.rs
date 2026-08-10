use anyhow::Result;
use chromiumoxide::{Browser, BrowserConfig};
use futures::StreamExt;
use stealth_oxide::{PlatformProfile, StealthConfig};

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
    let report = StealthConfig::for_platform(PlatformProfile::Linux)
        .apply(&page)
        .await?;
    page.goto("https://example.com").await?;

    println!(
        "page title: {}",
        page.get_title().await?.unwrap_or_default()
    );
    println!("applied patches: {:?}", report.applied());
    browser.close().await?;
    Ok(())
}
