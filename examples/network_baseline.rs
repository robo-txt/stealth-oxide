use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use anyhow::Result;
use chromiumoxide::cdp::browser_protocol::network::{
    EnableParams, EventLoadingFailed, EventLoadingFinished, EventRequestServedFromCache,
    EventRequestWillBeSent, EventRequestWillBeSentExtraInfo, EventResponseReceived,
    EventResponseReceivedExtraInfo,
};
use chromiumoxide::{Browser, BrowserConfig, Page};
use futures::StreamExt;
use serde_json::json;
use stealth_oxide::{
    NetworkAudit, PlatformProfile, StealthConfig, TargetCoordinator, compare_browser_versions,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<()> {
    let configured = std::env::args().any(|argument| argument == "--configured");
    let profile = PlatformProfile::Linux.profile();
    let (fixture_url, fixture_task) = start_fixture().await?;
    let browser_config = BrowserConfig::builder()
        .hide()
        .build()
        .map_err(anyhow::Error::msg)?;
    let (mut browser, mut handler) = Browser::launch(browser_config).await?;
    tokio::spawn(async move {
        while let Some(event) = handler.next().await {
            if let Err(error) = event {
                eprintln!("chromiumoxide handler error: {error:?}");
            }
        }
    });

    let page = browser.new_page("about:blank").await?;
    let audit = Arc::new(Mutex::new(NetworkAudit::default()));
    let collectors = start_collectors(&page, audit.clone()).await?;
    page.execute(EnableParams::default()).await?;
    let target_collector = if configured {
        let stealth = StealthConfig::from_profile(profile.clone());
        stealth.apply(&page).await?;
        let coordinator = TargetCoordinator::new(&stealth)?;
        let mut targets = coordinator.enable(&page).await?;
        let target_page = page.clone();
        Some(tokio::spawn(async move {
            while let Some(event) = targets.next().await {
                if let Err(error) = coordinator.apply(&target_page, &event).await {
                    eprintln!(
                        "target configuration failed for {}: {error}",
                        event.target_info.r#type
                    );
                }
            }
        }))
    } else {
        None
    };

    page.goto(&fixture_url).await?;
    tokio::time::sleep(Duration::from_secs(5)).await;

    for collector in collectors {
        collector.abort();
    }
    if let Some(target_collector) = target_collector {
        target_collector.abort();
    }
    let audit = audit.lock().await;
    let summary = audit.summary();
    let runtime = browser.version().await?;
    let requests = audit
        .requests()
        .map(|request| {
            json!({
                "url": request.url,
                "method": request.method,
                "type": request.resource_type,
                "status": request.status,
                "redirects": request.redirects.len(),
                "protocol": request.protocol,
                "cache": request.from_cache,
                "service_worker": request.from_service_worker,
                "connection_reused": request.connection_reused,
                "failure": request.failure_category.map(|category| category.to_string()),
                "finished": request.finished,
                "identity_headers": request.request_identity_headers,
                "eligible_high_entropy_hints": request.eligible_high_entropy_hints,
                "sent_high_entropy_hints": request.sent_high_entropy_hints,
            })
        })
        .collect::<Vec<_>>();
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "mode": if configured { "configured" } else { "native" },
            "runtime_product": runtime.product,
            "profile_compatibility": configured.then(|| format!(
                "{:?}",
                compare_browser_versions(profile.version(), &runtime.product)
            )),
            "summary": {
                "requests": summary.requests,
                "finished": summary.finished,
                "cache_hits": summary.cache_hits,
                "service_worker_responses": summary.service_worker_responses,
                "protocols": summary.protocols,
                "failures": summary.failures,
            },
            "requests": requests,
        }))?
    );

    fixture_task.abort();
    browser.close().await?;
    Ok(())
}

async fn start_collectors(
    page: &Page,
    audit: Arc<Mutex<NetworkAudit>>,
) -> Result<Vec<tokio::task::JoinHandle<()>>> {
    macro_rules! collect {
        ($event:ty, $method:ident) => {{
            let mut events = page.event_listener::<$event>().await?;
            let audit = audit.clone();
            tokio::spawn(async move {
                while let Some(event) = events.next().await {
                    audit.lock().await.$method(&event);
                }
            })
        }};
    }

    Ok(vec![
        collect!(EventRequestWillBeSent, observe_request_will_be_sent),
        collect!(EventRequestWillBeSentExtraInfo, observe_request_extra_info),
        collect!(EventResponseReceived, observe_response_received),
        collect!(EventResponseReceivedExtraInfo, observe_response_extra_info),
        collect!(
            EventRequestServedFromCache,
            observe_request_served_from_cache
        ),
        collect!(EventLoadingFinished, observe_loading_finished),
        collect!(EventLoadingFailed, observe_loading_failed),
    ])
}

async fn start_fixture() -> Result<(String, tokio::task::JoinHandle<()>)> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
    let address = listener.local_addr()?;
    let retry_count = Arc::new(AtomicUsize::new(0));
    let task = tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let retry_count = retry_count.clone();
            tokio::spawn(async move {
                let _ = serve_fixture_request(stream, retry_count).await;
            });
        }
    });
    Ok((format!("http://{address}/start"), task))
}

async fn serve_fixture_request(
    mut stream: tokio::net::TcpStream,
    retry_count: Arc<AtomicUsize>,
) -> Result<()> {
    let mut buffer = vec![0_u8; 8192];
    let read = stream.read(&mut buffer).await?;
    let request = String::from_utf8_lossy(&buffer[..read]);
    let path = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/");
    let (status, headers, body) = fixture_response(path, &retry_count);
    let response = format!(
        "HTTP/1.1 {status}\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes()).await?;
    Ok(())
}

fn fixture_response(path: &str, retry_count: &AtomicUsize) -> (&'static str, String, String) {
    match path.split('?').next().unwrap_or(path) {
        "/start" => (
            "302 Found",
            "Location: /page\r\nAccept-CH: Sec-CH-UA-Arch, Sec-CH-UA-Bitness\r\n".into(),
            String::new(),
        ),
        "/page" => (
            "200 OK",
            "Content-Type: text/html\r\nAccept-CH: Sec-CH-UA-Arch, Sec-CH-UA-Bitness\r\n".into(),
            r#"<!doctype html><title>network fixture</title>
<iframe src="/frame"></iframe>
<script>
const workerResult = new Promise((resolve, reject) => {
  const worker = new Worker('/worker.js');
  worker.onmessage = event => resolve(event.data);
  worker.onerror = reject;
});
const sharedWorkerResult = new Promise((resolve, reject) => {
  if (!('SharedWorker' in globalThis)) return resolve('unsupported');
  const worker = new SharedWorker('/shared-worker.js');
  worker.port.onmessage = event => resolve(event.data);
  worker.onerror = reject;
  worker.port.start();
});
const serviceWorkerResult = (async () => {
  if (!('serviceWorker' in navigator)) return 'unsupported';
  await navigator.serviceWorker.register('/service-worker.js');
  await navigator.serviceWorker.ready;
  return fetch('/sw-controlled').then(response => response.text());
})();
window.open('/popup', '_blank');
Promise.all([
  fetch('/cache').then(() => fetch('/cache')),
  fetch('/retry'),
  fetch('/burst?id=1'), fetch('/burst?id=2'), fetch('/burst?id=3'),
  workerResult, sharedWorkerResult, serviceWorkerResult
]).then(() => { document.title = 'fixture-complete' })
</script>"#
                .into(),
        ),
        "/frame" => (
            "200 OK",
            "Content-Type: text/html\r\n".into(),
            "<!doctype html><title>frame</title><script>fetch('/frame-resource')</script>".into(),
        ),
        "/popup" => (
            "200 OK",
            "Content-Type: text/html\r\n".into(),
            "<!doctype html><title>popup</title><script>fetch('/popup-resource')</script>".into(),
        ),
        "/worker.js" => (
            "200 OK",
            "Content-Type: text/javascript\r\n".into(),
            "fetch('/worker-resource').then(() => postMessage('done'))".into(),
        ),
        "/shared-worker.js" => (
            "200 OK",
            "Content-Type: text/javascript\r\n".into(),
            "onconnect = event => { const port = event.ports[0]; fetch('/shared-worker-resource').then(() => port.postMessage('done')); }".into(),
        ),
        "/service-worker.js" => (
            "200 OK",
            "Content-Type: text/javascript\r\nService-Worker-Allowed: /\r\nCache-Control: no-store\r\n".into(),
            "self.addEventListener('install', event => { self.skipWaiting(); event.waitUntil(fetch('/sw-install')); }); self.addEventListener('activate', event => event.waitUntil(self.clients.claim())); self.addEventListener('fetch', event => { if (new URL(event.request.url).pathname === '/sw-controlled') event.respondWith(new Response('service-worker', { headers: { 'Content-Type': 'text/plain' } })); });".into(),
        ),
        "/cache" => (
            "200 OK",
            "Content-Type: text/plain\r\nCache-Control: public, max-age=300\r\nETag: baseline\r\n"
                .into(),
            "cacheable".into(),
        ),
        "/retry" if retry_count.fetch_add(1, Ordering::SeqCst) == 0 => (
            "503 Service Unavailable",
            "Content-Type: text/plain\r\nRetry-After: 1\r\n".into(),
            "retry later".into(),
        ),
        _ => ("200 OK", "Content-Type: text/plain\r\n".into(), "ok".into()),
    }
}
