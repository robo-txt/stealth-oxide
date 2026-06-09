use anyhow::Result;
use chromiumoxide::Page;

use crate::patches::{navigator, network};
use crate::profiles::BrowserProfile;

pub struct StealthPage {
    page: Page,
}

impl StealthPage {
    pub fn new(page: Page) -> Self {
        Self { page }
    }

    pub async fn apply_profile(&self, profile: &BrowserProfile) -> Result<()> {
        network::apply(self, &profile.navigator).await?;
        navigator::apply(self, &profile.navigator).await?;
        Ok(())
    }

    pub async fn goto(&self, url: &str) -> Result<()> {
        self.page.goto(url).await?;
        Ok(())
    }
    pub fn inner(&self) -> &Page {
        &self.page
    }
}
