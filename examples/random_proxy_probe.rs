use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use rand::seq::IteratorRandom;
use serde::Serialize;
use serde_json::Value;
use stealth_oxide::PlatformProfile;
use tokio::time::timeout;

mod common;
use common::{BrowserSession, ExampleLaunch, ExampleProxy};

#[derive(Debug)]
struct Arguments {
    url: String,
    proxy_file: String,
    headful: bool,
    mesa: bool,
    timeout: Duration,
    post_load_wait: Duration,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ResultReport {
    selected_proxy: String,
    proxy_count: usize,
    requested_url: String,
    success: bool,
    elapsed_ms: u128,
    final_url: Option<String>,
    status: Option<u64>,
    title: Option<String>,
    ready_state: Option<String>,
    user_agent: Option<String>,
    ua_ch_platform: Option<String>,
    content_bytes: Option<u64>,
    body_sha256: Option<String>,
    error: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let arguments = parse_arguments()?;
    let proxies = read_proxies(&arguments.proxy_file)?;
    let (proxy_index, proxy) = proxies
        .iter()
        .enumerate()
        .choose(&mut rand::rng())
        .context("proxy file contained no usable entries")?;
    let selected_proxy = format!("proxy-{}", proxy_index + 1);

    let started = Instant::now();
    let browser = timeout(
        arguments.timeout,
        BrowserSession::launch_with(
            PlatformProfile::Linux.profile(),
            ExampleLaunch {
                headful: arguments.headful,
                mesa: arguments.mesa,
                native: false,
                proxy: Some(proxy.clone()),
                user_data_dir: None,
            },
        ),
    )
    .await
    .context("browser launch timed out")??;
    let report = match timeout(arguments.timeout, browser.new_page(&arguments.url)).await {
        Ok(Ok(page)) => {
            if !arguments.post_load_wait.is_zero() {
                tokio::time::sleep(arguments.post_load_wait).await;
            }
            let snapshot: Value = timeout(
                arguments.timeout,
                page.inner().evaluate(
                    r#"
                    (async () => {
                        const navigation = performance.getEntriesByType('navigation')[0];
                        const bodyBytes = new TextEncoder().encode(
                            document.body?.innerText ?? ''
                        );
                        const digest = await crypto.subtle.digest('SHA-256', bodyBytes);
                        const bodySha256 = [...new Uint8Array(digest)]
                            .map(byte => byte.toString(16).padStart(2, '0')).join('');
                        return {
                            finalUrl: location.href,
                            status: Number.isFinite(navigation?.responseStatus)
                                ? navigation.responseStatus
                                : null,
                            title: document.title,
                            readyState: document.readyState,
                            userAgent: navigator.userAgent,
                            uaChPlatform: navigator.userAgentData?.platform ?? null,
                            contentBytes: new TextEncoder().encode(
                                document.documentElement.outerHTML
                            ).byteLength,
                            bodySha256
                        };
                    })()
                    "#,
                ),
            )
            .await
            .context("page inspection timed out")??
            .into_value()?;
            let status = snapshot["status"].as_u64();

            ResultReport {
                selected_proxy,
                proxy_count: proxies.len(),
                requested_url: arguments.url,
                success: status.is_some_and(|status| (200..400).contains(&status)),
                elapsed_ms: started.elapsed().as_millis(),
                final_url: string(&snapshot, "finalUrl"),
                status,
                title: string(&snapshot, "title"),
                ready_state: string(&snapshot, "readyState"),
                user_agent: string(&snapshot, "userAgent"),
                ua_ch_platform: string(&snapshot, "uaChPlatform"),
                content_bytes: snapshot["contentBytes"].as_u64(),
                body_sha256: string(&snapshot, "bodySha256"),
                error: None,
            }
        }
        Ok(Err(error)) => failure(
            selected_proxy,
            proxies.len(),
            arguments.url,
            started.elapsed().as_millis(),
            format!("navigation failed: {error:#}"),
        ),
        Err(_) => failure(
            selected_proxy,
            proxies.len(),
            arguments.url,
            started.elapsed().as_millis(),
            "navigation timed out".to_string(),
        ),
    };

    browser.close().await?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn read_proxies(path: &str) -> Result<Vec<ExampleProxy>> {
    let contents =
        std::fs::read_to_string(path).with_context(|| format!("failed to read {path}"))?;
    contents
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let line = line.trim();
            (!line.is_empty() && !line.starts_with('#')).then_some((index, line))
        })
        .map(|(index, line)| {
            ExampleProxy::parse(line)
                .with_context(|| format!("invalid proxy on line {} of {path}", index + 1))
        })
        .collect()
}

fn parse_arguments() -> Result<Arguments> {
    let mut url = None;
    let mut proxy_file = None;
    let mut headful = false;
    let mut mesa = false;
    let mut timeout_seconds = 45_u64;
    let mut wait_seconds = 0_u64;
    let mut arguments = std::env::args().skip(1);

    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--url" => url = Some(arguments.next().context("--url requires a value")?),
            "--proxy-file" => {
                proxy_file = Some(arguments.next().context("--proxy-file requires a path")?)
            }
            "--headful" => headful = true,
            "--mesa" => mesa = true,
            "--timeout" => {
                timeout_seconds = arguments
                    .next()
                    .context("--timeout requires seconds")?
                    .parse()
                    .context("--timeout must be an integer")?;
            }
            "--wait" => {
                wait_seconds = arguments
                    .next()
                    .context("--wait requires seconds")?
                    .parse()
                    .context("--wait must be an integer")?;
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            value => bail!("unknown argument: {value}; use --help"),
        }
    }

    let url = url.context("--url is required")?;
    let parsed = url::Url::parse(&url).context("invalid --url value")?;
    if !matches!(parsed.scheme(), "http" | "https") {
        bail!("probe URL must use http or https");
    }

    Ok(Arguments {
        url,
        proxy_file: proxy_file.context("--proxy-file is required")?,
        headful,
        mesa,
        timeout: Duration::from_secs(timeout_seconds),
        post_load_wait: Duration::from_secs(wait_seconds),
    })
}

fn failure(
    selected_proxy: String,
    proxy_count: usize,
    requested_url: String,
    elapsed_ms: u128,
    error: String,
) -> ResultReport {
    ResultReport {
        selected_proxy,
        proxy_count,
        requested_url,
        success: false,
        elapsed_ms,
        final_url: None,
        status: None,
        title: None,
        ready_state: None,
        user_agent: None,
        ua_ch_platform: None,
        content_bytes: None,
        body_sha256: None,
        error: Some(error),
    }
}

fn string(value: &Value, key: &str) -> Option<String> {
    value[key].as_str().map(str::to_string)
}

fn print_help() {
    println!(
        "random_proxy_probe --url <URL> --proxy-file <PATH> [OPTIONS]\n\
         \nOptions:\n\
           --url <URL>          HTTP(S) URL to open\n\
           --proxy-file <PATH>  Proxy URLs or host:port:user:pass lines\n\
           --headful            Launch headful Chromium\n\
           --mesa               Enable the Mesa/ANGLE launch path\n\
           --wait <SECONDS>     Wait after navigation before inspecting the page\n\
           --timeout <SECONDS>  Operation timeout (default: 45)\n\n\
         The randomly selected proxy is reported only by its line index."
    );
}
