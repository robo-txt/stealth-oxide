use std::time::Duration;

use anyhow::{Context, Result};
use serde_json::Value;
use tokio::time::timeout;

use stealth_oxide::browser::StealthBrowser;
use stealth_oxide::profiles::chrome_windows::chrome_windows;

#[tokio::test]
#[ignore = "requires a local Chromium process with working CDP sockets"]
async fn reports_page_and_dedicated_worker_consistency() -> Result<()> {
    let browser = timeout(
        Duration::from_secs(20),
        StealthBrowser::launch(chrome_windows()),
    )
    .await
    .context("timed out while launching Chromium")??;
    let page = timeout(
        Duration::from_secs(20),
        browser.new_page("data:text/html,<p>worker consistency</p>"),
    )
    .await
    .context("timed out while creating the patched page")??;

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

    assert!(observed["mismatches"].is_array());
    Ok(())
}
