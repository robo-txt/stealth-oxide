use anyhow::Result;
use chromiumoxide::{Browser, BrowserConfig};
use futures::StreamExt;
use stealth_oxide::{CookieSeed, PlatformProfile, ProfileSeed, StealthConfig, apply_profile_seeds};

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
    StealthConfig::for_platform(PlatformProfile::Linux)
        .apply(&page)
        .await?;

    let seeds = [ProfileSeed::new().cookie(
        CookieSeed::new("example-session", "demo", "https://example.com/")
            .secure(true)
            .http_only(true),
    )];
    let report = apply_profile_seeds(&page, &seeds).await?;

    page.goto("https://example.com").await?;
    println!("seeded cookies: {}", report.cookies);

    browser.close().await?;
    Ok(())
}
