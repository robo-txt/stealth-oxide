use std::time::Duration;

use anyhow::{Context, Result};
use chromiumoxide::{Browser, BrowserConfig};
use futures::StreamExt;
use serde_json::Value;
use tokio::time::timeout;

const CREEPJS_URL: &str = "https://abrahamjuliot.github.io/creepjs/";

#[tokio::test]
#[ignore = "requires local Chromium with fake media-device support"]
async fn chromium_fake_media_flags_expose_audio_and_video_devices() -> Result<()> {
    let mut config = BrowserConfig::builder()
        .hide()
        .arg("use-fake-device-for-media-stream")
        .arg("use-fake-ui-for-media-stream");

    if matches!(
        std::env::var("STEALTH_OXIDE_HEADFUL").as_deref(),
        Ok("1") | Ok("true")
    ) {
        config = config.with_head();
    }

    let (mut browser, mut handler) = Browser::launch(config.build().map_err(anyhow::Error::msg)?)
        .await
        .context("failed to launch Chromium with fake media devices")?;
    tokio::spawn(async move {
        while let Some(event) = handler.next().await {
            if let Err(error) = event {
                eprintln!("chromiumoxide handler error: {error:?}");
            }
        }
    });

    let page = timeout(Duration::from_secs(20), browser.new_page(CREEPJS_URL))
        .await
        .context("timed out while opening the media probe")??;
    let observed: Value = timeout(
        Duration::from_secs(20),
        page.evaluate(
            r#"
            (async () => {
                if (!navigator.mediaDevices?.getUserMedia) {
                    throw new Error(JSON.stringify({
                        url: location.href,
                        secureContext: isSecureContext,
                        mediaDevicesAvailable: !!navigator.mediaDevices
                    }));
                }
                const stream = await navigator.mediaDevices.getUserMedia({
                    audio: true,
                    video: true
                });
                const devices = await navigator.mediaDevices.enumerateDevices();
                const result = {
                    secureContext: isSecureContext,
                    deviceKinds: devices.map(device => device.kind).sort(),
                    deviceLabels: devices.map(device => device.label),
                    tracks: stream.getTracks().map(track => ({
                        kind: track.kind,
                        label: track.label,
                        settings: track.getSettings()
                    }))
                };
                stream.getTracks().forEach(track => track.stop());
                return result;
            })()
            "#,
        ),
    )
    .await
    .context("timed out while querying fake media devices")??
    .into_value()?;

    println!("observed fake media devices: {observed:#}");
    timeout(Duration::from_secs(20), browser.close())
        .await
        .context("timed out while closing Chromium")??;

    assert_eq!(observed["secureContext"], true);
    let device_kinds = observed["deviceKinds"]
        .as_array()
        .context("deviceKinds was not an array")?;
    assert!(device_kinds.iter().any(|kind| kind == "audioinput"));
    assert!(device_kinds.iter().any(|kind| kind == "audiooutput"));
    assert!(device_kinds.iter().any(|kind| kind == "videoinput"));
    assert_eq!(
        observed["tracks"]
            .as_array()
            .context("tracks was not an array")?
            .len(),
        2
    );

    Ok(())
}
