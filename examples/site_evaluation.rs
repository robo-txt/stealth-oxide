use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use chromiumoxide::cdp::browser_protocol::network::{
    EnableParams, EventLoadingFailed, EventLoadingFinished, EventRequestServedFromCache,
    EventRequestWillBeSent, EventRequestWillBeSentExtraInfo, EventResponseReceived,
    EventResponseReceivedExtraInfo,
};
use chromiumoxide::{Browser, BrowserConfig, Page};
use futures::StreamExt;
use serde_json::{Value, json};
use stealth_oxide::{
    ChromeLanguageConfig, CompatibilityStatus, NetworkAudit, PlatformProfile, StealthConfig,
    compare_browser_versions,
};

#[derive(Debug)]
struct Arguments {
    url: String,
    expected_text: Vec<String>,
    expected_selectors: Vec<String>,
    minimum_html_bytes: usize,
    wait: Duration,
    timeout: Duration,
    profile_dir: Option<PathBuf>,
    run_label: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let arguments = parse_arguments()?;
    let profile = PlatformProfile::Linux.profile();
    let language = ChromeLanguageConfig::from_profile(&profile);
    let mut browser_builder = BrowserConfig::builder()
        .hide()
        .arg(language.chrome_argument());
    if let Some(profile_dir) = &arguments.profile_dir {
        browser_builder = browser_builder.user_data_dir(profile_dir);
    }
    let browser_config = browser_builder.build().map_err(anyhow::Error::msg)?;
    let started = Instant::now();
    let (mut browser, mut handler) = Browser::launch(browser_config).await?;
    tokio::spawn(async move {
        while let Some(event) = handler.next().await {
            if let Err(error) = event {
                eprintln!("chromiumoxide handler error: {error:?}");
            }
        }
    });

    let runtime = browser.version().await?;
    let compatibility = compare_browser_versions(profile.version(), &runtime.product);
    if !matches!(compatibility, CompatibilityStatus::Compatible { .. }) {
        browser.close().await?;
        bail!(
            "refusing an incomparable evaluation: profile {} does not match runtime {} ({compatibility:?})",
            profile
                .version()
                .map(|version| version.chrome_version.as_str())
                .unwrap_or("unversioned"),
            runtime.product,
        );
    }

    let page = browser.new_page("about:blank").await?;
    let audit = Arc::new(tokio::sync::Mutex::new(NetworkAudit::default()));
    let collectors = start_collectors(&page, audit.clone()).await?;
    page.execute(EnableParams::default()).await?;
    StealthConfig::from_profile(profile).apply(&page).await?;
    page.goto(&arguments.url).await?;
    let assertion_started = Instant::now();
    let mut observation = observe_page(&page, &arguments.expected_selectors).await?;
    loop {
        let body = observation["body"].as_str().unwrap_or("");
        let text_ready = arguments.expected_text.is_empty()
            || arguments
                .expected_text
                .iter()
                .any(|expected| body.contains(expected));
        let selectors_ready = observation["selectors"]
            .as_array()
            .is_none_or(|selectors| selectors.iter().all(|present| present == true));
        if text_ready && selectors_ready || assertion_started.elapsed() >= arguments.timeout {
            break;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
        if let Ok(next) = observe_page(&page, &arguments.expected_selectors).await {
            observation = next;
        }
    }
    tokio::time::sleep(arguments.wait).await;

    let html = page.content().await?;
    let title = page.get_title().await?.unwrap_or_default();
    let final_url = page.url().await?.unwrap_or_default();
    observation = observe_page(&page, &arguments.expected_selectors).await?;

    let body = observation["body"].as_str().unwrap_or("");
    let status = observation["responseStatus"].as_u64();
    let normalized_title = title.to_lowercase();
    let body_sample = body.chars().take(4_000).collect::<String>().to_lowercase();
    let challenge_terms = [
        "captcha",
        "verify you are human",
        "access denied",
        "just a moment",
        "javascript challenge",
        "checking your browser",
    ];
    let challenge_element_count = observation["challengeElements"].as_u64().unwrap_or(0);
    let challenge_in_title = challenge_terms
        .iter()
        .any(|term| normalized_title.contains(term));
    let challenge_on_error_page = status.is_some_and(|status| status >= 400)
        && challenge_terms
            .iter()
            .any(|term| body_sample.contains(term));
    let challenge_url = final_url.contains("js_challenge=");
    let expected_text_found = arguments.expected_text.is_empty()
        || arguments
            .expected_text
            .iter()
            .any(|expected| body.contains(expected));
    let expected_selectors_found = observation["selectors"]
        .as_array()
        .is_none_or(|selectors| selectors.iter().all(|present| present == true));
    let challenge_detected = challenge_in_title
        || challenge_on_error_page
        || challenge_url
        || (challenge_element_count > 0 && !expected_text_found);
    let http_ok = status.is_some_and(|status| (200..300).contains(&status));
    let success = http_ok
        && expected_text_found
        && expected_selectors_found
        && html.len() >= arguments.minimum_html_bytes
        && !challenge_detected;
    for collector in collectors {
        collector.abort();
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "success": success,
            "url": arguments.url,
            "final_url": final_url,
            "title": title,
            "status": status,
            "html_bytes": html.len(),
            "minimum_html_bytes": arguments.minimum_html_bytes,
            "expected_text": arguments.expected_text,
            "expected_text_found": expected_text_found,
            "expected_selectors": arguments.expected_selectors,
            "expected_selectors_found": expected_selectors_found,
            "challenge_detected": challenge_detected,
            "challenge_evidence": {
                "element_count": challenge_element_count,
                "title": challenge_in_title,
                "error_page_text": challenge_on_error_page,
                "url": challenge_url,
            },
            "runtime_product": runtime.product,
            "profile_compatibility": format!("{compatibility:?}"),
            "platform_profile": "linux",
            "profile_directory_configured": arguments.profile_dir.is_some(),
            "run_label": arguments.run_label,
            "wall_time_ms": started.elapsed().as_secs_f64() * 1_000.0,
        }))?
    );

    browser.close().await?;
    Ok(())
}

fn parse_arguments() -> Result<Arguments> {
    let mut arguments = std::env::args().skip(1);
    let mut url = None;
    let mut expected_text = Vec::new();
    let mut expected_selectors = Vec::new();
    let mut minimum_html_bytes = 100;
    let mut wait = Duration::from_millis(250);
    let mut timeout = Duration::from_secs(30);
    let mut profile_dir = None;
    let mut run_label = None;

    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--url" => url = Some(arguments.next().context("--url requires a value")?),
            "--expect" => {
                expected_text.push(arguments.next().context("--expect requires a value")?)
            }
            "--expect-selector" => expected_selectors.push(
                arguments
                    .next()
                    .context("--expect-selector requires a value")?,
            ),
            "--min-html-bytes" => {
                minimum_html_bytes = arguments
                    .next()
                    .context("--min-html-bytes requires a value")?
                    .parse()
                    .context("--min-html-bytes must be an integer")?;
            }
            "--wait" => {
                wait = Duration::from_secs(
                    arguments
                        .next()
                        .context("--wait requires a value")?
                        .parse()
                        .context("--wait must be an integer number of seconds")?,
                );
            }
            "--timeout" => {
                timeout = Duration::from_secs(
                    arguments
                        .next()
                        .context("--timeout requires a value")?
                        .parse()
                        .context("--timeout must be an integer number of seconds")?,
                );
            }
            "--profile-dir" => {
                profile_dir = Some(PathBuf::from(
                    arguments.next().context("--profile-dir requires a value")?,
                ));
            }
            "--run-label" => {
                run_label = Some(arguments.next().context("--run-label requires a value")?);
            }
            other => bail!("unknown argument: {other}"),
        }
    }

    let url = url.context(
        "usage: site_evaluation --url URL [--expect TEXT] [--expect-selector CSS] [--min-html-bytes N] [--timeout SECONDS] [--profile-dir PATH]",
    )?;
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        bail!("--url must use http:// or https://");
    }

    Ok(Arguments {
        url,
        expected_text,
        expected_selectors,
        minimum_html_bytes,
        wait,
        timeout,
        profile_dir,
        run_label,
    })
}

async fn observe_page(page: &Page, selectors: &[String]) -> Result<Value> {
    let selectors = serde_json::to_string(selectors)?;
    Ok(page
        .evaluate(format!(
            r#"(() => {{
                const navigation = performance.getEntriesByType('navigation')[0];
                const body = document.body?.innerText || '';
                const selectors = {selectors};
                return {{
                    body,
                    responseStatus: navigation?.responseStatus ?? null,
                    selectors: selectors.map((selector) => {{
                        try {{ return document.querySelector(selector) !== null; }}
                        catch {{ return false; }}
                    }}),
                    challengeElements: document.querySelectorAll(
                        'iframe[src*="captcha" i], #challenge-form, .cf-challenge, [data-sitekey]'
                    ).length
                }};
            }})()"#
        ))
        .await?
        .into_value()?)
}

async fn start_collectors(
    page: &Page,
    audit: Arc<tokio::sync::Mutex<NetworkAudit>>,
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
