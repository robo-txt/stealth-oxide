use anyhow::Result;

use stealth_oxide::{PlatformProfile, StealthBrowser, StealthConfig};

#[tokio::main]
async fn main() -> Result<()> {
    let config = StealthConfig::builder()
        .platform(PlatformProfile::Linux)
        .headful(true)
        .mesa(true)
        .build()?;

    let browser = StealthBrowser::launch_with(config).await?;

    let page = browser.new_page("https://httpbin.org/headers").await?;

    let html = page.inner().content().await?;

    println!("{html}");

    let user_agent_data: serde_json::Value = page
        .inner()
        .evaluate(
            r#"
          (async () => {
              if (!navigator.userAgentData) {
                  return { supported: false };
              }

              const highEntropy = await
              navigator.userAgentData.getHighEntropyValues([
                  "architecture",
                  "bitness",
                  "platform",
                  "platformVersion",
                  "fullVersionList",
                  "mobile",
                  "model"
              ]);

              return {
                  supported: true,
                  brands: navigator.userAgentData.brands,
                  mobile: navigator.userAgentData.mobile,
                  platform: navigator.userAgentData.platform,
                  highEntropy
              };
          })()
          "#,
        )
        .await?
        .into_value()?;
    println!("{}", serde_json::to_string_pretty(&user_agent_data)?);
    let vendor: String = page
        .inner()
        .evaluate("navigator.vendor")
        .await?
        .into_value()?;

    println!("navigator.vendor = {vendor}");
    browser.close().await?;

    Ok(())
}
