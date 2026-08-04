use anyhow::Result;
use chromiumoxide::cdp::browser_protocol::emulation::{
    ScreenOrientation, ScreenOrientationType, SetDeviceMetricsOverrideParams,
};

use crate::page::StealthPage;
use crate::profiles::ScreenProfile;

pub async fn apply(page: &StealthPage, profile: &ScreenProfile) -> Result<()> {
    if matches!(
        std::env::var("STEALTH_OXIDE_USE_NATIVE_SCREEN").as_deref(),
        Ok("1") | Ok("true")
    ) {
        return Ok(());
    }

    page.inner().execute(params(profile)?).await?;

    Ok(())
}

fn params(profile: &ScreenProfile) -> Result<SetDeviceMetricsOverrideParams> {
    let orientation_type = if profile.width >= profile.height {
        ScreenOrientationType::LandscapePrimary
    } else {
        ScreenOrientationType::PortraitPrimary
    };

    SetDeviceMetricsOverrideParams::builder()
        .width(profile.width as i64)
        .height(profile.height as i64)
        .device_scale_factor(profile.device_scale_factor)
        .mobile(false)
        .screen_width(profile.width as i64)
        .screen_height(profile.height as i64)
        .screen_orientation(ScreenOrientation::new(orientation_type, 0))
        .build()
        .map_err(anyhow::Error::msg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_desktop_landscape_metrics() -> Result<()> {
        let profile = ScreenProfile {
            width: 1920,
            height: 1080,
            available_width: 1920,
            available_height: 1040,
            device_scale_factor: 1.25,
        };

        let params = params(&profile)?;

        assert_eq!(params.width, 1920);
        assert_eq!(params.height, 1080);
        assert_eq!(params.screen_width, Some(1920));
        assert_eq!(params.screen_height, Some(1080));
        assert_eq!(params.device_scale_factor, 1.25);
        assert!(!params.mobile);
        assert_eq!(
            params.screen_orientation,
            Some(ScreenOrientation::new(
                ScreenOrientationType::LandscapePrimary,
                0
            ))
        );

        Ok(())
    }

    #[test]
    fn derives_portrait_orientation_from_dimensions() -> Result<()> {
        let profile = ScreenProfile {
            width: 1080,
            height: 1920,
            available_width: 1080,
            available_height: 1880,
            device_scale_factor: 1.0,
        };

        assert_eq!(
            params(&profile)?.screen_orientation,
            Some(ScreenOrientation::new(
                ScreenOrientationType::PortraitPrimary,
                0
            ))
        );

        Ok(())
    }
}
