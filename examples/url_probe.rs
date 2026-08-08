use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use serde::Serialize;
use serde_json::Value;
use stealth_oxide::PlatformProfile;
use tokio::time::timeout;

mod common;
use common::{BrowserSession, ExampleLaunch, ExampleProxy};

struct Arguments {
    urls: Vec<String>,
    proxies: Vec<ExampleProxy>,
    platform: PlatformProfile,
    headful: bool,
    mesa: bool,
    timeout: Duration,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProbeResult {
    proxy: String,
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
    let proxies: Vec<Option<ExampleProxy>> = if arguments.proxies.is_empty() {
        vec![None]
    } else {
        arguments.proxies.into_iter().map(Some).collect()
    };
    let mut results = Vec::new();

    for (proxy_index, proxy) in proxies.into_iter().enumerate() {
        let proxy_label = proxy.as_ref().map_or_else(
            || "direct".to_string(),
            |_| format!("proxy-{}", proxy_index + 1),
        );
        let browser = match timeout(
            arguments.timeout,
            BrowserSession::launch_with(
                arguments.platform.profile(),
                ExampleLaunch {
                    headful: arguments.headful,
                    mesa: arguments.mesa,
                    native: false,
                    proxy,
                    user_data_dir: None,
                    network_information_downlink_max: false,
                },
            ),
        )
        .await
        {
            Ok(Ok(browser)) => browser,
            Ok(Err(error)) => {
                for url in &arguments.urls {
                    results.push(failed(
                        &proxy_label,
                        url,
                        0,
                        format!("browser launch failed: {error:#}"),
                    ));
                }
                continue;
            }
            Err(_) => {
                for url in &arguments.urls {
                    results.push(failed(
                        &proxy_label,
                        url,
                        0,
                        "browser launch timed out".to_string(),
                    ));
                }
                continue;
            }
        };

        for url in &arguments.urls {
            results.push(probe_url(&browser, &proxy_label, url, arguments.timeout).await);
        }
        if let Err(error) = browser.close().await {
            eprintln!("failed to close browser for {proxy_label}: {error:#}");
        }
    }

    println!("{}", serde_json::to_string_pretty(&results)?);
    Ok(())
}

async fn probe_url(
    browser: &BrowserSession,
    proxy: &str,
    url: &str,
    deadline: Duration,
) -> ProbeResult {
    let started = Instant::now();
    let page = match timeout(deadline, browser.new_page(url)).await {
        Ok(Ok(page)) => page,
        Ok(Err(error)) => {
            return failed(
                proxy,
                url,
                started.elapsed().as_millis(),
                format!("navigation failed: {error:#}"),
            );
        }
        Err(_) => {
            return failed(
                proxy,
                url,
                started.elapsed().as_millis(),
                "navigation timed out".to_string(),
            );
        }
    };

    let snapshot = timeout(
        deadline,
        page.inner().evaluate(
            r#"
            (async () => {
                const navigation = performance.getEntriesByType('navigation')[0];
                const bodyBytes = new TextEncoder().encode(document.body?.innerText ?? '');
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
    .await;

    match snapshot {
        Ok(Ok(value)) => match value.into_value::<Value>() {
            Ok(value) => ProbeResult {
                proxy: proxy.to_string(),
                requested_url: url.to_string(),
                success: value["status"]
                    .as_u64()
                    .is_some_and(|status| (200..400).contains(&status)),
                elapsed_ms: started.elapsed().as_millis(),
                final_url: string(&value, "finalUrl"),
                status: value["status"].as_u64(),
                title: string(&value, "title"),
                ready_state: string(&value, "readyState"),
                user_agent: string(&value, "userAgent"),
                ua_ch_platform: string(&value, "uaChPlatform"),
                content_bytes: value["contentBytes"].as_u64(),
                body_sha256: string(&value, "bodySha256"),
                error: None,
            },
            Err(error) => failed(
                proxy,
                url,
                started.elapsed().as_millis(),
                format!("could not decode page snapshot: {error:#}"),
            ),
        },
        Ok(Err(error)) => failed(
            proxy,
            url,
            started.elapsed().as_millis(),
            format!("page inspection failed: {error:#}"),
        ),
        Err(_) => failed(
            proxy,
            url,
            started.elapsed().as_millis(),
            "page inspection timed out".to_string(),
        ),
    }
}

fn failed(proxy: &str, url: &str, elapsed_ms: u128, error: String) -> ProbeResult {
    ProbeResult {
        proxy: proxy.to_string(),
        requested_url: url.to_string(),
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

fn parse_arguments() -> Result<Arguments> {
    let mut urls = Vec::new();
    let mut proxies = Vec::new();
    let mut platform = PlatformProfile::Linux;
    let mut headful = false;
    let mut mesa = false;
    let mut timeout_seconds = 45_u64;
    let mut arguments = std::env::args().skip(1);

    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--url" => urls.push(arguments.next().context("--url requires a value")?),
            "--proxy" => proxies.push(ExampleProxy::parse(
                &arguments.next().context("--proxy requires a value")?,
            )?),
            "--proxy-file" => {
                let path = arguments.next().context("--proxy-file requires a path")?;
                let contents = std::fs::read_to_string(&path)
                    .with_context(|| format!("failed to read proxy file: {path}"))?;
                for (index, line) in contents.lines().enumerate() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    proxies.push(ExampleProxy::parse(line).with_context(|| {
                        format!("invalid proxy on line {} of {path}", index + 1)
                    })?);
                }
            }
            "--profile" => {
                platform = match arguments
                    .next()
                    .context("--profile requires linux, windows, or macos")?
                    .as_str()
                {
                    "linux" => PlatformProfile::Linux,
                    "windows" => PlatformProfile::Windows,
                    "macos" => PlatformProfile::MacOS,
                    value => bail!("unknown profile: {value}"),
                }
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
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            value => bail!("unknown argument: {value}; use --help"),
        }
    }

    if urls.is_empty() {
        bail!("provide at least one --url");
    }
    for url in &urls {
        let parsed = url::Url::parse(url).context("invalid --url value")?;
        if !matches!(parsed.scheme(), "http" | "https") {
            bail!("probe URLs must use http or https");
        }
    }

    Ok(Arguments {
        urls,
        proxies,
        platform,
        headful,
        mesa,
        timeout: Duration::from_secs(timeout_seconds),
    })
}

fn print_help() {
    println!(
        "url_probe --url <URL> [--url <URL> ...] [--proxy <PROXY> ...]\n\
         \nOptions:\n\
           --url <URL>            HTTP(S) URL to probe; repeatable\n\
           --proxy <PROXY>        http(s)/socks proxy URL; repeatable\n\
           --proxy-file <PATH>    Read proxy URLs or host:port:user:pass lines\n\
           --profile <PLATFORM>   linux, windows, or macos (default: linux)\n\
           --headful              Launch headful Chromium\n\
           --mesa                 Enable the Mesa/ANGLE launch path\n\
           --timeout <SECONDS>    Per-operation timeout (default: 45)\n\n\
         Authenticated HTTP proxies may use http://user:pass@host:port.\n\
         Results are emitted as a JSON array. Only test authorized URLs."
    );
}
