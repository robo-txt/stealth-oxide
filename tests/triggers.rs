//! Environment-dependent browser diagnostics.
//!
//! These probes are ignored by default and intentionally kept separate from
//! deterministic release-gating tests.

#[path = "common/mod.rs"]
mod common;

mod triggers {
    pub(super) use super::common;

    mod device_environment;
    mod font_speech_environment;
    mod headless_emulation;
    mod media_emulation;
    mod screen_emulation;
    mod timezone_intl;
    mod webgl_emulation;
    mod worker_consistency;
}
