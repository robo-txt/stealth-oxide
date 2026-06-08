use chromiumoxide::Page;

pub struct StealthPage {
    page: Page,
}

impl StealthPage {
    pub fn new(page: Page) -> Self {
        Self { page }
    }

    pub fn inner(&self) -> &Page {
        &self.page
    }
}
