//! Runtime-neutral coordination for CDP targets created after a page is configured.
//!
//! Target auto-attachment pauses new targets before their code runs. Applications
//! must continuously drive the returned event stream and call
//! [`TargetCoordinator::apply`] for every event. The coordinator always resumes
//! the target, including unsupported target types, to avoid freezing browser work.

use chromiumoxide::Browser;
use chromiumoxide::Page;
use chromiumoxide::cdp::browser_protocol::emulation::{
    SetGeolocationOverrideParams, SetHardwareConcurrencyOverrideParams, SetLocaleOverrideParams,
    SetTimezoneOverrideParams,
};
use chromiumoxide::cdp::browser_protocol::network::SetUserAgentOverrideParams;
use chromiumoxide::cdp::browser_protocol::target::{
    EventAttachedToTarget, FilterEntry, SessionId, SetAutoAttachParams, TargetFilter,
};
use chromiumoxide::cdp::js_protocol::runtime::EvaluateParams as RuntimeEvaluateParams;
use chromiumoxide::cdp::js_protocol::runtime::RunIfWaitingForDebuggerParams;
use chromiumoxide::listeners::EventStream;
use chromiumoxide::types::{Command, Method, MethodId};
use serde::Serialize;
use serde_json::Value;

use crate::config::PatchMode;
use crate::{Error, Result, StealthConfig};

/// Creates a configured page and navigates it to the destination URL.
pub(crate) async fn new_page_with_profile(
    browser: &Browser,
    config: &StealthConfig,
    url: &str,
) -> Result<Page> {
    config.apply_browser(browser).await?;
    let page_config = config.clone().use_native(crate::Patch::Permissions);
    let page = browser
        .new_page("about:blank")
        .await
        .map_err(|source| Error::cdp("configured page creation", source))?;
    page_config.apply(&page).await?;
    page.goto(url)
        .await
        .map_err(|source| Error::cdp("configured page navigation", source))?;
    // Chromium can reset target-scoped emulation during a cross-origin load.
    page_config.apply(&page).await?;
    Ok(page)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SendMessageToTargetParams {
    message: String,
    session_id: SessionId,
}

impl Method for SendMessageToTargetParams {
    fn identifier(&self) -> MethodId {
        "Target.sendMessageToTarget".into()
    }
}

impl Command for SendMessageToTargetParams {
    type Response = Value;
}

/// Result of handling one newly attached target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetApplyReport {
    target_type: String,
    supported: bool,
    applied_commands: usize,
}

impl TargetApplyReport {
    /// Browser-reported target type such as `worker` or `iframe`.
    pub fn target_type(&self) -> &str {
        &self.target_type
    }

    /// Whether this target type receives the configured emulation commands.
    pub const fn supported(&self) -> bool {
        self.supported
    }

    /// Number of emulation commands sent before the target was resumed.
    pub const fn applied_commands(&self) -> usize {
        self.applied_commands
    }
}

/// Applies target-scoped identity, locale, timezone, hardware, and geolocation
/// commands to new targets.
#[derive(Debug, Clone)]
pub struct TargetCoordinator {
    locale: Option<SetLocaleOverrideParams>,
    timezone: Option<SetTimezoneOverrideParams>,
    identity: Option<SetUserAgentOverrideParams>,
    worker_platform: Option<String>,
    hardware_concurrency: Option<SetHardwareConcurrencyOverrideParams>,
    geolocation: Option<SetGeolocationOverrideParams>,
}

impl TargetCoordinator {
    /// Builds target commands from the enabled override portions of a configuration.
    pub fn new(config: &StealthConfig) -> Result<Self> {
        let locale = match config.locale_mode() {
            PatchMode::Override(value) => Some(crate::patches::locale::params(value)),
            PatchMode::Native | PatchMode::Disabled => None,
        };
        let timezone = match config.timezone_mode() {
            PatchMode::Override(value) => Some(crate::patches::timezone::params(value)?),
            PatchMode::Native | PatchMode::Disabled => None,
        };
        let identity = match config.identity_mode() {
            PatchMode::Override(value) => Some(target_identity_params(value)?),
            PatchMode::Native | PatchMode::Disabled => None,
        };
        let worker_platform = match config.identity_mode() {
            PatchMode::Override(value) => Some(value.platform.clone()),
            PatchMode::Native | PatchMode::Disabled => None,
        };
        let hardware_concurrency = match config.hardware_concurrency_mode() {
            PatchMode::Override(value) => Some(crate::patches::hardware::params(*value)?),
            PatchMode::Native | PatchMode::Disabled => None,
        };
        let geolocation = match config.geolocation_mode() {
            PatchMode::Override(value) => Some(crate::patches::geolocation::params(value)),
            PatchMode::Native | PatchMode::Disabled => None,
        };
        Ok(Self {
            locale,
            timezone,
            identity,
            worker_platform,
            hardware_concurrency,
            geolocation,
        })
    }

    /// Subscribes to attachment events, then enables related-target pausing.
    ///
    /// The returned stream must be driven for as long as auto-attach remains
    /// enabled. Dropping it without first calling [`Self::disable`] can leave a
    /// newly created target waiting for the debugger.
    pub async fn enable(&self, page: &Page) -> Result<EventStream<EventAttachedToTarget>> {
        let events = page
            .event_listener::<EventAttachedToTarget>()
            .await
            .map_err(|source| Error::cdp("target event subscription", source))?;
        let params = SetAutoAttachParams::builder()
            .auto_attach(true)
            .wait_for_debugger_on_start(true)
            .flatten(false)
            .filter(TargetFilter::new(vec![
                FilterEntry::builder()
                    .r#type("service_worker")
                    .exclude(true)
                    .build(),
                FilterEntry::default(),
            ]))
            .build()
            .map_err(|message| Error::invalid_parameters("target auto-attach", message))?;
        page.execute(params)
            .await
            .map_err(|source| Error::cdp("target auto-attach", source))?;
        Ok(events)
    }

    /// Disables automatic attachment for targets related to this page.
    pub async fn disable(&self, page: &Page) -> Result<()> {
        page.execute(SetAutoAttachParams::new(false, false))
            .await
            .map_err(|source| Error::cdp("target auto-attach", source))?;
        Ok(())
    }

    /// Number of target-scoped emulation commands configured for a page target.
    ///
    /// Geolocation is intentionally sent only to page and OOPIF targets;
    /// worker targets receive the identity and hardware commands they support.
    pub fn configured_commands(&self) -> usize {
        usize::from(self.locale.is_some())
            + usize::from(self.timezone.is_some())
            + usize::from(self.identity.is_some())
            + usize::from(self.hardware_concurrency.is_some())
            + usize::from(self.geolocation.is_some())
    }

    /// Applies configured commands to one paused target and always resumes it.
    pub async fn apply(
        &self,
        page: &Page,
        event: &EventAttachedToTarget,
    ) -> Result<TargetApplyReport> {
        let target_type = event.target_info.r#type.clone();
        let supported = matches!(
            target_type.as_str(),
            "page" | "iframe" | "worker" | "shared_worker"
        );
        let mut next_id = 1_u64;
        let mut applied_commands = 0;
        let mut command_error = None;

        if supported {
            if let Some(command) = &self.locale {
                match send_nested(page, &event.session_id, next_id, command).await {
                    Ok(()) => applied_commands += 1,
                    Err(error) => command_error = Some(error),
                }
                next_id += 1;
            }
            if command_error.is_none() {
                if let Some(command) = &self.timezone {
                    match send_nested(page, &event.session_id, next_id, command).await {
                        Ok(()) => applied_commands += 1,
                        Err(error) => command_error = Some(error),
                    }
                    next_id += 1;
                }
            }
            if command_error.is_none() {
                if let Some(command) = &self.identity {
                    match send_nested(page, &event.session_id, next_id, command).await {
                        Ok(()) => applied_commands += 1,
                        Err(error) => command_error = Some(error),
                    }
                    next_id += 1;
                }
            }
            if command_error.is_none() && matches!(target_type.as_str(), "worker" | "shared_worker")
            {
                if let Some(platform) = &self.worker_platform {
                    let expression = worker_platform_expression(platform)?;
                    let command = RuntimeEvaluateParams::new(expression);
                    match send_nested(page, &event.session_id, next_id, &command).await {
                        Ok(()) => applied_commands += 1,
                        Err(error) => command_error = Some(error),
                    }
                    next_id += 1;
                }
            }
            if command_error.is_none() {
                if let Some(command) = &self.hardware_concurrency {
                    match send_nested(page, &event.session_id, next_id, command).await {
                        Ok(()) => applied_commands += 1,
                        Err(error) => command_error = Some(error),
                    }
                    next_id += 1;
                }
            }
            if command_error.is_none() && matches!(target_type.as_str(), "page" | "iframe") {
                if let Some(command) = &self.geolocation {
                    match send_nested(page, &event.session_id, next_id, command).await {
                        Ok(()) => applied_commands += 1,
                        Err(error) => command_error = Some(error),
                    }
                    next_id += 1;
                }
            }
        }

        let resume_result = send_nested(
            page,
            &event.session_id,
            next_id,
            &RunIfWaitingForDebuggerParams::default(),
        )
        .await;

        if let Some(error) = command_error {
            return Err(error);
        }
        resume_result?;

        Ok(TargetApplyReport {
            target_type,
            supported,
            applied_commands,
        })
    }
}

fn worker_platform_expression(platform: &str) -> Result<String> {
    let platform = serde_json::to_string(platform).map_err(Error::target_command_json)?;
    Ok(format!(
        "(() => {{ Object.defineProperty(navigator, 'platform', {{ configurable: true, enumerable: true, value: {platform} }}); }})()"
    ))
}

fn target_identity_params(
    profile: &crate::profiles::NavigatorProfile,
) -> Result<SetUserAgentOverrideParams> {
    let mut builder = SetUserAgentOverrideParams::builder()
        .user_agent(profile.user_agent.clone())
        .accept_language(profile.languages.join(","))
        .platform(profile.platform.clone());
    if let Some(hints) = &profile.client_hints {
        builder = builder
            .user_agent_metadata(crate::patches::identity::build_user_agent_metadata(hints)?);
    }
    builder
        .build()
        .map_err(|message| Error::invalid_parameters("target identity", message))
}

async fn send_nested<T: Method + Serialize>(
    page: &Page,
    session_id: &SessionId,
    id: u64,
    command: &T,
) -> Result<()> {
    let message = serde_json::to_string(&serde_json::json!({
        "id": id,
        "method": command.identifier(),
        "params": command,
    }))
    .map_err(Error::target_command_json)?;
    page.execute(SendMessageToTargetParams {
        message,
        session_id: session_id.clone(),
    })
    .await
    .map_err(|source| Error::cdp("attached target", source))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{GeolocationConfig, PlatformProfile};

    #[test]
    fn complete_profile_configures_locale_timezone_identity_and_hardware() -> Result<()> {
        let config = StealthConfig::for_platform(PlatformProfile::Linux);
        assert_eq!(TargetCoordinator::new(&config)?.configured_commands(), 3);
        let config = config.hardware_concurrency(8);
        assert_eq!(TargetCoordinator::new(&config)?.configured_commands(), 4);
        let config = config.geolocation(GeolocationConfig::position(40.0, -74.0, 25.0));
        assert_eq!(TargetCoordinator::new(&config)?.configured_commands(), 5);
        Ok(())
    }

    #[test]
    fn native_and_disabled_modes_do_not_emit_target_commands() -> Result<()> {
        let config = StealthConfig::none();
        assert_eq!(TargetCoordinator::new(&config)?.configured_commands(), 0);
        Ok(())
    }
}
