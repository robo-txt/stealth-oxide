use chromiumoxide::Page;
use chromiumoxide::cdp::browser_protocol::emulation::{
    ScreenOrientation, ScreenOrientationType, SetDeviceMetricsOverrideParams,
};
use chromiumoxide::cdp::browser_protocol::page::AddScriptToEvaluateOnNewDocumentParams;

use crate::error::{Error, Result};
use crate::profiles::ScreenConfig;

pub async fn apply(page: &Page, profile: &ScreenConfig) -> Result<()> {
    page.execute(params(profile)?)
        .await
        .map_err(|source| Error::cdp("screen metrics", source))?;

    let script = available_dimensions_script(profile);
    page.execute(AddScriptToEvaluateOnNewDocumentParams::new(&script))
        .await
        .map_err(|source| Error::cdp("screen available dimensions initialization", source))?;
    page.evaluate(script)
        .await
        .map_err(|source| Error::cdp("screen available dimensions", source))?;

    Ok(())
}

fn available_dimensions_script(profile: &ScreenConfig) -> String {
    format!(
        r#"(() => {{
            const screen = window.screen;
            Object.defineProperty(screen, 'availWidth', {{
                configurable: true,
                enumerable: true,
                value: {},
            }});
            Object.defineProperty(screen, 'availHeight', {{
                configurable: true,
                enumerable: true,
                value: {},
            }});
        }})()"#,
        profile.available_width, profile.available_height
    )
}

fn params(profile: &ScreenConfig) -> Result<SetDeviceMetricsOverrideParams> {
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
        .map_err(|message| Error::invalid_parameters("screen metrics", message))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_desktop_landscape_metrics() -> Result<()> {
        let profile = ScreenConfig {
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
        let profile = ScreenConfig {
            width: 1080,
            height: 1920,
            available_width: 1080,
            available_height: 1920,
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
