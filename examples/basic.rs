use anyhow::Result;
use chromiumoxide::{Browser, BrowserConfig};
use futures::StreamExt;
use stealth_oxide::{PlatformProfile, StealthConfig};

#[tokio::main]
async fn main() -> Result<()> {
    let profile = PlatformProfile::Linux.profile();
    let browser_config = BrowserConfig::builder()
        .with_head()
        .arg(("user-agent", profile.navigator().user_agent.as_str()))
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
    StealthConfig::from_profile(profile).apply(&page).await?;
    page.goto("https://example.com").await?;

    println!("{}", page.content().await?);
    browser.close().await?;
    Ok(())
}
