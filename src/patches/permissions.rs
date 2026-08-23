use chromiumoxide::Browser;
use chromiumoxide::cdp::browser_protocol::browser::{
    PermissionDescriptor, PermissionSetting as CdpPermissionSetting, SetPermissionParams,
};

use crate::error::{Error, Result};
use crate::profiles::{PermissionOverride, PermissionSetting};

/// Applies one native browser-context permission override.
pub async fn apply(browser: &Browser, permission: &PermissionOverride) -> Result<()> {
    browser
        .execute(params(permission)?)
        .await
        .map_err(|source| Error::cdp("permission", source))?;
    Ok(())
}

pub(crate) fn params(permission: &PermissionOverride) -> Result<SetPermissionParams> {
    let setting = match permission.setting {
        PermissionSetting::Granted => CdpPermissionSetting::Granted,
        PermissionSetting::Denied => CdpPermissionSetting::Denied,
        PermissionSetting::Prompt => CdpPermissionSetting::Prompt,
    };
    let mut builder = SetPermissionParams::builder()
        .permission(PermissionDescriptor::new(permission.name.clone()))
        .setting(setting);
    if let Some(origin) = &permission.origin {
        builder = builder.origin(origin.clone());
    }
    if let Some(origin) = &permission.embedded_origin {
        builder = builder.embedded_origin(origin.clone());
    }
    builder
        .build()
        .map_err(|message| Error::invalid_parameters("permission", message))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chromiumoxide::types::Method;

    #[test]
    fn builds_origin_scoped_permission_command() -> Result<()> {
        let permission = PermissionOverride::for_origin(
            "geolocation",
            PermissionSetting::Granted,
            "https://example.com",
        );
        let params = params(&permission)?;

        assert_eq!(params.permission.name, "geolocation");
        assert_eq!(params.origin.as_deref(), Some("https://example.com"));
        assert_eq!(params.identifier().as_ref(), "Browser.setPermission");
        Ok(())
    }

    #[test]
    fn preserves_each_native_permission_setting() -> Result<()> {
        for (setting, expected) in [
            (PermissionSetting::Granted, CdpPermissionSetting::Granted),
            (PermissionSetting::Denied, CdpPermissionSetting::Denied),
            (PermissionSetting::Prompt, CdpPermissionSetting::Prompt),
        ] {
            let permission =
                PermissionOverride::for_origin("notifications", setting, "https://example.com");
            assert_eq!(params(&permission)?.setting, expected);
        }
        Ok(())
    }
}
