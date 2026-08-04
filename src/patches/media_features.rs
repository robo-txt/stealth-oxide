use chromiumoxide::Page;
use chromiumoxide::cdp::browser_protocol::emulation::{MediaFeature, SetEmulatedMediaParams};

use crate::error::{Error, Result};
use crate::profiles::MediaFeaturesConfig;

pub async fn apply(page: &Page, config: &MediaFeaturesConfig) -> Result<()> {
    page.execute(params(config))
        .await
        .map_err(|source| Error::cdp("media features", source))?;
    Ok(())
}

fn params(config: &MediaFeaturesConfig) -> SetEmulatedMediaParams {
    SetEmulatedMediaParams::builder()
        .features([
            MediaFeature::new("prefers-color-scheme", config.color_scheme.as_str()),
            MediaFeature::new("prefers-reduced-motion", config.reduced_motion.as_str()),
            MediaFeature::new("forced-colors", config.forced_colors.as_str()),
            MediaFeature::new("color-gamut", config.color_gamut.as_str()),
            MediaFeature::new("monochrome", config.monochrome.to_string()),
        ])
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profiles::{ColorGamut, ColorScheme, ForcedColors, ReducedMotion};

    #[test]
    fn builds_desktop_media_features() {
        let config = MediaFeaturesConfig {
            color_scheme: ColorScheme::Dark,
            reduced_motion: ReducedMotion::NoPreference,
            forced_colors: ForcedColors::None,
            color_gamut: ColorGamut::Srgb,
            monochrome: 0,
        };
        let features = params(&config)
            .features
            .expect("media features should be set");
        assert!(features.contains(&MediaFeature::new("prefers-color-scheme", "dark")));
        assert!(features.contains(&MediaFeature::new("monochrome", "0")));
    }
}
