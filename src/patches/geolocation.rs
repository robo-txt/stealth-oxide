use chromiumoxide::Page;
use chromiumoxide::cdp::browser_protocol::emulation::SetGeolocationOverrideParams;

use crate::error::{Error, Result};
use crate::profiles::GeolocationConfig;

/// Applies Chromium's native geolocation position or unavailable-state override.
pub async fn apply(page: &Page, geolocation: &GeolocationConfig) -> Result<()> {
    page.execute(params(geolocation))
        .await
        .map_err(|source| Error::cdp("geolocation", source))?;
    Ok(())
}

pub(crate) fn params(geolocation: &GeolocationConfig) -> SetGeolocationOverrideParams {
    let mut builder = SetGeolocationOverrideParams::builder();
    if let Some(value) = geolocation.latitude {
        builder = builder.latitude(value);
    }
    if let Some(value) = geolocation.longitude {
        builder = builder.longitude(value);
    }
    if let Some(value) = geolocation.accuracy {
        builder = builder.accuracy(value);
    }
    if let Some(value) = geolocation.altitude {
        builder = builder.altitude(value);
    }
    if let Some(value) = geolocation.altitude_accuracy {
        builder = builder.altitude_accuracy(value);
    }
    if let Some(value) = geolocation.heading {
        builder = builder.heading(value);
    }
    if let Some(value) = geolocation.speed {
        builder = builder.speed(value);
    }
    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chromiumoxide::types::Method;

    #[test]
    fn builds_position_override_command() {
        let params = params(&GeolocationConfig::position(40.7128, -74.006, 25.0));

        assert_eq!(params.latitude, Some(40.7128));
        assert_eq!(params.longitude, Some(-74.006));
        assert_eq!(params.accuracy, Some(25.0));
        assert_eq!(
            params.identifier().as_ref(),
            "Emulation.setGeolocationOverride"
        );
    }

    #[test]
    fn builds_unavailable_override_without_coordinates() {
        let params = params(&GeolocationConfig::unavailable());

        assert_eq!(params.latitude, None);
        assert_eq!(params.longitude, None);
        assert_eq!(params.accuracy, None);
    }
}
