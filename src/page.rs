use anyhow::Result;
use chromiumoxide::Page;

use crate::config::PatchSet;
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
        self.apply_profile_with(profile, &PatchSet::default()).await
    }

    pub async fn apply_profile_with(
        &self,
        profile: &BrowserProfile,
        patches: &PatchSet,
    ) -> Result<()> {
        if patches.locale_and_timezone {
            timezone_intl::apply(self, &profile.locale).await?;
        }
        if patches.identity {
            network::apply(self, &profile.navigator).await?;
            navigator::apply(self, &profile.navigator).await?;
        }
        if patches.screen {
            screen::apply(self, &profile.screen).await?;
        }
        if patches.device_environment {
            device_environment::apply(self, &profile.device_environment).await?;
        }

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
