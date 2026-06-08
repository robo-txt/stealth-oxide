use anyhow::Result;

use crate::page::StealthPage;
use crate::profiles::NavigatorProfile;

pub async fn apply(_page: &StealthPage, _profile: &NavigatorProfile) -> Result<()> {
    Ok(())
}
