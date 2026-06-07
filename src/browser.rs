use chromiumoxide::Browser;

pub struct StealthBrowser {
    browser: Browser,
    profile: BroswerProfile,
}

impl StealthBrowser {
    pub async fn launch(profile: BrowserProfile) -> Result<self> {
        let config = BrowserConfig::builder();
        //return struct of browserProfile not self
    }

    pub async fn new_page(&self, url: &str) -> Result<StealthPage> {
        let page = self.browser.new_page(url).await?;

        let stealth_page = StealthPage::new(page);

        //patches apply here
        Ok(StealthPage)
    }

    pub async fn close(self) -> Result<()> {
        self.browser.close().await?;
        Ok(())
    }
}
