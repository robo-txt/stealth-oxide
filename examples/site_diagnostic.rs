use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use chromiumoxide::cdp::browser_protocol::{
    emulation::GetScreenInfosParams, page::CaptureScreenshotFormat,
};
use chromiumoxide::detection::{DetectionOptions, default_executable};
use chromiumoxide::page::ScreenshotParams;
use chromiumoxide::{Browser, BrowserConfig};
use futures::StreamExt;
use serde_json::{Value, json};
use stealth_oxide::{
    BrowserProfile, BrowserProfileBuilder, CapabilityExpectation, CompatibilityStatus,
    GeolocationConfig, NativeCapability, NativeCapabilityExpectations, NativeCapabilityObservation,
    Patch, PermissionOverride, PermissionSetting, PlatformProfile, ProfileVersion, StealthConfig,
    TargetCoordinator, compare_browser_versions,
};
use tokio::time::sleep;
use url::Url;

const DEFAULT_SITES: [&str; 5] = [
    "https://example.com/",
    "https://abrahamjuliot.github.io/creepjs/",
    "https://stackoverflow.com/",
    "https://www.ticketmaster.com/",
    "https://www.reddit.com/",
];

const PROBE: &str = r#"
(async () => {
    const permissionState = async (name) => {
        try {
            return (await navigator.permissions.query({ name })).state;
        } catch (error) {
            return `error:${error.name}`;
        }
    };

    const webgl = () => {
        try {
            const canvas = document.createElement('canvas');
            const context = canvas.getContext('webgl');
            const debug = context?.getExtension('WEBGL_debug_renderer_info');
            return {
                supported: Boolean(context),
                vendor: debug ? context.getParameter(debug.UNMASKED_VENDOR_WEBGL) : null,
                renderer: debug ? context.getParameter(debug.UNMASKED_RENDERER_WEBGL) : null,
                version: context?.getParameter(context.VERSION) ?? null,
            };
        } catch (error) {
            return { supported: false, error: error.name };
        }
    };

    const geolocation = await new Promise(resolve => {
        if (!navigator.geolocation) {
            resolve({ supported: false });
            return;
        }
        navigator.geolocation.getCurrentPosition(
            position => resolve({
                ok: true,
                latitude: position.coords.latitude,
                longitude: position.coords.longitude,
                accuracy: position.coords.accuracy,
            }),
            error => resolve({ ok: false, code: error.code, message: error.message }),
            { maximumAge: 0, timeout: 10000 }
        );
    });

    const userAgentData = navigator.userAgentData
        ? await navigator.userAgentData.getHighEntropyValues([
            'platform', 'platformVersion', 'architecture', 'bitness',
            'model', 'mobile', 'uaFullVersion'
        ])
        : null;

    const snapshot = {
        userAgent: navigator.userAgent,
        appVersion: navigator.appVersion,
        platform: navigator.platform,
        language: navigator.language,
        languages: [...navigator.languages],
        hardwareConcurrency: navigator.hardwareConcurrency,
        deviceMemory: navigator.deviceMemory ?? null,
        webdriver: navigator.webdriver ?? null,
        vendor: navigator.vendor ?? null,
        userAgentData,
        locale: Intl.DateTimeFormat().resolvedOptions().locale,
        timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
        timezoneOffset: new Date().getTimezoneOffset(),
        screen: {
            width: screen.width,
            height: screen.height,
            availWidth: screen.availWidth,
            availHeight: screen.availHeight,
            colorDepth: screen.colorDepth,
            pixelDepth: screen.pixelDepth,
        },
        viewport: { width: innerWidth, height: innerHeight, dpr: devicePixelRatio },
        touch: {
            maxTouchPoints: navigator.maxTouchPoints,
            ontouchstart: 'ontouchstart' in window,
        },
        media: {
            dark: matchMedia('(prefers-color-scheme: dark)').matches,
            reducedMotion: matchMedia('(prefers-reduced-motion: reduce)').matches,
            forcedColors: matchMedia('(forced-colors: active)').matches,
            srgb: matchMedia('(color-gamut: srgb)').matches,
        },
        permissions: {
            geolocation: await permissionState('geolocation'),
            notifications: await permissionState('notifications'),
        },
        geolocationPolicy: document.permissionsPolicy
            ? document.permissionsPolicy.allowsFeature('geolocation')
            : null,
        geolocationApi: {
            type: Object.prototype.toString.call(navigator.geolocation),
            getCurrentPosition: String(navigator.geolocation?.getCurrentPosition ?? '')
                .slice(0, 200),
        },
        geolocation,
        webgl: webgl(),
        nativeCapabilities: {
            webShare: 'share' in navigator && 'canShare' in navigator,
            contactsManager: 'ContactsManager' in window,
            contentIndex: 'ContentIndex' in window,
            networkInformationDownlinkMax: 'downlinkMax' in (window.NetworkInformation?.prototype || {}),
        },
    };

    let iframe = null;
    for (const candidate of document.querySelectorAll('iframe')) {
        try {
            if (candidate.contentWindow?.location.origin === location.origin) {
                iframe = {
                    platform: candidate.contentWindow.navigator.platform,
                    language: candidate.contentWindow.navigator.language,
                    languages: [...candidate.contentWindow.navigator.languages],
                    hardwareConcurrency: candidate.contentWindow.navigator.hardwareConcurrency,
                    timezone: candidate.contentWindow.Intl.DateTimeFormat().resolvedOptions().timeZone,
                };
                break;
            }
        } catch (_) {}
    }

    const worker = await new Promise(resolve => {
        const source = `
            const snapshot = {
                userAgent: navigator.userAgent,
                platform: navigator.platform,
                language: navigator.language,
                languages: [...navigator.languages],
                hardwareConcurrency: navigator.hardwareConcurrency,
                deviceMemory: navigator.deviceMemory ?? null,
                locale: Intl.DateTimeFormat().resolvedOptions().locale,
                timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
                timezoneOffset: new Date().getTimezoneOffset(),
            };
            postMessage(snapshot);
        `;
        const workerUrl = URL.createObjectURL(new Blob([source], { type: 'text/javascript' }));
        const instance = new Worker(workerUrl);
        const timer = setTimeout(() => {
            instance.terminate();
            URL.revokeObjectURL(workerUrl);
            resolve({ error: 'timeout' });
        }, 2000);
        instance.onmessage = event => {
            clearTimeout(timer);
            instance.terminate();
            URL.revokeObjectURL(workerUrl);
            resolve(event.data);
        };
        instance.onerror = event => {
            clearTimeout(timer);
            instance.terminate();
            URL.revokeObjectURL(workerUrl);
            resolve({ error: event.message || 'worker error' });
        };
    });

    const bodyText = document.body?.innerText ?? '';
    const isCreepJs = /creepjs/i.test(`${location.hostname} ${document.title}`);
    const creepFingerprint = globalThis.Fingerprint;
    const creepHeadless = creepFingerprint?.headless;
    const lines = bodyText
        .split(/\n+/)
        .map(line => line.trim())
        .filter(line => /%|score|trust|lie|bot|creep|headless|stealth/i.test(line))
        .slice(0, 40);
    const percentages = [...bodyText.matchAll(/\b(?:100|[1-9]?\d)(?:\.\d+)?%/g)]
        .map(match => match[0]);

    return {
        window: snapshot,
        iframe,
        worker,
        creepjs: {
            detected: isCreepJs,
            fingerprintHeadless: creepHeadless
                ? {
                    likeHeadless: creepHeadless.likeHeadless ?? null,
                    headless: creepHeadless.headless ?? null,
                    stealth: creepHeadless.stealth ?? null,
                }
                : null,
            fingerprintLies: creepFingerprint?.lies ?? null,
            fingerprintStatus: creepFingerprint
                ? {
                    navigatorLied: creepFingerprint.navigator?.lied ?? null,
                    workerLied: creepFingerprint.workerScope?.lied ?? null,
                    screenLied: creepFingerprint.screen?.lied ?? null,
                    webglLied: creepFingerprint.canvasWebgl?.lied ?? null,
                }
                : null,
            workerScope: creepFingerprint?.workerScope
                ? {
                    userAgent: creepFingerprint.workerScope.userAgent ?? null,
                    platform: creepFingerprint.workerScope.platform ?? null,
                    hardwareConcurrency: creepFingerprint.workerScope.hardwareConcurrency ?? null,
                    deviceMemory: creepFingerprint.workerScope.deviceMemory ?? null,
                    workerType: creepFingerprint.workerScope.type ?? null,
                }
                : null,
            pageTextLength: bodyText.length,
            percentageCandidates: percentages,
            scoreCandidates: isCreepJs
                ? lines.filter(line => /%|score|trust|headless|stealth/i.test(line))
                : [],
            signalLines: isCreepJs ? lines : [],
        },
    };
})()
"#;

#[tokio::main]
async fn main() -> Result<()> {
    let sites = std::env::args().skip(1).collect::<Vec<_>>();
    let sites = if sites.is_empty() {
        DEFAULT_SITES
            .iter()
            .map(|site| (*site).to_string())
            .collect()
    } else {
        sites
    };
    let wait_seconds = std::env::var("STEALTH_OXIDE_DIAGNOSTIC_WAIT")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(5);

    for site in sites {
        let url = Url::parse(&site).with_context(|| format!("invalid site URL: {site}"))?;
        if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
            bail!("site URL must be HTTP(S) with a host: {site}");
        }
        let result = run_site(&site, Duration::from_secs(wait_seconds)).await?;
        println!("{}", serde_json::to_string_pretty(&result)?);
    }
    Ok(())
}

async fn run_site(site: &str, wait: Duration) -> Result<Value> {
    let parsed = Url::parse(site)?;
    let origin = parsed.origin().ascii_serialization();
    let selected_profile = selected_profile()?;
    let (chrome_executable, profile) = runtime_profile(selected_profile)?;

    let use_mesa = env_enabled("STEALTH_OXIDE_USE_MESA");
    let mut browser_builder = BrowserConfig::builder()
        .chrome_executable(&chrome_executable)
        .arg(("user-agent", profile.navigator().user_agent.as_str()));
    browser_builder = if env_enabled("STEALTH_OXIDE_DIAGNOSTIC_NEW_HEADLESS") {
        browser_builder.new_headless_mode()
    } else {
        browser_builder.hide()
    };
    if use_mesa {
        browser_builder = browser_builder
            .arg(("use-gl", "angle"))
            .arg(("use-angle", "gl"))
            .arg("ignore-gpu-blocklist")
            .arg("enable-gpu-rasterization");
    }
    let browser_config = browser_builder.build().map_err(anyhow::Error::msg)?;
    let (mut browser, mut handler) = Browser::launch(browser_config).await?;
    tokio::spawn(async move {
        while let Some(event) = handler.next().await {
            if let Err(error) = event {
                eprintln!("chromiumoxide handler error: {error:?}");
            }
        }
    });

    let runtime_product = browser.version().await?.product;
    let profile = ProfileVersion::from_product(&runtime_product)
        .map(|version| {
            BrowserProfileBuilder::new(profile.clone())
                .chrome_version(version)
                .build()
        })
        .transpose()?
        .unwrap_or(profile);
    let profile_name = profile.name().to_string();
    let config = StealthConfig::from_profile(profile.clone())
        .permission(PermissionOverride::for_origin(
            "geolocation",
            PermissionSetting::Granted,
            &origin,
        ))
        .geolocation(GeolocationConfig::position(40.7128, -74.006, 25.0));
    let page_config = config.clone().use_native(Patch::Permissions);
    let profile_compatibility = compatibility_json(compare_browser_versions(
        profile.version(),
        &runtime_product,
    ));
    config.apply_browser(&browser).await?;
    let page = browser.new_page("about:blank").await?;
    page_config.apply(&page).await?;
    let coordinator = TargetCoordinator::new(&page_config)?;
    let mut attached_targets = coordinator.enable(&page).await?;
    let coordinator_for_task = coordinator.clone();
    let target_page = page.clone();
    let target_task = tokio::spawn(async move {
        while let Some(event) = attached_targets.next().await {
            if let Err(error) = coordinator_for_task.apply(&target_page, &event).await {
                eprintln!(
                    "target configuration failed for {} ({}): {error}",
                    event.target_info.r#type, event.target_info.url
                );
            }
        }
    });
    page.goto(site).await?;
    page_config.apply(&page).await?;
    let native_screen_infos = serde_json::to_value(
        &page
            .execute(GetScreenInfosParams::default())
            .await?
            .screen_infos,
    )?;

    sleep(wait).await;

    let observed: Value = page.evaluate(PROBE).await?.into_value()?;
    let capability_expectations = native_capability_expectations(&profile);
    let capability_observation = native_capability_observation(&observed);
    let capability_mismatches = capability_observation
        .map(|observation| capability_expectations.mismatches(observation))
        .unwrap_or_default();
    if let Ok(path) = std::env::var("STEALTH_OXIDE_DIAGNOSTIC_SCREENSHOT") {
        page.save_screenshot(
            ScreenshotParams::builder()
                .format(CaptureScreenshotFormat::Png)
                .full_page(true)
                .build(),
            &path,
        )
        .await
        .with_context(|| format!("failed to save screenshot to {path}"))?;
        eprintln!("saved screenshot: {path}");
    }
    let final_url = page.url().await?.map(|url| sanitize_url(&url));
    let navigation = json!({
        "status": null,
        "finalUrl": final_url,
        "redirectStatuses": [],
        "title": page.get_title().await?.unwrap_or_default(),
    });

    let mismatches = compare_contexts(&observed);
    coordinator.disable(&page).await?;
    browser.close().await?;
    target_task.abort();

    Ok(json!({
        "site": sanitize_url(site),
        "origin": origin,
        "profile": profile_name,
        "runtime": {
            "product": runtime_product,
            "profileCompatibility": profile_compatibility,
        },
        "gpuMode": if use_mesa { "mesa" } else { "default" },
        "nativeCapabilities": {
            "expectations": capability_expectations_json(capability_expectations),
            "observed": capability_observation.map(capability_observation_json),
            "mismatches": capability_mismatches
                .iter()
                .map(|mismatch| {
                    json!({
                        "capability": capability_name(mismatch.capability),
                        "expectation": expectation_name(mismatch.expectation),
                        "observed": mismatch.observed,
                    })
                })
                .collect::<Vec<_>>(),
        },
        "nativeScreenInfos": native_screen_infos,
        "navigation": navigation,
        "observed": observed,
        "contextMismatches": mismatches,
    }))
}

fn native_capability_expectations(profile: &BrowserProfile) -> NativeCapabilityExpectations {
    match profile.name() {
        "chrome-linux" => PlatformProfile::Linux.native_capability_expectations(),
        "chrome-macos" => PlatformProfile::MacOS.native_capability_expectations(),
        "chrome-windows" => PlatformProfile::Windows.native_capability_expectations(),
        _ => NativeCapabilityExpectations {
            web_share: CapabilityExpectation::RuntimeDependent,
            contacts_manager: CapabilityExpectation::RuntimeDependent,
            content_index: CapabilityExpectation::RuntimeDependent,
            network_information_downlink_max: CapabilityExpectation::RuntimeDependent,
        },
    }
}

fn runtime_profile(profile: BrowserProfile) -> Result<(PathBuf, BrowserProfile)> {
    let executable = default_executable(DetectionOptions::default())
        .map_err(|message| anyhow::anyhow!(message))?;
    let output = Command::new(&executable)
        .arg("--version")
        .output()
        .with_context(|| {
            format!(
                "failed to inspect Chromium executable {}",
                executable.display()
            )
        })?;
    if !output.status.success() {
        bail!("Chromium --version failed for {}", executable.display());
    }
    let version_text = String::from_utf8_lossy(&output.stdout);
    let version = version_text
        .split_whitespace()
        .find(|token| {
            token
                .chars()
                .next()
                .is_some_and(|character| character.is_ascii_digit())
        })
        .ok_or_else(|| anyhow::anyhow!("Chromium version was not present in --version output"))?;
    let chrome_major = version
        .split('.')
        .next()
        .and_then(|major| major.parse::<u32>().ok())
        .ok_or_else(|| anyhow::anyhow!("invalid Chromium version {version}"))?;
    let runtime_version = ProfileVersion::new(chrome_major, version);
    let profile = BrowserProfileBuilder::new(profile)
        .chrome_version(runtime_version)
        .build()?;
    Ok((executable, profile))
}

fn native_capability_observation(observed: &Value) -> Option<NativeCapabilityObservation> {
    let capabilities = observed.get("window")?.get("nativeCapabilities")?;
    Some(NativeCapabilityObservation::new(
        capabilities.get("webShare")?.as_bool()?,
        capabilities.get("contactsManager")?.as_bool()?,
        capabilities.get("contentIndex")?.as_bool()?,
        capabilities
            .get("networkInformationDownlinkMax")?
            .as_bool()?,
    ))
}

fn capability_expectations_json(expectations: NativeCapabilityExpectations) -> Value {
    json!({
        "webShare": expectation_name(expectations.web_share),
        "contactsManager": expectation_name(expectations.contacts_manager),
        "contentIndex": expectation_name(expectations.content_index),
        "networkInformationDownlinkMax": expectation_name(expectations.network_information_downlink_max),
    })
}

fn capability_observation_json(observation: NativeCapabilityObservation) -> Value {
    json!({
        "webShare": observation.web_share,
        "contactsManager": observation.contacts_manager,
        "contentIndex": observation.content_index,
        "networkInformationDownlinkMax": observation.network_information_downlink_max,
    })
}

fn capability_name(capability: NativeCapability) -> &'static str {
    match capability {
        NativeCapability::WebShare => "webShare",
        NativeCapability::ContactsManager => "contactsManager",
        NativeCapability::ContentIndex => "contentIndex",
        NativeCapability::NetworkInformationDownlinkMax => "networkInformationDownlinkMax",
        _ => "unknown",
    }
}

fn expectation_name(expectation: CapabilityExpectation) -> &'static str {
    match expectation {
        CapabilityExpectation::Expected => "expected",
        CapabilityExpectation::NotExpected => "not-expected",
        CapabilityExpectation::RuntimeDependent => "runtime-dependent",
        _ => "unknown",
    }
}

fn selected_profile() -> Result<BrowserProfile> {
    profile_from_name(
        std::env::var("STEALTH_OXIDE_DIAGNOSTIC_PROFILE")
            .ok()
            .as_deref(),
    )
}

fn profile_from_name(value: Option<&str>) -> Result<BrowserProfile> {
    match value.unwrap_or("windows") {
        "linux" => Ok(PlatformProfile::Linux.profile()),
        "windows" => Ok(PlatformProfile::Windows.profile()),
        "macos" | "mac" => Ok(PlatformProfile::MacOS.profile()),
        value => {
            bail!("STEALTH_OXIDE_DIAGNOSTIC_PROFILE must be linux, windows, or macos; got {value}")
        }
    }
}

fn compatibility_json(status: CompatibilityStatus) -> Value {
    match status {
        CompatibilityStatus::Compatible { chrome_major } => {
            json!({ "status": "compatible", "chromeMajor": chrome_major })
        }
        CompatibilityStatus::MajorMismatch {
            profile_major,
            runtime_major,
        } => json!({
            "status": "major-mismatch",
            "profileMajor": profile_major,
            "runtimeMajor": runtime_major,
        }),
        CompatibilityStatus::UnknownProfileVersion => json!({
            "status": "unknown-profile-version",
        }),
        CompatibilityStatus::UnknownRuntimeVersion => json!({
            "status": "unknown-runtime-version",
        }),
        _ => json!({
            "status": "unknown",
        }),
    }
}

fn sanitize_url(value: &str) -> String {
    let Ok(mut url) = Url::parse(value) else {
        return "[invalid-url]".to_string();
    };
    url.set_query(None);
    url.set_fragment(None);
    url.to_string()
}

fn compare_contexts(observed: &Value) -> Vec<String> {
    let Some(window) = observed.get("window") else {
        return vec!["window snapshot missing".to_string()];
    };
    let Some(worker) = observed.get("worker") else {
        return vec!["worker snapshot missing".to_string()];
    };
    [
        "userAgent",
        "platform",
        "language",
        "languages",
        "hardwareConcurrency",
        "locale",
        "timezone",
        "timezoneOffset",
    ]
    .into_iter()
    .filter(|key| window.get(*key) != worker.get(*key))
    .map(|key| format!("window.worker.{key}"))
    .collect()
}

fn env_enabled(name: &str) -> bool {
    matches!(std::env::var(name).as_deref(), Ok("1") | Ok("true"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_each_builtin_profile() -> Result<()> {
        for (name, expected) in [
            ("linux", "chrome-linux"),
            ("windows", "chrome-windows"),
            ("macos", "chrome-macos"),
            ("mac", "chrome-macos"),
        ] {
            assert_eq!(profile_from_name(Some(name))?.name(), expected);
        }
        Ok(())
    }

    #[test]
    fn rejects_unknown_profile_names() {
        assert!(profile_from_name(Some("android")).is_err());
    }

    #[test]
    fn adapts_profile_chrome_version_without_changing_platform() -> Result<()> {
        let profile = BrowserProfileBuilder::new(PlatformProfile::Windows.profile())
            .chrome_version(ProfileVersion::from_product("Chrome/150.0.7871.128").unwrap())
            .build()?;

        assert!(
            profile
                .navigator()
                .user_agent
                .contains("Chrome/150.0.7871.128")
        );
        assert_eq!(profile.navigator().platform, "Win32");
        assert_eq!(profile.version().unwrap().chrome_major, 150);
        Ok(())
    }

    #[test]
    fn reads_native_capability_observations_from_the_probe_shape() {
        let observed = json!({
            "window": {
                "nativeCapabilities": {
                    "webShare": true,
                    "contactsManager": false,
                    "contentIndex": false,
                    "networkInformationDownlinkMax": false,
                }
            }
        });

        assert_eq!(
            native_capability_observation(&observed),
            Some(NativeCapabilityObservation::new(true, false, false, false))
        );
    }

    #[test]
    fn unknown_profile_names_use_runtime_dependent_capability_expectations() {
        let profile = BrowserProfileBuilder::new(PlatformProfile::Linux.profile())
            .name("custom")
            .build()
            .unwrap();

        let expectations = native_capability_expectations(&profile);
        assert_eq!(
            expectations.web_share,
            CapabilityExpectation::RuntimeDependent
        );
        assert_eq!(
            expectations.contacts_manager,
            CapabilityExpectation::RuntimeDependent
        );
    }
}
