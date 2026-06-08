use anyhow::Result;

use stealth_oxide::browser::StealthBrowser;
use stealth_oxide::profiles::chrome_windows::chrome_windows;

#[tokio::main]
async fn main() -> Result<()> {
    let profile = chrome_windows();

    let browser = StealthBrowser::launch(profile).await?;

    let page = browser.new_page("https://httpbin.org/headers").await?;

    let html = page.inner().content().await?;

    println!("{html}");

    browser.close().await?;

    Ok(())
}
