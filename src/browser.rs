use anyhow::Result;
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
        let config = BrowserConfig::builder()
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
        let page = self.browser.new_page(url).await?;

        let stealth_page = StealthPage::new(page);
        stealth_page.apply_profile(&self.profile).await?;
        //patches apply here
        Ok(stealth_page)
    }

    pub fn profile(&self) -> &BrowserProfile {
        &self.profile
    }

    pub async fn close(mut self) -> Result<()> {
        self.browser.close().await?;
        Ok(())
    }
}
