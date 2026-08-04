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

fn build_user_agent_metadata(
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

    UserAgentMetadata::builder()
        .brands(brands)
        .full_version_lists(full_version_list)
        .platform(client_hints.platform.clone())
        .platform_version(client_hints.platform_version.clone())
        .architecture(client_hints.architecture.clone())
        .bitness(client_hints.bitness.clone())
        .model(client_hints.model.clone())
        .mobile(client_hints.mobile)
        .build()
        .map_err(|message| Error::invalid_parameters("user-agent metadata", message))
}

pub async fn apply(page: &Page, profile: &NavigatorProfile) -> Result<()> {
    let mut builder = SetUserAgentOverrideParams::builder()
        .user_agent(profile.user_agent.clone())
        .accept_language(profile.languages.join(","))
        .platform(profile.platform.clone());

    if let Some(client_hints) = &profile.client_hints {
        builder = builder.user_agent_metadata(build_user_agent_metadata(client_hints)?);
    }

    let params = builder
        .build()
        .map_err(|message| Error::invalid_parameters("identity", message))?;
    page.execute(params)
        .await
        .map_err(|source| Error::cdp("identity", source))?;

    Ok(())
}
