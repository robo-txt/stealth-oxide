use chromiumoxide::Page;
use chromiumoxide::cdp::browser_protocol::emulation::{
    GetScreenInfosParams, ScreenId, ScreenOrientation, ScreenOrientationType,
    SetDeviceMetricsOverrideParams, WorkAreaInsets,
};
use chromiumoxide::types::{Command, Method, MethodId, MethodType};
use serde::Serialize;
use serde_json::Value;

use crate::error::{Error, Result};
use crate::profiles::ScreenConfig;

pub async fn apply(page: &Page, profile: &ScreenConfig) -> Result<()> {
    page.execute(params(profile)?)
        .await
        .map_err(|source| Error::cdp("screen metrics", source))?;

    let screen_infos = page
        .execute(GetScreenInfosParams::default())
        .await
        .map_err(|source| Error::cdp("screen information", source))?;
    let primary_screen = screen_infos
        .screen_infos
        .iter()
        .find(|screen| screen.is_primary)
        .map(|screen| screen.id.clone())
        .ok_or_else(|| Error::invalid_parameters("screen information", "primary screen missing"))?;

    let work_area = WorkAreaInsets::builder()
        .top(0)
        .left(0)
        .right((profile.width - profile.available_width) as i64)
        .bottom((profile.height - profile.available_height) as i64)
        .build();
    page.execute(
        UpdateScreenParams::builder()
            .screen_id(primary_screen)
            .left(0)
            .top(0)
            .width(profile.width as i64)
            .height(profile.height as i64)
            .work_area_insets(work_area)
            .device_pixel_ratio(profile.device_scale_factor)
            .color_depth(24)
            .internal(true)
            .build(),
    )
    .await
    .map_err(|source| Error::cdp("screen work area", source))?;

    Ok(())
}

#[derive(Debug, Serialize)]
struct UpdateScreenParams {
    #[serde(rename = "screenId")]
    screen_id: ScreenId,
    left: i64,
    top: i64,
    width: i64,
    height: i64,
    #[serde(rename = "workAreaInsets")]
    work_area_insets: WorkAreaInsets,
    #[serde(rename = "devicePixelRatio")]
    device_pixel_ratio: f64,
    #[serde(rename = "colorDepth")]
    color_depth: i64,
    #[serde(rename = "isInternal")]
    is_internal: bool,
}

#[derive(Default)]
struct UpdateScreenParamsBuilder {
    screen_id: Option<ScreenId>,
    left: Option<i64>,
    top: Option<i64>,
    width: Option<i64>,
    height: Option<i64>,
    work_area_insets: Option<WorkAreaInsets>,
    device_pixel_ratio: Option<f64>,
    color_depth: Option<i64>,
    is_internal: Option<bool>,
}

impl UpdateScreenParams {
    const IDENTIFIER: &'static str = "Emulation.updateScreen";

    fn builder() -> UpdateScreenParamsBuilder {
        UpdateScreenParamsBuilder::default()
    }
}

impl UpdateScreenParamsBuilder {
    fn screen_id(mut self, screen_id: impl Into<ScreenId>) -> Self {
        self.screen_id = Some(screen_id.into());
        self
    }

    fn left(mut self, left: impl Into<i64>) -> Self {
        self.left = Some(left.into());
        self
    }

    fn top(mut self, top: impl Into<i64>) -> Self {
        self.top = Some(top.into());
        self
    }

    fn width(mut self, width: impl Into<i64>) -> Self {
        self.width = Some(width.into());
        self
    }

    fn height(mut self, height: impl Into<i64>) -> Self {
        self.height = Some(height.into());
        self
    }

    fn work_area_insets(mut self, work_area_insets: WorkAreaInsets) -> Self {
        self.work_area_insets = Some(work_area_insets);
        self
    }

    fn device_pixel_ratio(mut self, device_pixel_ratio: impl Into<f64>) -> Self {
        self.device_pixel_ratio = Some(device_pixel_ratio.into());
        self
    }

    fn color_depth(mut self, color_depth: impl Into<i64>) -> Self {
        self.color_depth = Some(color_depth.into());
        self
    }

    fn internal(mut self, is_internal: impl Into<bool>) -> Self {
        self.is_internal = Some(is_internal.into());
        self
    }

    fn build(self) -> UpdateScreenParams {
        UpdateScreenParams {
            screen_id: self.screen_id.expect("screen id is required"),
            left: self.left.expect("screen left is required"),
            top: self.top.expect("screen top is required"),
            width: self.width.expect("screen width is required"),
            height: self.height.expect("screen height is required"),
            work_area_insets: self.work_area_insets.expect("work area is required"),
            device_pixel_ratio: self
                .device_pixel_ratio
                .expect("device pixel ratio is required"),
            color_depth: self.color_depth.expect("color depth is required"),
            is_internal: self.is_internal.expect("internal screen state is required"),
        }
    }
}

impl Method for UpdateScreenParams {
    fn identifier(&self) -> MethodId {
        Self::IDENTIFIER.into()
    }
}

impl MethodType for UpdateScreenParams {
    fn method_id() -> MethodId {
        Self::IDENTIFIER.into()
    }
}

impl Command for UpdateScreenParams {
    type Response = Value;
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

    #[test]
    fn builds_native_work_area_insets() {
        let work_area = WorkAreaInsets::builder()
            .top(0)
            .left(0)
            .right(80)
            .bottom(40)
            .build();

        assert_eq!(work_area.top, Some(0));
        assert_eq!(work_area.left, Some(0));
        assert_eq!(work_area.right, Some(80));
        assert_eq!(work_area.bottom, Some(40));
    }
}
