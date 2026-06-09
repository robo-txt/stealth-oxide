use anyhow::Result;
use chromiumoxide::cdp::browser_protocol::network::SetUserAgentOverrideParams;

use crate::page::StealthPage;
use crate::profiles::NavigatorProfile;

pub async fn apply(page: &StealthPage, profile: &NavigatorProfile) -> Result<()> {
    let params = SetUserAgentOverrideParams::builder()
        .user_agent(profile.user_agent.clone())
        .accept_language(profile.languages.join(","))
        .platform(profile.platform.clone())
        .build()
        .map_err(anyhow::Error::msg)?;

    page.inner().execute(params).await?;

    Ok(())
}
