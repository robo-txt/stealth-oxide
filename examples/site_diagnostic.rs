use std::time::Duration;

use anyhow::{Context, Result, bail};
use chromiumoxide::cdp::browser_protocol::network::{EventResponseReceived, ResourceType};
use chromiumoxide::cdp::browser_protocol::page::GetFrameTreeParams;
use chromiumoxide::{Browser, BrowserConfig};
use futures::StreamExt;
use serde_json::{Value, json};
use stealth_oxide::profiles::chrome_windows::chrome_windows;
use stealth_oxide::{
    GeolocationConfig, Patch, PermissionOverride, PermissionSetting, StealthConfig,
    TargetCoordinator,
};
use tokio::time::{sleep, timeout};
use url::Url;

const DEFAULT_SITES: [&str; 3] = [
    "https://example.com/",
    "https://abrahamjuliot.github.io/creepjs/",
    "https://stackoverflow.com/",
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
            { timeout: 2000 }
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
    let profile = chrome_windows();
    let config = StealthConfig::from_profile(profile.clone())
        .hardware_concurrency(profile.hardware().hardware_concurrency)
        .permission(PermissionOverride::for_origin(
            "geolocation",
            PermissionSetting::Granted,
            &origin,
        ))
        .geolocation(GeolocationConfig::position(40.7128, -74.006, 25.0));
    let page_config = config.clone().use_native(Patch::Permissions);

    let browser_config = BrowserConfig::builder()
        .hide()
        .arg(("user-agent", profile.navigator().user_agent.as_str()))
        .build()
        .map_err(anyhow::Error::msg)?;
    let (mut browser, mut handler) = Browser::launch(browser_config).await?;
    tokio::spawn(async move {
        while let Some(event) = handler.next().await {
            if let Err(error) = event {
                eprintln!("chromiumoxide handler error: {error:?}");
            }
        }
    });

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
                eprintln!("target configuration failed: {error}");
            }
        }
    });

    let mut responses = page
        .event_listener::<EventResponseReceived>()
        .await
        .map_err(|source| anyhow::anyhow!(source))?;
    let main_frame_id = page
        .execute(GetFrameTreeParams {})
        .await?
        .result
        .frame_tree
        .frame
        .id;
    page.goto(site).await?;
    sleep(wait).await;

    let mut document_responses = Vec::new();
    loop {
        match timeout(Duration::from_millis(250), responses.next()).await {
            Ok(Some(event))
                if event.r#type == ResourceType::Document
                    && event.frame_id.as_ref() == Some(&main_frame_id) =>
            {
                document_responses.push(event);
            }
            Ok(Some(_)) => {}
            Ok(None) | Err(_) => break,
        }
    }

    let observed: Value = page.evaluate(PROBE).await?.into_value()?;
    let navigation = json!({
        "status": document_responses.last().map(|event| event.response.status),
        "finalUrl": document_responses.last().map(|event| event.response.url.clone()),
        "redirectStatuses": document_responses
            .iter()
            .rev()
            .skip(1)
            .map(|event| event.response.status)
            .collect::<Vec<_>>(),
        "title": page.get_title().await?.unwrap_or_default(),
    });

    let mismatches = compare_contexts(&observed);
    coordinator.disable(&page).await?;
    browser.close().await?;
    target_task.abort();

    Ok(json!({
        "site": site,
        "origin": origin,
        "profile": "chrome-windows",
        "navigation": navigation,
        "observed": observed,
        "contextMismatches": mismatches,
    }))
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
