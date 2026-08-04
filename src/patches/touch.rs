use chromiumoxide::Page;
use chromiumoxide::cdp::browser_protocol::emulation::SetTouchEmulationEnabledParams;

use crate::error::{Error, Result};
use crate::profiles::TouchConfig;

pub async fn apply(page: &Page, config: &TouchConfig) -> Result<()> {
    page.execute(params(config)?)
        .await
        .map_err(|source| Error::cdp("touch emulation", source))?;
    Ok(())
}

fn params(config: &TouchConfig) -> Result<SetTouchEmulationEnabledParams> {
    let mut builder = SetTouchEmulationEnabledParams::builder().enabled(config.enabled);
    if config.enabled {
        builder = builder.max_touch_points(i64::from(config.max_touch_points));
    }
    builder
        .build()
        .map_err(|message| Error::invalid_parameters("touch emulation", message))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_non_touch_desktop_override() -> Result<()> {
        let params = params(&TouchConfig {
            enabled: false,
            max_touch_points: 0,
        })?;
        assert!(!params.enabled);
        assert_eq!(params.max_touch_points, None);
        Ok(())
    }
}
