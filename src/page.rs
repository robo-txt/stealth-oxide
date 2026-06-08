use anyhow::Result;
use chromiumoxide::Page;

use crate::patches::navigator;
use crate::profiles::BrowserProfile;

pub struct StealthPage {
    page: Page,
}

impl StealthPage {
    pub fn new(page: Page) -> Self {
        Self { page }
    }

    pub async fn apply_profile(&self, profile: &BrowserProfile) -> Result<()> {
        navigator::apply(self, &profile.navigator).await?;
        Ok(())
    }

    pub fn inner(&self) -> &Page {
        &self.page
    }
}
