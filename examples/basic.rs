use anyhow::Result;
use chromiumoxide::{Browser, BrowserConfig};
use futures::StreamExt;
use stealth_oxide::StealthConfig;

#[tokio::main]
async fn main() -> Result<()> {
    let browser_config = BrowserConfig::builder()
        .with_head()
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
    let report = StealthConfig::recommended().apply(&page).await?;
    page.goto("https://example.com").await?;

    println!("applied patches: {:?}", report.applied());
    println!("{}", page.content().await?);
    browser.close().await?;
    Ok(())
}
