use anyhow::{Context, Result};
use chromiumoxide::cdp::browser_protocol::emulation::{
    MediaFeature, SetEmulatedMediaParams, SetTouchEmulationEnabledParams,
};

use crate::page::StealthPage;
use crate::profiles::DeviceEnvironmentProfile;

pub async fn apply(page: &StealthPage, profile: &DeviceEnvironmentProfile) -> Result<()> {
    page.inner()
        .execute(media_params(profile))
        .await
        .context("failed to apply CDP media-feature overrides")?;
    page.inner()
        .execute(touch_params(profile)?)
        .await
        .context("failed to apply the CDP touch override")?;

    Ok(())
}

fn media_params(profile: &DeviceEnvironmentProfile) -> SetEmulatedMediaParams {
    SetEmulatedMediaParams::builder()
        .features([
            MediaFeature::new("prefers-color-scheme", &profile.color_scheme),
            MediaFeature::new("prefers-reduced-motion", &profile.reduced_motion),
            MediaFeature::new("forced-colors", &profile.forced_colors),
            MediaFeature::new("color-gamut", &profile.color_gamut),
            MediaFeature::new("monochrome", &profile.monochrome),
        ])
        .build()
}

fn touch_params(profile: &DeviceEnvironmentProfile) -> Result<SetTouchEmulationEnabledParams> {
    let mut builder = SetTouchEmulationEnabledParams::builder().enabled(profile.touch_enabled);
    if profile.touch_enabled {
        builder = builder.max_touch_points(i64::from(profile.max_touch_points));
    }
    builder.build().map_err(anyhow::Error::msg)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> DeviceEnvironmentProfile {
        DeviceEnvironmentProfile {
            color_scheme: "dark".to_string(),
            reduced_motion: "no-preference".to_string(),
            forced_colors: "none".to_string(),
            color_gamut: "srgb".to_string(),
            monochrome: "0".to_string(),
            touch_enabled: false,
            max_touch_points: 0,
        }
    }

    #[test]
    fn builds_desktop_media_features() {
        let params = media_params(&profile());
        let features = params.features.expect("media features should be set");

        assert!(features.contains(&MediaFeature::new("prefers-color-scheme", "dark")));
        assert!(features.contains(&MediaFeature::new(
            "prefers-reduced-motion",
            "no-preference"
        )));
        assert!(features.contains(&MediaFeature::new("monochrome", "0")));
    }

    #[test]
    fn builds_non_touch_desktop_override() -> Result<()> {
        let params = touch_params(&profile())?;

        assert!(!params.enabled);
        assert_eq!(params.max_touch_points, None);
        Ok(())
    }
}
