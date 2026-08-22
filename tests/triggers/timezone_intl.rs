use std::time::Duration;

use anyhow::{Context, Result};
use serde_json::Value;
use tokio::time::timeout;

use super::common;
use common::TestBrowser as StealthBrowser;
use stealth_oxide::BrowserProfileBuilder;
use stealth_oxide::profiles::chrome_windows::chrome_windows;

#[tokio::test]
#[ignore = "requires a local Chromium process with working CDP sockets"]
async fn timezone_and_intl_are_consistent_across_page_iframe_and_worker() -> Result<()> {
    let profile = BrowserProfileBuilder::new(chrome_windows())
        .locale("fr-FR")
        .languages(["fr-FR", "fr"])
        .timezone("Asia/Tokyo")
        .build()?;
    let expected_locale = profile.locale().locale.clone();
    let expected_timezone = profile.locale().timezone.clone();
    let expected_languages = serde_json::to_value(&profile.navigator().languages)?;
    let browser = timeout(Duration::from_secs(20), StealthBrowser::launch(profile))
        .await
        .context("timed out while launching Chromium")??;
    let page = timeout(
        Duration::from_secs(20),
        browser.new_page(
            "data:text/html,<script>window.initialIntl=[Intl.DateTimeFormat().resolvedOptions().locale,Intl.DateTimeFormat().resolvedOptions().timeZone]</script><iframe id='probe' srcdoc='<p>probe</p>'></iframe>",
        ),
    )
    .await
    .context("timed out while creating the patched page")??;

    let observed: Value = timeout(
        Duration::from_secs(20),
        page.inner().evaluate(
            r#"
            (async () => {
                const snapshot = window => {
                    const intlLocales = {
                        collator: new window.Intl.Collator().resolvedOptions().locale,
                        dateTimeFormat: new window.Intl.DateTimeFormat().resolvedOptions().locale,
                        displayNames: new window.Intl.DisplayNames(undefined, { type: 'language' }).resolvedOptions().locale,
                        listFormat: new window.Intl.ListFormat().resolvedOptions().locale,
                        numberFormat: new window.Intl.NumberFormat().resolvedOptions().locale,
                        pluralRules: new window.Intl.PluralRules().resolvedOptions().locale,
                        relativeTimeFormat: new window.Intl.RelativeTimeFormat().resolvedOptions().locale
                    };
                    return {
                        language: window.navigator.language,
                        languages: [...window.navigator.languages],
                        locale: new window.Intl.DateTimeFormat().resolvedOptions().locale,
                        timezone: new window.Intl.DateTimeFormat().resolvedOptions().timeZone,
                        winterOffset: new window.Date(2026, 0, 15).getTimezoneOffset(),
                        summerOffset: new window.Date(2026, 6, 15).getTimezoneOffset(),
                        historicalOffset: new window.Date(1113, 6, 1).getTimezoneOffset(),
                        numberSample: new window.Intl.NumberFormat().format(1234567.89),
                        dateSample: new window.Intl.DateTimeFormat(undefined, {
                            month: 'long',
                            timeZoneName: 'long'
                        }).format(963644400000),
                        intlLocales
                    };
                };

                const iframeWindow = document.querySelector('#probe').contentWindow;
                const workerSource = `
                    const intlLocales = {
                        collator: new Intl.Collator().resolvedOptions().locale,
                        dateTimeFormat: new Intl.DateTimeFormat().resolvedOptions().locale,
                        displayNames: new Intl.DisplayNames(undefined, { type: 'language' }).resolvedOptions().locale,
                        listFormat: new Intl.ListFormat().resolvedOptions().locale,
                        numberFormat: new Intl.NumberFormat().resolvedOptions().locale,
                        pluralRules: new Intl.PluralRules().resolvedOptions().locale,
                        relativeTimeFormat: new Intl.RelativeTimeFormat().resolvedOptions().locale
                    };
                    postMessage({
                        language: navigator.language,
                        languages: [...navigator.languages],
                        locale: new Intl.DateTimeFormat().resolvedOptions().locale,
                        timezone: new Intl.DateTimeFormat().resolvedOptions().timeZone,
                        winterOffset: new Date(2026, 0, 15).getTimezoneOffset(),
                        summerOffset: new Date(2026, 6, 15).getTimezoneOffset(),
                        historicalOffset: new Date(1113, 6, 1).getTimezoneOffset(),
                        numberSample: new Intl.NumberFormat().format(1234567.89),
                        dateSample: new Intl.DateTimeFormat(undefined, {
                            month: 'long',
                            timeZoneName: 'long'
                        }).format(963644400000),
                        intlLocales
                    });
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

                return {
                    initialIntl: window.initialIntl,
                    top: snapshot(window),
                    iframe: snapshot(iframeWindow),
                    worker: workerSnapshot
                };
            })()
            "#,
        ),
    )
    .await
    .context("timed out while reading timezone and Intl values")??
    .into_value()?;

    println!("observed timezone and Intl surfaces: {observed:#}");

    timeout(Duration::from_secs(20), browser.close())
        .await
        .context("timed out while closing Chromium")??;

    assert_eq!(observed["initialIntl"][0], expected_locale);
    assert_eq!(observed["initialIntl"][1], expected_timezone);

    for realm in ["top", "iframe"] {
        assert_eq!(observed[realm]["language"], expected_locale);
        assert_eq!(observed[realm]["languages"], expected_languages);
    }

    // Emulation locale/timezone overrides propagate to dedicated workers, but
    // Network.setUserAgentOverride's navigator language list does not. Worker
    // navigator consistency is tracked separately from Intl behavior.
    for realm in ["top", "iframe", "worker"] {
        assert_eq!(observed[realm]["locale"], expected_locale);
        assert_eq!(observed[realm]["timezone"], expected_timezone);
        assert_eq!(observed[realm]["winterOffset"], -540);
        assert_eq!(observed[realm]["summerOffset"], -540);

        for constructor in [
            "collator",
            "dateTimeFormat",
            "displayNames",
            "listFormat",
            "numberFormat",
            "pluralRules",
            "relativeTimeFormat",
        ] {
            assert_eq!(observed[realm]["intlLocales"][constructor], expected_locale);
        }
    }

    for property in [
        "historicalOffset",
        "numberSample",
        "dateSample",
        "intlLocales",
    ] {
        assert_eq!(observed["iframe"][property], observed["top"][property]);
        assert_eq!(observed["worker"][property], observed["top"][property]);
    }

    Ok(())
}
