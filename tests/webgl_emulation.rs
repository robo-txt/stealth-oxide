use std::time::Duration;

use anyhow::{Context, Result};
use serde_json::Value;
use tokio::time::timeout;

use stealth_oxide::browser::StealthBrowser;
use stealth_oxide::profiles::chrome_windows::chrome_windows;

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
