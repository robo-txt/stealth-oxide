use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use chromiumoxide::Browser;
use chromiumoxide::Page;
use chromiumoxide::cdp::browser_protocol::target::{
    EventAttachedToTarget, SessionId, SetAutoAttachParams,
};
use chromiumoxide::cdp::js_protocol::runtime::RunIfWaitingForDebuggerParams;
use chromiumoxide::types::{Command, Method, MethodId};
use futures::StreamExt;
use serde_json::{Value, json};
use tokio::sync::Mutex;
use tokio::time::timeout;

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SendMessageToTargetParams {
    message: String,
    session_id: SessionId,
}

impl Method for SendMessageToTargetParams {
    fn identifier(&self) -> MethodId {
        "Target.sendMessageToTarget".into()
    }
}

impl Command for SendMessageToTargetParams {
    type Response = Value;
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct TargetObservation {
    target_type: String,
    url: String,
    target_id: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cdp_url = std::env::var("CDP_URL").unwrap_or_else(|_| "http://127.0.0.1:9222".into());
    let page_url = std::env::var("PROBE_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8765/worker-cdp-probe.html".into());

    let (mut browser, mut handler) = Browser::connect(cdp_url).await?;
    eprintln!("connected to CDP");
    tokio::spawn(async move {
        while let Some(event) = handler.next().await {
            if let Err(error) = event {
                eprintln!("browser handler error: {error:?}");
            }
        }
    });

    let page = browser.new_page("about:blank").await?;
    eprintln!("created page");
    let mut attached = page
        .event_listener::<EventAttachedToTarget>()
        .await
        .context("subscribe to target attachment events")?;
    page.execute(
        SetAutoAttachParams::builder()
            .auto_attach(true)
            .wait_for_debugger_on_start(true)
            .flatten(false)
            .build()
            .map_err(|error| anyhow::anyhow!("build Target.setAutoAttach: {error}"))?,
    )
    .await
    .context("enable Target.setAutoAttach")?;
    eprintln!("enabled target auto-attach");

    let observations = Arc::new(Mutex::new(Vec::<TargetObservation>::new()));
    let target_page = page.clone();
    let target_observations = observations.clone();
    let target_task = tokio::spawn(async move {
        while let Some(event) = attached.next().await {
            let target_type = event.target_info.r#type.clone();
            let target_id = format!("{:?}", event.target_info.target_id);
            let url = event.target_info.url.clone();
            eprintln!("attached target type={target_type} url={url}");
            if matches!(
                target_type.as_str(),
                "worker" | "shared_worker" | "service_worker"
            ) {
                target_observations.lock().await.push(TargetObservation {
                    target_type,
                    url,
                    target_id,
                });
            }

            if let Err(error) = resume_target(&target_page, &event.session_id).await {
                eprintln!(
                    "failed to resume {} target: {error:#}",
                    event.target_info.r#type
                );
            }
        }
    });

    page.goto(page_url).await?;
    eprintln!("navigated probe page");
    let started: Value = timeout(Duration::from_secs(30), page.evaluate(PROBE_SCRIPT))
        .await
        .context("worker CDP probe timed out")??
        .into_value()?;
    eprintln!("page evaluation completed");

    tokio::time::sleep(Duration::from_secs(5)).await;
    let observed: Value = page
        .evaluate("({ results: window.__workerResults })")
        .await
        .context("read asynchronous worker results")?
        .into_value()?;
    let targets = observations.lock().await;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "observed": observed,
            "started": started,
            "attachedTargets": &*targets,
            "targetCoordinatorServiceWorkerPolicy": "current production coordinator excludes service_worker",
        }))?
    );

    page.execute(SetAutoAttachParams::new(false, false)).await?;
    target_task.abort();
    browser.close().await?;
    Ok(())
}

async fn resume_target(page: &Page, session_id: &SessionId) -> Result<()> {
    let message = serde_json::to_string(&json!({
        "id": 1,
        "method": RunIfWaitingForDebuggerParams::default().identifier(),
        "params": {},
    }))?;
    page.execute(SendMessageToTargetParams {
        message,
        session_id: session_id.clone(),
    })
    .await?;
    Ok(())
}

const PROBE_SCRIPT: &str = r#"
(() => {
  const snapshot = () => ({
    userAgent: navigator.userAgent,
    platform: navigator.platform,
    language: navigator.language,
    languages: [...navigator.languages],
    hardwareConcurrency: navigator.hardwareConcurrency,
    deviceMemory: navigator.deviceMemory ?? null,
    webdriver: navigator.webdriver ?? null,
    locale: Intl.DateTimeFormat().resolvedOptions().locale,
    timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
  });

  window.__workerResults = {};
  navigator.serviceWorker.onmessage = event => {
    window.__workerResults.serviceWorker = event.data;
  };

  const dedicatedSource = `
    const gpu = () => {
      try {
        const canvas = new OffscreenCanvas(16, 16);
        const gl = canvas.getContext('webgl') || canvas.getContext('webgl2');
        const debug = gl?.getExtension('WEBGL_debug_renderer_info');
        return {
          available: !!gl,
          vendor: debug ? gl.getParameter(debug.UNMASKED_VENDOR_WEBGL) : null,
          renderer: debug ? gl.getParameter(debug.UNMASKED_RENDERER_WEBGL) : null,
          contextLost: gl ? gl.isContextLost() : null,
        };
      } catch (error) { return { available: false, error: String(error) }; }
    };
    self.onmessage = () => self.postMessage({ kind: 'dedicated', ...snapshot(), webgl: gpu() });
    function snapshot() {
      return {
        userAgent: navigator.userAgent,
        platform: navigator.platform,
        language: navigator.language,
        languages: [...navigator.languages],
        hardwareConcurrency: navigator.hardwareConcurrency,
        deviceMemory: navigator.deviceMemory ?? null,
        webdriver: navigator.webdriver ?? null,
        locale: Intl.DateTimeFormat().resolvedOptions().locale,
        timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
      };
    }
  `;
  const dedicatedUrl = URL.createObjectURL(new Blob([dedicatedSource], { type: 'text/javascript' }));
  const dedicatedWorker = new Worker(dedicatedUrl);
  dedicatedWorker.onmessage = event => { window.__workerResults.dedicated = event.data; };
  dedicatedWorker.onerror = event => { window.__workerResults.dedicatedError = event.message || 'dedicated worker error'; };
  dedicatedWorker.postMessage({ command: 'snapshot' });

  const sharedWorker = new SharedWorker('/worker-cdp-shared.js');
  sharedWorker.port.onmessage = event => { window.__workerResults.shared = event.data; };
  sharedWorker.onerror = event => { window.__workerResults.sharedError = event.message || 'shared worker error'; };
  sharedWorker.port.start();
  sharedWorker.port.postMessage({ command: 'snapshot' });

  navigator.serviceWorker.register('/worker-cdp-service.js').then(registration => {
    return navigator.serviceWorker.ready.then(() => registration.active);
  }).then(active => {
    if (!active) throw new Error('service worker has no active registration');
    const channel = new MessageChannel();
    channel.port1.onmessage = event => { window.__workerResults.serviceWorker = event.data; };
    channel.port1.start();
    active.postMessage({ command: 'snapshot' }, [channel.port2]);
  }).catch(error => { window.__workerResults.serviceWorkerError = String(error); });

  return { page: snapshot(), started: true };
})()
"#;
