//! Run a selected desktop profile in a CPU-rendered Docker Chromium job.
//!
//! This example is intended for the repository's Mesa/LLVMpipe Docker image.
//! It selects an explicit operating-system/GPU profile pair in Rust, applies
//! launch-time ANGLE settings before Chromium starts, and applies the CDP
//! profile before navigating to CreepJS.
//!
//! Examples from the repository root:
//!
//! ```text
//! cargo run --example docker_gpu_profiles
//! cargo run --example docker_gpu_profiles -- windows
//! ```
//!
//! To run the compiled example in the repository's Docker image, first build
//! `examples/Dockerfile.docker_gpu_profiles`, then use:
//!
//! ```text
//! docker run --rm --network host --shm-size=2g -v /tmp:/tmp \
//!     stealth-oxide/docker-gpu-profiles:local
//! ```
//!
//! The generated screenshots default to
//! `/tmp/stealth-oxide-creepjs-docker/creepjs-<platform>.png`. Compare them
//! with the prior Docker/Mesa reference at
//! `dev/screen-gpu-lab/artifacts/docker-xvfb/creepjs-docker-angle-profile.png`.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat;
use chromiumoxide::page::ScreenshotParams;
use chromiumoxide::{Browser, BrowserConfig};
use futures::StreamExt;
use serde_json::{Value, json};
use stealth_oxide::{
    GpuPreset, GpuRuntimeProfile, PlatformProfile, StealthConfig, TargetCoordinator,
};
use tokio::time::sleep;

const CREEPJS_URL: &str = "https://abrahamjuliot.github.io/creepjs/";
const DEFAULT_SCREENSHOT_DIR: &str = "/tmp/stealth-oxide-creepjs-docker";

#[derive(Debug, Clone, Copy)]
struct DockerProfile {
    platform: PlatformProfile,
    gpu_name: &'static str,
    gpu: GpuRuntimeProfile,
}

impl DockerProfile {
    fn for_platform(platform: PlatformProfile) -> Self {
        match platform {
            PlatformProfile::Linux => Self {
                platform,
                gpu_name: "linux-amd-radeon-hd-3200",
                gpu: GpuRuntimeProfile {
                    angle_vendor: "AMD",
                    angle_renderer: "AMD Radeon HD 3200 Graphics",
                },
            },
            PlatformProfile::Windows => Self::from_preset(GpuPreset::WindowsIntelIrisXe),
            PlatformProfile::MacOS => Self::from_preset(GpuPreset::MacosAppleM1),
        }
    }

    fn from_preset(preset: GpuPreset) -> Self {
        let platform = if preset.name().starts_with("windows-") {
            PlatformProfile::Windows
        } else {
            PlatformProfile::MacOS
        };
        Self {
            platform,
            gpu_name: preset.name(),
            gpu: preset.runtime(),
        }
    }

    fn slug(self) -> &'static str {
        match self.platform {
            PlatformProfile::Linux => "linux",
            PlatformProfile::Windows => "windows",
            PlatformProfile::MacOS => "macos",
        }
    }
}

const PROBE: &str = r#"
(() => {
    const canvas = document.createElement('canvas');
    const gl = canvas.getContext('webgl');
    const debug = gl?.getExtension('WEBGL_debug_renderer_info');
    return {
        title: document.title,
        url: location.href,
        webglVendor: debug ? gl.getParameter(debug.UNMASKED_VENDOR_WEBGL) : null,
        webglRenderer: debug ? gl.getParameter(debug.UNMASKED_RENDERER_WEBGL) : null,
        bodyText: document.body?.innerText?.slice(0, 1000) ?? '',
    };
})()
"#;

#[tokio::main]
async fn main() -> Result<()> {
    let (profiles, url) = selection_from_args()?;
    let output_dir = std::env::var_os("CREEPJS_SCREENSHOT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SCREENSHOT_DIR));
    std::fs::create_dir_all(&output_dir).with_context(|| {
        format!(
            "failed to create screenshot directory {}",
            output_dir.display()
        )
    })?;

    let mut observations = Vec::with_capacity(profiles.len());
    for profile in profiles {
        let screenshot = output_dir.join(format!("creepjs-{}.png", profile.slug()));
        observations.push(run_profile(profile, &url, &screenshot).await?);
    }
    println!("{}", serde_json::to_string_pretty(&observations)?);
    Ok(())
}

async fn run_profile(selection: DockerProfile, url: &str, screenshot: &Path) -> Result<Value> {
    let profile = selection.platform.profile();
    // The GPU is selected together with the browser profile in Rust. This
    // applies ANGLE_GL_VENDOR/ANGLE_GL_RENDERER before Chromium launches.
    let stealth = StealthConfig::from_profile(profile.clone()).docker_gpu_runtime(selection.gpu);
    let browser_builder = BrowserConfig::builder()
        .hide()
        .new_headless_mode()
        .no_sandbox()
        .arg("disable-dev-shm-usage")
        .arg(("ozone-platform", "x11"))
        .arg(("user-agent", profile.navigator().user_agent.as_str()))
        .arg(("use-gl", "angle"))
        .arg(("use-angle", "gl"))
        .arg("enable-gpu")
        .arg("ignore-gpu-blocklist")
        .arg("enable-gpu-rasterization")
        .env("DISPLAY", ":99");
    let browser_config = stealth
        .apply_docker_gpu_environment(browser_builder)
        .build()
        .map_err(anyhow::Error::msg)
        .context("failed to build Chromium Docker configuration")?;

    println!("platform: {:?}", selection.platform);
    println!("profile: {}", profile.name());
    println!("gpu profile: {}", selection.gpu_name);
    println!("ANGLE vendor: {}", selection.gpu.angle_vendor);
    println!("ANGLE renderer: {}", selection.gpu.angle_renderer);
    println!("CreepJS URL: {url}");
    println!("output screenshot: {}", screenshot.display());

    let (mut browser, mut handler) = Browser::launch(browser_config).await?;
    tokio::spawn(async move {
        while let Some(event) = handler.next().await {
            if let Err(error) = event {
                eprintln!("chromiumoxide handler error: {error:?}");
            }
        }
    });

    stealth.apply_browser(&browser).await?;
    let page = browser.new_page("about:blank").await?;
    stealth.apply(&page).await?;

    let coordinator = TargetCoordinator::new(&stealth)?;
    let mut attached_targets = coordinator.enable(&page).await?;
    let coordinator_task = {
        let coordinator = coordinator.clone();
        let target_page = page.clone();
        tokio::spawn(async move {
            while let Some(event) = attached_targets.next().await {
                if let Err(error) = coordinator.apply(&target_page, &event).await {
                    eprintln!(
                        "target configuration failed for {} ({}): {error}",
                        event.target_info.r#type, event.target_info.url
                    );
                }
            }
        })
    };

    page.goto(url).await?;
    stealth.apply(&page).await?;
    sleep(Duration::from_secs(10)).await;

    let observation: Value = page
        .evaluate(PROBE)
        .await?
        .into_value()
        .context("failed to read the CreepJS observation")?;
    println!(
        "CreepJS observation: {}",
        serde_json::to_string_pretty(&observation)?
    );

    page.save_screenshot(
        ScreenshotParams::builder()
            .format(CaptureScreenshotFormat::Png)
            .full_page(true)
            .build(),
        screenshot,
    )
    .await
    .with_context(|| format!("failed to save screenshot to {}", screenshot.display()))?;

    coordinator.disable(&page).await?;
    coordinator_task.abort();
    browser.close().await?;
    Ok(json!({
        "platform": format!("{:?}", selection.platform),
        "profile": profile.name(),
        "gpuProfile": selection.gpu_name,
        "angleVendor": selection.gpu.angle_vendor,
        "angleRenderer": selection.gpu.angle_renderer,
        "url": url,
        "screenshot": screenshot,
        "observation": observation,
    }))
}

fn selection_from_args() -> Result<(Vec<DockerProfile>, String)> {
    let mut args = std::env::args().skip(1);
    let platform_name = args.next();
    if matches!(platform_name.as_deref(), Some("--help" | "-h")) {
        print_usage();
        std::process::exit(0);
    }

    let profiles = match platform_name {
        Some(platform_name) => vec![DockerProfile::for_platform(parse_platform(&platform_name)?)],
        None => vec![
            DockerProfile::for_platform(PlatformProfile::Linux),
            DockerProfile::for_platform(PlatformProfile::Windows),
            DockerProfile::for_platform(PlatformProfile::MacOS),
        ],
    };
    let url = args.next().unwrap_or_else(|| CREEPJS_URL.to_string());
    if args.next().is_some() {
        bail!("usage accepts at most one platform and one URL")
    }
    Ok((profiles, url))
}

fn parse_platform(value: &str) -> Result<PlatformProfile> {
    match value {
        "linux" => Ok(PlatformProfile::Linux),
        "windows" => Ok(PlatformProfile::Windows),
        "macos" | "mac" => Ok(PlatformProfile::MacOS),
        value => bail!("unknown platform {value}; expected linux, windows, or macos"),
    }
}

fn print_usage() {
    eprintln!(
        "usage: cargo run --example docker_gpu_profiles [linux|windows|macos] [url]\n\n\
with no platform, runs all three explicit Rust profile/GPU pairs:\n\
  linux   -> AMD Radeon HD 3200 ANGLE identity\n\
  windows -> Intel Iris Xe\n\
  macos   -> Apple M1\n\n\
Set CREEPJS_SCREENSHOT_DIR to change the output directory."
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_each_platform() -> Result<()> {
        assert_eq!(parse_platform("linux")?, PlatformProfile::Linux);
        assert_eq!(parse_platform("windows")?, PlatformProfile::Windows);
        assert_eq!(parse_platform("mac")?, PlatformProfile::MacOS);
        Ok(())
    }

    #[test]
    fn maps_each_platform_to_a_gpu_in_code() {
        let linux = DockerProfile::for_platform(PlatformProfile::Linux);
        let windows = DockerProfile::for_platform(PlatformProfile::Windows);
        let macos = DockerProfile::for_platform(PlatformProfile::MacOS);

        assert_eq!(linux.gpu_name, "linux-amd-radeon-hd-3200");
        assert_eq!(windows.gpu_name, "windows-intel-iris-xe");
        assert_eq!(macos.gpu_name, "macos-apple-m1");
    }
}
