use std::time::Duration;

use anyhow::{Context, Result};
use serde_json::Value;
use tokio::time::timeout;

mod common;
use common::TestBrowser as StealthBrowser;
use stealth_oxide::profiles::chrome_windows::chrome_windows;

#[tokio::test]
#[ignore = "requires a local Chromium process with working CDP sockets"]
async fn direct_headless_signals_are_disabled_across_page_iframe_and_worker() -> Result<()> {
    let profile = chrome_windows();
    let expected_user_agent = profile.navigator().user_agent.clone();
    let browser = timeout(Duration::from_secs(20), StealthBrowser::launch(profile))
        .await
        .context("timed out while launching Chromium")??;
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
                const iframeWindow = document.querySelector('#probe').contentWindow;
                const source = `postMessage({
                    userAgent: navigator.userAgent,
                    platform: navigator.platform
                })`;
                const workerUrl = URL.createObjectURL(
                    new Blob([source], { type: 'text/javascript' })
                );
                const worker = new Worker(workerUrl);
                const workerNavigator = await new Promise((resolve, reject) => {
                    worker.onmessage = event => resolve(event.data);
                    worker.onerror = reject;
                });
                worker.terminate();
                URL.revokeObjectURL(workerUrl);

                return {
                    webdriver: navigator.webdriver,
                    userAgent: navigator.userAgent,
                    appVersion: navigator.appVersion,
                    iframeWebdriver: iframeWindow.navigator.webdriver,
                    iframeUserAgent: iframeWindow.navigator.userAgent,
                    workerNavigator
                };
            })()
            "#,
        ),
    )
    .await
    .context("timed out while reading direct headless signals")??
    .into_value()?;

    timeout(Duration::from_secs(20), browser.close())
        .await
        .context("timed out while closing Chromium")??;

    assert_eq!(observed["webdriver"], false);
    assert_eq!(observed["iframeWebdriver"], false);
    assert_eq!(observed["userAgent"], expected_user_agent);
    assert_eq!(observed["iframeUserAgent"], expected_user_agent);
    assert_eq!(
        observed["workerNavigator"]["userAgent"],
        expected_user_agent
    );
    assert!(
        !observed["appVersion"]
            .as_str()
            .unwrap_or_default()
            .contains("HeadlessChrome")
    );

    Ok(())
}
