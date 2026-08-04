use anyhow::Result;
use chromiumoxide::Page;

use crate::patches::{device_environment, navigator, network, screen, timezone_intl};
use crate::profiles::BrowserProfile;

pub struct StealthPage {
    page: Page,
}

impl StealthPage {
    pub fn new(page: Page) -> Self {
        Self { page }
    }

    pub async fn apply_profile(&self, profile: &BrowserProfile) -> Result<()> {
        timezone_intl::apply(self, &profile.locale).await?;
        network::apply(self, &profile.navigator).await?;
        navigator::apply(self, &profile.navigator).await?;
        screen::apply(self, &profile.screen).await?;
        device_environment::apply(self, &profile.device_environment).await?;

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
