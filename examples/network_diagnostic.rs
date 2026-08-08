use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use chromiumoxide::cdp::browser_protocol::network::{
    EnableParams, EventLoadingFailed, EventRequestWillBeSent, EventResponseReceived,
};
use chromiumoxide::cdp::browser_protocol::page::GetFrameTreeParams;
use futures::StreamExt;
use serde::Serialize;
use serde_json::Value;
use stealth_oxide::redaction;
use stealth_oxide::{
    PlatformProfile, classify_failure, classify_resource, sanitize_error_name,
    sanitize_initiator_origin,
};

mod common;
use common::{BrowserSession, ExampleLaunch};

#[derive(Debug)]
struct Arguments {
    url: String,
    wait: Duration,
    native: bool,
    headful: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ResponseRecord {
    url: String,
    status: i64,
    resource_type: String,
    protocol: Option<String>,
    remote_ip: Option<String>,
    headers: BTreeMap<String, String>,
    frame_id: Option<String>,
    loader_id: String,
    scope: ResourceScopeJson,
    initiator_type: Option<String>,
    initiator_origin: Option<String>,
}

#[derive(Debug, Clone)]
struct RequestMeta {
    frame_id: Option<String>,
    loader_id: String,
    initiator_type: Option<String>,
    initiator_origin: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ResourceScopeJson {
    classification: String,
    evidence: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FailureRecord {
    resource_type: String,
    category: String,
    error_name: String,
    error: String,
    blocked_reason: Option<String>,
    cors_error: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Report {
    mode: &'static str,
    browser_mode: &'static str,
    requested_url: String,
    final_url: String,
    status: Option<u64>,
    title: String,
    response_count: usize,
    status_counts: BTreeMap<i64, usize>,
    host_counts: BTreeMap<String, usize>,
    document_responses: Vec<ResponseRecord>,
    suspicious_responses: Vec<ResponseRecord>,
    failures: Vec<FailureRecord>,
    failure_counts: BTreeMap<String, usize>,
    topology_coverage: &'static str,
}

#[tokio::main]
async fn main() -> Result<()> {
    let arguments = parse_arguments()?;
    let browser = BrowserSession::launch_with(
        PlatformProfile::Linux.profile(),
        ExampleLaunch {
            native: arguments.native,
            headful: arguments.headful,
            ..ExampleLaunch::default()
        },
    )
    .await?;
    let page = browser.new_blank_page().await?;
    page.inner().execute(EnableParams::default()).await?;
    let main_frame_id = page
        .inner()
        .execute(GetFrameTreeParams {})
        .await?
        .result
        .frame_tree
        .frame
        .id
        .inner()
        .clone();

    let mut response_events = page
        .inner()
        .event_listener::<EventResponseReceived>()
        .await?;
    let mut failure_events = page.inner().event_listener::<EventLoadingFailed>().await?;
    let responses = Arc::new(Mutex::new(Vec::new()));
    let failures = Arc::new(Mutex::new(Vec::new()));
    let requests = Arc::new(Mutex::new(BTreeMap::<String, RequestMeta>::new()));

    let mut request_events = page
        .inner()
        .event_listener::<EventRequestWillBeSent>()
        .await?;
    let request_sink = Arc::clone(&requests);
    let request_task = tokio::spawn(async move {
        while let Some(event) = request_events.next().await {
            request_sink.lock().unwrap().insert(
                event.request_id.inner().clone(),
                RequestMeta {
                    frame_id: event.frame_id.as_ref().map(|id| id.inner().clone()),
                    loader_id: event.loader_id.inner().clone(),
                    initiator_type: Some(format!("{:?}", event.initiator.r#type)),
                    initiator_origin: sanitize_initiator_origin(event.initiator.url.as_deref()),
                },
            );
        }
    });

    let response_sink = Arc::clone(&responses);
    let request_lookup = Arc::clone(&requests);
    let response_main_frame = main_frame_id.clone();
    let response_task = tokio::spawn(async move {
        while let Some(event) = response_events.next().await {
            let meta = request_lookup
                .lock()
                .unwrap()
                .get(event.request_id.inner())
                .cloned();
            let frame_id = event
                .frame_id
                .as_ref()
                .map(|id| id.inner().clone())
                .or_else(|| meta.as_ref().and_then(|meta| meta.frame_id.clone()));
            let scope = classify_resource(
                frame_id.as_deref(),
                Some(response_main_frame.as_str()),
                &format!("{:?}", event.r#type),
            );
            response_sink.lock().unwrap().push(ResponseRecord {
                url: sanitize_url(&event.response.url),
                status: event.response.status,
                resource_type: format!("{:?}", event.r#type),
                protocol: event.response.protocol.clone(),
                remote_ip: event.response.remote_ip_address.clone(),
                headers: redaction::safe_headers(event.response.headers.inner()),
                frame_id,
                loader_id: meta
                    .as_ref()
                    .map(|meta| meta.loader_id.clone())
                    .unwrap_or_default(),
                scope: ResourceScopeJson {
                    classification: format!("{scope:?}"),
                    evidence: if meta.is_some() {
                        "request-correlated"
                    } else {
                        "response-only"
                    }
                    .to_string(),
                },
                initiator_type: meta.as_ref().and_then(|meta| meta.initiator_type.clone()),
                initiator_origin: meta.as_ref().and_then(|meta| meta.initiator_origin.clone()),
            });
        }
    });

    let failure_sink = Arc::clone(&failures);
    let failure_task = tokio::spawn(async move {
        while let Some(event) = failure_events.next().await {
            let blocked_reason = event
                .blocked_reason
                .as_ref()
                .map(|value| format!("{value:?}"));
            let category = classify_failure(
                &event.error_text,
                event.canceled.unwrap_or(false),
                blocked_reason.as_deref(),
                event.cors_error_status.is_some(),
            );
            failure_sink.lock().unwrap().push(FailureRecord {
                resource_type: format!("{:?}", event.r#type),
                category: category.to_string(),
                error_name: sanitize_error_name(&event.error_text),
                error: sanitize_error_name(&event.error_text),
                blocked_reason,
                cors_error: event.cors_error_status.is_some(),
            });
        }
    });

    page.goto(&arguments.url).await?;
    tokio::time::sleep(arguments.wait).await;
    let snapshot: Value = page
        .inner()
        .evaluate(
            r#"({
                finalUrl: location.href,
                status: performance.getEntriesByType('navigation')[0]?.responseStatus ?? null,
                title: document.title
            })"#,
        )
        .await?
        .into_value()?;

    response_task.abort();
    request_task.abort();
    failure_task.abort();
    let responses = std::mem::take(&mut *responses.lock().unwrap());
    let failures = std::mem::take(&mut *failures.lock().unwrap());
    let mut failure_counts = BTreeMap::new();
    for failure in &failures {
        *failure_counts.entry(failure.category.clone()).or_default() += 1;
    }
    let mut status_counts = BTreeMap::new();
    let mut host_counts = BTreeMap::new();
    for response in &responses {
        *status_counts.entry(response.status).or_insert(0) += 1;
        if let Ok(url) = url::Url::parse(&response.url)
            && let Some(host) = url.host_str()
        {
            *host_counts.entry(host.to_string()).or_insert(0) += 1;
        }
    }

    let (document_responses, other): (Vec<_>, Vec<_>) = responses
        .into_iter()
        .partition(|response| response.resource_type == "Document");
    let suspicious_responses = other
        .into_iter()
        .filter(|response| response.status >= 400 || suspicious_url(&response.url))
        .take(100)
        .collect();
    let report = Report {
        mode: if arguments.native {
            "native"
        } else {
            "stealth"
        },
        browser_mode: if arguments.headful {
            "headful"
        } else {
            "headless"
        },
        requested_url: arguments.url,
        final_url: snapshot["finalUrl"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        status: snapshot["status"].as_u64(),
        title: snapshot["title"].as_str().unwrap_or_default().to_string(),
        response_count: status_counts.values().sum(),
        status_counts,
        host_counts,
        document_responses,
        suspicious_responses,
        failures,
        failure_counts,
        topology_coverage: "page-target-only; worker and OOPIF targets may be unobserved",
    };

    browser.close().await?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn sanitize_url(value: &str) -> String {
    redaction::url(value)
}

fn suspicious_url(value: &str) -> bool {
    let lowercase = value.to_ascii_lowercase();
    [
        "akamai",
        "captcha",
        "challenge",
        "cloudflare",
        "datadome",
        "perimeterx",
        "/_px",
        "/px/",
        "recaptcha",
        "turnstile",
    ]
    .iter()
    .any(|needle| lowercase.contains(needle))
}

fn parse_arguments() -> Result<Arguments> {
    let mut url = None;
    let mut wait = 5;
    let mut native = false;
    let mut headful = false;
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--url" => url = Some(arguments.next().context("--url requires a value")?),
            "--wait" => {
                wait = arguments
                    .next()
                    .context("--wait requires seconds")?
                    .parse()
                    .context("--wait must be an integer")?;
            }
            "--native" => native = true,
            "--headful" => headful = true,
            "--help" | "-h" => {
                println!(
                    "network_diagnostic --url <URL> [--wait <SECONDS>] [--native] [--headful]"
                );
                std::process::exit(0);
            }
            value => bail!("unknown argument: {value}; use --help"),
        }
    }
    let url = url.context("--url is required")?;
    let parsed = url::Url::parse(&url).context("invalid --url value")?;
    if !matches!(parsed.scheme(), "http" | "https") {
        bail!("diagnostic URL must use http or https");
    }
    Ok(Arguments {
        url,
        wait: Duration::from_secs(wait),
        native,
        headful,
    })
}
