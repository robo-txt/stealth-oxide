//! Runtime-neutral coordination for CDP targets created after a page is configured.
//!
//! Target auto-attachment pauses new targets before their code runs. Applications
//! must continuously drive the returned event stream and call
//! [`TargetCoordinator::apply`] for every event. The coordinator always resumes
//! the target, including unsupported target types, to avoid freezing browser work.

use chromiumoxide::Page;
use chromiumoxide::cdp::browser_protocol::emulation::{
    SetLocaleOverrideParams, SetTimezoneOverrideParams, SetUserAgentOverrideParams,
};
use chromiumoxide::cdp::browser_protocol::target::{
    EventAttachedToTarget, FilterEntry, SessionId, SetAutoAttachParams, TargetFilter,
};
use chromiumoxide::cdp::js_protocol::runtime::RunIfWaitingForDebuggerParams;
use chromiumoxide::listeners::EventStream;
use chromiumoxide::types::{Command, Method, MethodId};
use serde::Serialize;
use serde_json::Value;

use crate::config::PatchMode;
use crate::{Error, Result, StealthConfig};

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

/// Applies target-scoped identity, locale, and timezone commands to new targets.
#[derive(Debug, Clone)]
pub struct TargetCoordinator {
    locale: Option<SetLocaleOverrideParams>,
    timezone: Option<SetTimezoneOverrideParams>,
    identity: Option<SetUserAgentOverrideParams>,
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
        Ok(Self {
            locale,
            timezone,
            identity,
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

    /// Number of emulation commands configured for each supported target.
    pub fn configured_commands(&self) -> usize {
        usize::from(self.locale.is_some())
            + usize::from(self.timezone.is_some())
            + usize::from(self.identity.is_some())
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
    use crate::PlatformProfile;

    #[test]
    fn complete_profile_configures_locale_timezone_and_identity() -> Result<()> {
        let config = StealthConfig::for_platform(PlatformProfile::Linux);
        assert_eq!(TargetCoordinator::new(&config)?.configured_commands(), 3);
        Ok(())
    }

    #[test]
    fn native_and_disabled_modes_do_not_emit_target_commands() -> Result<()> {
        let config = StealthConfig::none();
        assert_eq!(TargetCoordinator::new(&config)?.configured_commands(), 0);
        Ok(())
    }
}
