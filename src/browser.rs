use anyhow::Result;
use chromiumoxide::cdp::browser_protocol::system_info::{GetInfoParams, GetInfoReturns};
use chromiumoxide::{Browser, BrowserConfig};
use futures::StreamExt;

use crate::page::StealthPage;
use crate::profiles::BrowserProfile;

pub struct StealthBrowser {
    browser: Browser,
    profile: BrowserProfile,
}

impl StealthBrowser {
    pub async fn launch(profile: BrowserProfile) -> Result<Self> {
        // Apply launch-scoped identity inputs before any page or worker target
        // exists. Page-scoped CDP user-agent overrides do not reach workers.
        let config = BrowserConfig::builder()
            .hide()
            .arg(("user-agent", profile.navigator.user_agent.as_str()))
            .build()
            .map_err(anyhow::Error::msg)?;

        let (browser, mut handler) = Browser::launch(config).await?;

        tokio::spawn(async move {
            while let Some(event) = handler.next().await {
                if let Err(err) = event {
                    eprintln!("chromiumoxide handler error: {err:?}");
                }
            }
        });

        Ok(Self { browser, profile })
    }

    pub async fn new_page(&self, url: &str) -> Result<StealthPage> {
        let page = self.browser.new_page("about:blank").await?;

        let stealth_page = StealthPage::new(page);
        stealth_page.apply_profile(&self.profile).await?;
        stealth_page.goto(url).await?;

        //patches apply here
        Ok(stealth_page)
    }

    pub fn profile(&self) -> &BrowserProfile {
        &self.profile
    }

    pub async fn version(&self) -> Result<String> {
        Ok(self.browser.version().await?.product)
    }

    pub async fn system_info(&self) -> Result<GetInfoReturns> {
        Ok(self.browser.execute(GetInfoParams {}).await?.result)
    }

    pub async fn close(mut self) -> Result<()> {
        self.browser.close().await?;
        Ok(())
    }
}
