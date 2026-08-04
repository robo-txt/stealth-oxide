use chromiumoxide::Page;
use chromiumoxide::cdp::browser_protocol::emulation::SetLocaleOverrideParams;

use crate::error::{Error, Result};

pub async fn apply(page: &Page, locale: &str) -> Result<()> {
    page.execute(params(locale))
        .await
        .map_err(|source| Error::cdp("locale", source))?;
    Ok(())
}

fn params(locale: &str) -> SetLocaleOverrideParams {
    SetLocaleOverrideParams::builder().locale(locale).build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_locale_override() {
        assert_eq!(params("en-US").locale.as_deref(), Some("en-US"));
    }
}
