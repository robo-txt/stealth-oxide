use chromiumoxide::Page;
use chromiumoxide::cdp::browser_protocol::emulation::{UserAgentBrandVersion, UserAgentMetadata};
use chromiumoxide::cdp::browser_protocol::network::SetUserAgentOverrideParams;

use crate::error::{Error, Result};
use crate::profiles::{BrandVersion, NavigatorProfile, UserAgentClientHintsProfile};

fn build_brand_version(brand: &BrandVersion) -> Result<UserAgentBrandVersion> {
    UserAgentBrandVersion::builder()
        .brand(brand.brand.clone())
        .version(brand.version.clone())
        .build()
        .map_err(|message| Error::invalid_parameters("user-agent metadata brand", message))
}

pub(crate) fn build_user_agent_metadata(
    client_hints: &UserAgentClientHintsProfile,
) -> Result<UserAgentMetadata> {
    let brands = client_hints
        .brands
        .iter()
        .map(build_brand_version)
        .collect::<Result<Vec<_>>>()?;

    let full_version_list = client_hints
        .full_version_list
        .iter()
        .map(build_brand_version)
        .collect::<Result<Vec<_>>>()?;

    let mut builder = UserAgentMetadata::builder()
        .brands(brands)
        .full_version_lists(full_version_list)
        .platform(client_hints.platform.clone())
        .platform_version(client_hints.platform_version.clone())
        .architecture(client_hints.architecture.clone())
        .bitness(client_hints.bitness.clone())
        .model(client_hints.model.clone())
        .mobile(client_hints.mobile);
    if let Some(wow64) = client_hints.wow64 {
        builder = builder.wow64(wow64);
    }
    if let Some(form_factors) = &client_hints.form_factors {
        builder = builder.form_factors(form_factors.iter().cloned());
    }
    builder
        .build()
        .map_err(|message| Error::invalid_parameters("user-agent metadata", message))
}

pub async fn apply(page: &Page, profile: &NavigatorProfile) -> Result<()> {
    let params = params(profile)?;
    page.execute(params)
        .await
        .map_err(|source| Error::cdp("identity", source))?;

    Ok(())
}

pub(crate) fn params(profile: &NavigatorProfile) -> Result<SetUserAgentOverrideParams> {
    let mut builder = SetUserAgentOverrideParams::builder()
        .user_agent(profile.user_agent.clone())
        .accept_language(profile.languages.join(","))
        .platform(profile.platform.clone());

    if let Some(client_hints) = &profile.client_hints {
        builder = builder.user_agent_metadata(build_user_agent_metadata(client_hints)?);
    }

    builder
        .build()
        .map_err(|message| Error::invalid_parameters("identity", message))
}
