use anyhow::Result;
use chromiumoxide::{Browser, BrowserConfig};
use futures::StreamExt;
use stealth_oxide::{BrowserProfileBuilder, ColorScheme, PlatformProfile, StealthConfig};

#[tokio::main]
async fn main() -> Result<()> {
    // Start with a coherent built-in identity, then customize related values
    // through the typed builder. `build` rejects contradictory profiles.
    let profile = BrowserProfileBuilder::new(PlatformProfile::Windows.profile())
        .name("Canadian Windows desktop")
        .locale("en-CA")
        .timezone("America/Toronto")
        .color_scheme(ColorScheme::Dark)
        .build()?;

    let browser_config = BrowserConfig::builder()
        .hide()
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
    let report = StealthConfig::from_profile(profile).apply(&page).await?;
    page.goto("https://example.com").await?;

    println!("applied coherent profile patches: {:?}", report.applied());
    browser.close().await?;
    Ok(())
}
