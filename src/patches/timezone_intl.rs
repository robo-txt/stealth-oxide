use anyhow::{Context, Result};
use chromiumoxide::cdp::browser_protocol::emulation::{
    SetLocaleOverrideParams, SetTimezoneOverrideParams,
};

use crate::page::StealthPage;
use crate::profiles::LocaleProfile;

pub async fn apply(page: &StealthPage, profile: &LocaleProfile) -> Result<()> {
    page.inner()
        .execute(locale_params(profile))
        .await
        .context("failed to apply the CDP locale override")?;
    page.inner()
        .execute(timezone_params(profile)?)
        .await
        .context("failed to apply the CDP timezone override")?;

    Ok(())
}

fn locale_params(profile: &LocaleProfile) -> SetLocaleOverrideParams {
    SetLocaleOverrideParams::builder()
        .locale(profile.locale.clone())
        .build()
}

fn timezone_params(profile: &LocaleProfile) -> Result<SetTimezoneOverrideParams> {
    SetTimezoneOverrideParams::builder()
        .timezone_id(profile.timezone.clone())
        .build()
        .map_err(anyhow::Error::msg)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> LocaleProfile {
        LocaleProfile {
            locale: "en-US".to_string(),
            timezone: "America/New_York".to_string(),
        }
    }

    #[test]
    fn builds_locale_override() {
        assert_eq!(locale_params(&profile()).locale.as_deref(), Some("en-US"));
    }

    #[test]
    fn builds_timezone_override() -> Result<()> {
        assert_eq!(timezone_params(&profile())?.timezone_id, "America/New_York");
        Ok(())
    }
}
