use std::time::Duration;

use anyhow::{Context, Result, bail};
use chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat;
use chromiumoxide::page::ScreenshotParams;
use chromiumoxide::{Browser, BrowserConfig};
use futures::StreamExt;
use serde_json::{Value, json};
use stealth_oxide::{
    BrowserProfile, BrowserProfileBuilder, CompatibilityStatus, GeolocationConfig, Patch,
    PermissionOverride, PermissionSetting, PlatformProfile, ProfileVersion, StealthConfig,
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
    let profile = selected_profile()?;

    let use_mesa = env_enabled("STEALTH_OXIDE_USE_MESA");
    let mut browser_builder = BrowserConfig::builder()
        .hide()
        .arg(("user-agent", profile.navigator().user_agent.as_str()));
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
        .hardware_concurrency(profile.hardware().hardware_concurrency)
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
    let page = config.new_page(&browser, site).await?;

    let coordinator = TargetCoordinator::new(&page_config)?;
    let mut attached_targets = coordinator.enable(&page).await?;
    let coordinator_for_task = coordinator.clone();
    let target_page = page.clone();
    let target_task = tokio::spawn(async move {
        while let Some(event) = attached_targets.next().await {
            if let Err(error) = coordinator_for_task.apply(&target_page, &event).await {
                eprintln!("target configuration failed: {error}");
            }
        }
    });

    sleep(wait).await;

    let observed: Value = page.evaluate(PROBE).await?.into_value()?;
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
        "navigation": navigation,
        "observed": observed,
        "contextMismatches": mismatches,
    }))
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
}
