use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use chromiumoxide::Browser;
use chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat;
use chromiumoxide::page::ScreenshotParams;
use futures::StreamExt;
use serde_json::Value;
use stealth_oxide::profiles::chrome_linux::chrome_linux;
use stealth_oxide::{StealthConfig, TargetCoordinator};
use tokio::time::timeout;

const CREEPJS_URL: &str = "https://abrahamjuliot.github.io/creepjs/";

const SNAPSHOT: &str = r#"
(() => {
  const fp = globalThis.Fingerprint;
  const headless = fp?.headless;
  const worker = fp?.workerScope;
  const webgl = fp?.canvasWebgl;
  const text = document.body?.innerText || '';
  const lines = text.split(/\n+/).map(line => line.trim()).filter(Boolean);
  return {
    title: document.title,
    url: location.href,
    pageTextLength: text.length,
    headless: headless ? {
      chromium: headless.chromium ?? null,
      headless: headless.headless ?? null,
      likeHeadless: headless.likeHeadless ?? null,
      stealth: headless.stealth ?? null,
      platform: headless.platform ?? null,
    } : null,
    webgl: webgl ? {
      lied: webgl.lied ?? null,
      gpu: webgl.gpu ?? null,
      renderer: webgl.renderer ?? null,
      vendor: webgl.vendor ?? null,
    } : null,
    worker: worker ? {
      type: worker.type ?? null,
      userAgent: worker.userAgent ?? null,
      platform: worker.platform ?? null,
      hardwareConcurrency: worker.hardwareConcurrency ?? null,
      deviceMemory: worker.deviceMemory ?? null,
      timezone: worker.timezone ?? null,
      lied: worker.lied ?? null,
    } : null,
    navigator: fp?.navigator ? {
      lied: fp.navigator.lied ?? null,
      platform: fp.navigator.platform ?? null,
      userAgent: fp.navigator.userAgent ?? null,
    } : null,
    screen: fp?.screen ? {
      lied: fp.screen.lied ?? null,
      width: fp.screen.width ?? null,
      height: fp.screen.height ?? null,
      availWidth: fp.screen.availWidth ?? null,
      availHeight: fp.screen.availHeight ?? null,
    } : null,
    signalLines: lines.filter(line => /%|headless|stealth|swiftshader|mesa|llvmpipe|gpu|lie/i.test(line)).slice(0, 100),
  };
})()
"#;

#[tokio::main]
async fn main() -> Result<()> {
    let cdp_url = std::env::var("CDP_URL").unwrap_or_else(|_| "http://127.0.0.1:9222".into());
    let screenshot = std::env::var("CREEPJS_SCREENSHOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp/stealth-oxide-docker-creepjs.png"));
    let wait_seconds = std::env::var("CREEPJS_WAIT_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(15);

    let (mut browser, mut handler) = Browser::connect(cdp_url).await?;
    tokio::spawn(async move {
        while let Some(event) = handler.next().await {
            if let Err(error) = event {
                eprintln!("browser handler error: {error:?}");
            }
        }
    });

    let page = browser.new_page("about:blank").await?;
    let config = StealthConfig::from_profile(chrome_linux());
    config.apply_browser(&browser).await?;
    config.apply(&page).await?;
    let coordinator = TargetCoordinator::new(&config)?;
    let mut attached_targets = coordinator.enable(&page).await?;
    let coordinator_for_task = coordinator.clone();
    let target_page = page.clone();
    let coordinator_task = tokio::spawn(async move {
        while let Some(event) = attached_targets.next().await {
            if let Err(error) = coordinator_for_task.apply(&target_page, &event).await {
                eprintln!("target configuration failed: {error}");
            }
        }
    });
    page.goto(CREEPJS_URL).await?;
    tokio::time::sleep(Duration::from_secs(wait_seconds)).await;

    let observed: Value = timeout(Duration::from_secs(30), page.evaluate(SNAPSHOT))
        .await
        .context("CreepJS snapshot timed out")??
        .into_value()?;
    page.save_screenshot(
        ScreenshotParams::builder()
            .format(CaptureScreenshotFormat::Png)
            .full_page(true)
            .build(),
        &screenshot,
    )
    .await
    .with_context(|| {
        format!(
            "failed to save CreepJS screenshot to {}",
            screenshot.display()
        )
    })?;

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "url": CREEPJS_URL,
            "waitSeconds": wait_seconds,
            "screenshot": screenshot,
            "observed": observed,
        }))?
    );

    coordinator.disable(&page).await?;
    coordinator_task.abort();
    browser.close().await?;
    Ok(())
}
