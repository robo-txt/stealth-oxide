use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde_json::Value;
use tokio::time::{Instant, sleep};

mod common;
use common::BrowserSession as StealthBrowser;
use stealth_oxide::profiles::chrome_windows::chrome_windows;

const CREEPJS_URL: &str = "https://abrahamjuliot.github.io/creepjs/";

#[tokio::main]
async fn main() -> Result<()> {
    let browser = StealthBrowser::launch(chrome_windows()).await?;
    let version = browser.version().await?;
    let page = browser
        .new_page(CREEPJS_URL)
        .await
        .context("failed to open CreepJS")?;

    let deadline = Instant::now() + Duration::from_secs(60);
    let report = loop {
        let report: Value = page
            .inner()
            .evaluate(
                r#"
                (async () => {
                    const section = (name, columns = 1) => {
                        const heading = [...document.querySelectorAll('strong')]
                            .find(element => element.textContent?.trim() === name);
                        const first = heading?.closest('[class*="col-"]');
                        const result = [];
                        let current = first;
                        for (let index = 0; current && index < columns; index += 1) {
                            result.push(current.innerText?.trim() || '');
                            current = current.nextElementSibling;
                        }
                        return result;
                    };
                    const devices = navigator.mediaDevices?.enumerateDevices
                        ? await navigator.mediaDevices.enumerateDevices()
                        : [];
                    const codecs = kind => globalThis.RTCRtpSender?.getCapabilities?.(kind)?.codecs
                        ?.map(({ mimeType, clockRate, channels, sdpFmtpLine }) => ({
                            mimeType, clockRate, channels, sdpFmtpLine
                        })) || [];
                    return {
                        creepjs: {
                            webrtc: section('WebRTC', 2),
                            media: section('Media', 1)
                        },
                        direct: {
                            mediaDevicesAvailable: !!navigator.mediaDevices,
                            deviceKinds: devices.map(device => device.kind).sort(),
                            audioCodecs: codecs('audio'),
                            videoCodecs: codecs('video'),
                            mediaCapabilitiesAvailable: !!navigator.mediaCapabilities,
                            mediaRecorderAvailable: 'MediaRecorder' in globalThis,
                            mediaSourceAvailable: 'MediaSource' in globalThis,
                            rtcPeerConnectionAvailable: 'RTCPeerConnection' in globalThis
                        }
                    };
                })()
                "#,
            )
            .await?
            .into_value()?;

        let ready = report["creepjs"]["webrtc"][0]
            .as_str()
            .is_some_and(|text| text.contains("WebRTC"))
            && report["creepjs"]["media"][0]
                .as_str()
                .is_some_and(|text| text.contains("Media"));
        if ready {
            break report;
        }
        if Instant::now() >= deadline {
            bail!("timed out waiting for CreepJS WebRTC/media results");
        }
        sleep(Duration::from_millis(500)).await;
    };

    println!("Chromium: {version}");
    println!("{}", serde_json::to_string_pretty(&report)?);

    browser.close().await?;
    Ok(())
}
