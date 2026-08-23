use std::time::Duration;

use anyhow::{Context, Result};
use futures::StreamExt;
use serde_json::Value;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::time::timeout;

use super::common;
use common::TestBrowser as StealthBrowser;
use stealth_oxide::profiles::chrome_windows::chrome_windows;
use stealth_oxide::{StealthConfig, TargetCoordinator};

async fn loopback_page() -> Result<String> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
    let address = listener.local_addr()?;
    tokio::spawn(async move {
        if let Ok((mut stream, _)) = listener.accept().await {
            let body = "<!doctype html><p>worker consistency</p>";
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(response.as_bytes()).await;
        }
    });
    Ok(format!("http://{address}/"))
}

#[tokio::test]
#[ignore = "requires a local Chromium process with working CDP sockets"]
async fn reports_page_and_dedicated_worker_consistency() -> Result<()> {
    let profile = chrome_windows();
    let expected_hardware_concurrency = profile.hardware().hardware_concurrency;
    let stealth = StealthConfig::from_profile(profile.clone())
        .hardware_concurrency(expected_hardware_concurrency);
    let target_coordinator = TargetCoordinator::new(&stealth)?;
    let page_url = loopback_page().await?;
    let browser = timeout(Duration::from_secs(20), StealthBrowser::launch(profile))
        .await
        .context("timed out while launching Chromium")??;
    let page = timeout(
        Duration::from_secs(20),
        browser.new_page_with_stealth(&page_url, &stealth),
    )
    .await
    .context("timed out while creating the patched page")??;

    let mut attached_targets = target_coordinator.enable(page.inner()).await?;

    let worker_page = page.inner().clone();
    let worker_handler = tokio::spawn(async move {
        while let Some(event) = attached_targets.next().await {
            let is_worker = matches!(
                event.target_info.r#type.as_str(),
                "worker" | "shared_worker" | "service_worker"
            );
            target_coordinator.apply(&worker_page, &event).await?;
            if is_worker {
                break;
            }
        }
        Ok::<_, anyhow::Error>(())
    });

    let observed: Value = timeout(
        Duration::from_secs(20),
        page.inner().evaluate(
            r#"
            (async () => {
                const snapshotSource = `
                    const snapshot = async () => {
                        const hints = navigator.userAgentData ?
                            await navigator.userAgentData.getHighEntropyValues([
                                'platform', 'platformVersion', 'architecture',
                                'bitness', 'model', 'uaFullVersion'
                            ]) : null;
                        const canvas = new OffscreenCanvas(16, 16);
                        const gl = canvas.getContext('webgl');
                        const debug = gl?.getExtension('WEBGL_debug_renderer_info');
                        return {
                            userAgent: navigator.userAgent,
                            platform: navigator.platform,
                            language: navigator.language,
                            languages: [...navigator.languages],
                            hardwareConcurrency: navigator.hardwareConcurrency,
                            deviceMemory: navigator.deviceMemory ?? null,
                            locale: Intl.DateTimeFormat().resolvedOptions().locale,
                            timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
                            userAgentData: hints,
                            webglVendor: debug ? gl.getParameter(debug.UNMASKED_VENDOR_WEBGL) : null,
                            webglRenderer: debug ? gl.getParameter(debug.UNMASKED_RENDERER_WEBGL) : null
                        };
                    };
                `;
                const workerSource = snapshotSource + `snapshot().then(postMessage);`;
                const workerUrl = URL.createObjectURL(
                    new Blob([workerSource], { type: 'text/javascript' })
                );
                const worker = new Worker(workerUrl);
                const workerSnapshot = await new Promise((resolve, reject) => {
                    worker.onmessage = event => resolve(event.data);
                    worker.onerror = reject;
                });
                worker.terminate();
                URL.revokeObjectURL(workerUrl);

                const canvas = document.createElement('canvas');
                const gl = canvas.getContext('webgl');
                const debug = gl?.getExtension('WEBGL_debug_renderer_info');
                const hints = navigator.userAgentData ?
                    await navigator.userAgentData.getHighEntropyValues([
                        'platform', 'platformVersion', 'architecture',
                        'bitness', 'model', 'uaFullVersion'
                    ]) : null;
                const windowSnapshot = {
                    userAgent: navigator.userAgent,
                    platform: navigator.platform,
                    language: navigator.language,
                    languages: [...navigator.languages],
                    hardwareConcurrency: navigator.hardwareConcurrency,
                    deviceMemory: navigator.deviceMemory ?? null,
                    locale: Intl.DateTimeFormat().resolvedOptions().locale,
                    timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
                    userAgentData: hints,
                    webglVendor: debug ? gl.getParameter(debug.UNMASKED_VENDOR_WEBGL) : null,
                    webglRenderer: debug ? gl.getParameter(debug.UNMASKED_RENDERER_WEBGL) : null
                };
                const mismatches = Object.keys(windowSnapshot).filter(key =>
                    JSON.stringify(windowSnapshot[key]) !== JSON.stringify(workerSnapshot[key])
                );
                return { window: windowSnapshot, worker: workerSnapshot, mismatches };
            })()
            "#,
        ),
    )
    .await
    .context("timed out while reading worker surfaces")??
    .into_value()?;

    timeout(Duration::from_secs(20), worker_handler)
        .await
        .context("timed out while applying the worker-target override")???;

    println!("observed worker consistency: {observed:#}");

    timeout(Duration::from_secs(20), browser.close())
        .await
        .context("timed out while closing Chromium")??;

    for property in [
        "userAgent",
        "hardwareConcurrency",
        "deviceMemory",
        "locale",
        "timezone",
        "webglVendor",
        "webglRenderer",
    ] {
        assert_eq!(observed["worker"][property], observed["window"][property]);
    }

    assert_eq!(observed["window"]["platform"], "Win32");
    assert_eq!(observed["worker"]["platform"], "Win32");
    assert_eq!(
        observed["window"]["hardwareConcurrency"].as_u64(),
        Some(u64::from(expected_hardware_concurrency))
    );
    assert_eq!(observed["mismatches"], serde_json::json!([]));
    Ok(())
}
