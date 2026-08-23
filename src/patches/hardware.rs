use chromiumoxide::Page;
use chromiumoxide::cdp::browser_protocol::emulation::SetHardwareConcurrencyOverrideParams;

use crate::error::{Error, Result};

/// Applies Chromium's native hardware-concurrency override.
pub async fn apply(page: &Page, hardware_concurrency: u32) -> Result<()> {
    page.execute(params(hardware_concurrency)?)
        .await
        .map_err(|source| Error::cdp("hardware concurrency", source))?;
    Ok(())
}

pub(crate) fn params(hardware_concurrency: u32) -> Result<SetHardwareConcurrencyOverrideParams> {
    SetHardwareConcurrencyOverrideParams::builder()
        .hardware_concurrency(i64::from(hardware_concurrency))
        .build()
        .map_err(|message| Error::invalid_parameters("hardware concurrency", message))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chromiumoxide::types::Method;

    #[test]
    fn builds_native_override_command() -> Result<()> {
        let params = params(8)?;

        assert_eq!(params.hardware_concurrency, 8);
        assert_eq!(
            params.identifier().as_ref(),
            "Emulation.setHardwareConcurrencyOverride"
        );
        Ok(())
    }
}
