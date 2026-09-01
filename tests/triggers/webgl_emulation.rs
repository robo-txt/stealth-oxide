use std::time::Duration;

use anyhow::{Context, Result};
use chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat;
use chromiumoxide::page::ScreenshotParams;
use serde_json::Value;
use tokio::time::timeout;

use super::common;
use common::TestBrowser as StealthBrowser;
use stealth_oxide::profiles::chrome_windows::chrome_windows;
use stealth_oxide::{GpuProfile, StealthConfig};

#[tokio::test]
#[ignore = "requires a local Chromium process with working CDP sockets"]
async fn webgl_surfaces_are_consistent_across_contexts_and_realms() -> Result<()> {
    let browser = timeout(
        Duration::from_secs(20),
        StealthBrowser::launch(chrome_windows()),
    )
    .await
    .context("timed out while launching Chromium")??;
    let system_info = browser.system_info().await?;
    let page = timeout(
        Duration::from_secs(20),
        browser.new_page("data:text/html,<iframe id='probe' srcdoc='<p>probe</p>'></iframe>"),
    )
    .await
    .context("timed out while creating the patched page")??;

    let observed: Value = timeout(
        Duration::from_secs(20),
        page.inner().evaluate(
            r#"
            (async () => {
                const inspect = (window, contextType) => {
                    const canvas = window.document.createElement('canvas');
                    canvas.width = 16;
                    canvas.height = 16;
                    const gl = canvas.getContext(contextType);
                    if (!gl) return null;
                    const debug = gl.getExtension('WEBGL_debug_renderer_info');
                    const precision = gl.getShaderPrecisionFormat(
                        gl.FRAGMENT_SHADER,
                        gl.HIGH_FLOAT
                    );
                    const parameters = Object.fromEntries([
                        'MAX_TEXTURE_SIZE', 'MAX_RENDERBUFFER_SIZE',
                        'MAX_VERTEX_ATTRIBS', 'MAX_TEXTURE_IMAGE_UNITS',
                        'MAX_VERTEX_TEXTURE_IMAGE_UNITS',
                        'MAX_COMBINED_TEXTURE_IMAGE_UNITS'
                    ].map(name => [name, gl.getParameter(gl[name])]));
                    gl.clearColor(0.47, 0.7, 0.78, 1);
                    gl.clear(gl.COLOR_BUFFER_BIT);
                    const pixels = new Uint8Array(4);
                    gl.readPixels(0, 0, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, pixels);
                    return {
                        vendor: debug ? gl.getParameter(debug.UNMASKED_VENDOR_WEBGL) : null,
                        renderer: debug ? gl.getParameter(debug.UNMASKED_RENDERER_WEBGL) : null,
                        maskedVendor: gl.getParameter(gl.VENDOR),
                        maskedRenderer: gl.getParameter(gl.RENDERER),
                        version: gl.getParameter(gl.VERSION),
                        shadingLanguageVersion: gl.getParameter(gl.SHADING_LANGUAGE_VERSION),
                        parameters,
                        extensions: gl.getSupportedExtensions().sort(),
                        precision: {
                            rangeMin: precision.rangeMin,
                            rangeMax: precision.rangeMax,
                            precision: precision.precision
                        },
                        pixel: [...pixels]
                    };
                };

                const workerSource = `
                    const inspect = contextType => {
                        const canvas = new OffscreenCanvas(16, 16);
                        const gl = canvas.getContext(contextType);
                        if (!gl) return null;
                        const debug = gl.getExtension('WEBGL_debug_renderer_info');
                        const precision = gl.getShaderPrecisionFormat(gl.FRAGMENT_SHADER, gl.HIGH_FLOAT);
                        const parameters = Object.fromEntries([
                            'MAX_TEXTURE_SIZE', 'MAX_RENDERBUFFER_SIZE',
                            'MAX_VERTEX_ATTRIBS', 'MAX_TEXTURE_IMAGE_UNITS',
                            'MAX_VERTEX_TEXTURE_IMAGE_UNITS',
                            'MAX_COMBINED_TEXTURE_IMAGE_UNITS'
                        ].map(name => [name, gl.getParameter(gl[name])]));
                        gl.clearColor(0.47, 0.7, 0.78, 1);
                        gl.clear(gl.COLOR_BUFFER_BIT);
                        const pixels = new Uint8Array(4);
                        gl.readPixels(0, 0, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, pixels);
                        return {
                            vendor: debug ? gl.getParameter(debug.UNMASKED_VENDOR_WEBGL) : null,
                            renderer: debug ? gl.getParameter(debug.UNMASKED_RENDERER_WEBGL) : null,
                            maskedVendor: gl.getParameter(gl.VENDOR),
                            maskedRenderer: gl.getParameter(gl.RENDERER),
                            version: gl.getParameter(gl.VERSION),
                            shadingLanguageVersion: gl.getParameter(gl.SHADING_LANGUAGE_VERSION),
                            parameters,
                            extensions: gl.getSupportedExtensions().sort(),
                            precision: {
                                rangeMin: precision.rangeMin,
                                rangeMax: precision.rangeMax,
                                precision: precision.precision
                            },
                            pixel: [...pixels]
                        };
                    };
                    postMessage({ webgl: inspect('webgl'), webgl2: inspect('webgl2') });
                `;
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

                const iframeWindow = document.querySelector('#probe').contentWindow;
                return {
                    top: { webgl: inspect(window, 'webgl'), webgl2: inspect(window, 'webgl2') },
                    iframe: {
                        webgl: inspect(iframeWindow, 'webgl'),
                        webgl2: inspect(iframeWindow, 'webgl2')
                    },
                    worker: workerSnapshot
                };
            })()
            "#,
        ),
    )
    .await
    .context("timed out while reading WebGL surfaces")??
    .into_value()?;

    println!("CDP GPU info: {:#?}", system_info.gpu);
    println!("observed WebGL surfaces: {observed:#}");

    timeout(Duration::from_secs(20), browser.close())
        .await
        .context("timed out while closing Chromium")??;

    for realm in ["top", "iframe", "worker"] {
        assert!(observed[realm]["webgl"].is_object());
        assert!(observed[realm]["webgl2"].is_object());
        assert_eq!(
            observed[realm]["webgl"]["vendor"],
            observed[realm]["webgl2"]["vendor"]
        );
        assert_eq!(
            observed[realm]["webgl"]["renderer"],
            observed[realm]["webgl2"]["renderer"]
        );
    }

    for context in ["webgl", "webgl2"] {
        for property in ["vendor", "renderer", "parameters", "precision", "pixel"] {
            assert_eq!(
                observed["iframe"][context][property],
                observed["top"][context][property]
            );
            assert_eq!(
                observed["worker"][context][property],
                observed["top"][context][property]
            );
        }
    }

    Ok(())
}

#[tokio::test]
#[ignore = "requires network access and a local Chromium process with working CDP sockets"]
async fn gpu_surface_patch_is_visible_on_creepjs() -> Result<()> {
    let profile = chrome_windows();
    let browser = timeout(
        Duration::from_secs(20),
        StealthBrowser::launch(profile.clone()),
    )
    .await
    .context("timed out while launching Chromium")??;
    let gpu_profile = if matches!(
        std::env::var("STEALTH_OXIDE_USE_MESA").as_deref(),
        Ok("1") | Ok("true")
    ) {
        GpuProfile::mesa_amd_renoir()
    } else {
        GpuProfile::new("Test GPU Vendor", "ANGLE (Test GPU)")
    };
    let expected_vendor = gpu_profile.unmasked_vendor.clone();
    let expected_renderer = gpu_profile.unmasked_renderer.clone();
    let stealth = StealthConfig::from_profile(profile).gpu_profile(gpu_profile);
    let (page, target_task) = timeout(
        Duration::from_secs(20),
        browser.new_page_with_stealth_and_targets(
            "https://abrahamjuliot.github.io/creepjs/",
            &stealth,
        ),
    )
    .await
    .context("timed out while loading CreepJS")??;

    tokio::time::sleep(Duration::from_secs(3)).await;
    let observed: Value = timeout(
        Duration::from_secs(20),
        page.inner().evaluate(
            r#"
            (() => {
                const canvas = document.createElement('canvas');
                const gl = canvas.getContext('webgl');
                const debug = gl?.getExtension('WEBGL_debug_renderer_info');
                return {
                    title: document.title,
                    url: location.href,
                    vendor: debug ? gl.getParameter(debug.UNMASKED_VENDOR_WEBGL) : null,
                    renderer: debug ? gl.getParameter(debug.UNMASKED_RENDERER_WEBGL) : null,
                };
            })()
            "#,
        ),
    )
    .await
    .context("timed out while reading CreepJS GPU surfaces")??
    .into_value()?;

    println!("CreepJS GPU-surface probe: {observed:#}");
    assert_eq!(observed["vendor"], expected_vendor);
    assert_eq!(observed["renderer"], expected_renderer);
    assert_eq!(observed["title"], "CreepJS");

    page.inner()
        .save_screenshot(
            ScreenshotParams::builder()
                .format(CaptureScreenshotFormat::Png)
                .full_page(true)
                .build(),
            "/tmp/stealth-oxide-creepjs-gpu-surface.png",
        )
        .await
        .context("failed to save the CreepJS GPU-surface screenshot")?;

    target_task.abort();
    let _ = target_task.await;
    timeout(Duration::from_secs(20), browser.close())
        .await
        .context("timed out while closing Chromium")??;
    Ok(())
}
