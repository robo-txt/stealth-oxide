//! Environment-dependent browser diagnostics.
//!
//! These probes are ignored by default and intentionally kept separate from
//! deterministic release-gating tests.

#[path = "common/mod.rs"]
mod common;

mod bypass {
    pub(super) use super::common;

    mod creepjs_device_environment;
    mod creepjs_headless;
    mod creepjs_remaining;
    mod creepjs_timezone_intl;
    mod creepjs_webgl;
    mod creepjs_webrtc_media;
    mod creepjs_worker;
    mod device_environment;
    mod font_speech_environment;
    mod headless_emulation;
    mod media_emulation;
    mod screen_emulation;
    mod timezone_intl;
    mod webgl_emulation;
    mod worker_consistency;
}
