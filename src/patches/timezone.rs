use chromiumoxide::Page;
use chromiumoxide::cdp::browser_protocol::emulation::SetTimezoneOverrideParams;

use crate::error::{Error, Result};

pub async fn apply(page: &Page, timezone: &str) -> Result<()> {
    page.execute(params(timezone)?)
        .await
        .map_err(|source| Error::cdp("timezone", source))?;
    Ok(())
}

fn params(timezone: &str) -> Result<SetTimezoneOverrideParams> {
    SetTimezoneOverrideParams::builder()
        .timezone_id(timezone)
        .build()
        .map_err(|message| Error::invalid_parameters("timezone", message))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_timezone_override() -> Result<()> {
        assert_eq!(params("America/New_York")?.timezone_id, "America/New_York");
        Ok(())
    }
}
