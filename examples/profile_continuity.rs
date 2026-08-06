use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use chromiumoxide::cdp::browser_protocol::network::{
    Cookie, DeleteCookiesParams, EnableParams, EventResponseReceived, GetCookiesParams,
};
use futures::StreamExt;
use rand::seq::IteratorRandom;
use serde::Serialize;
use serde_json::Value;
use stealth_oxide::PlatformProfile;

mod common;
use common::{BrowserSession, ExampleLaunch, ExampleProxy};

struct Arguments {
    url: String,
    wait: Duration,
    headful: bool,
    proxy: Option<ExampleProxy>,
    proxy_label: Option<String>,
    reopen: ReopenPolicy,
    mode: DiagnosticMode,
    clear_cookies: Vec<String>,
    network_check_url: Option<String>,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
enum DiagnosticMode {
    Preserve,
    ClearSelected,
    FreshProfile,
}

#[derive(Clone, Copy)]
enum ReopenPolicy {
    Never,
    Always,
    On403,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CookieRecord {
    name: String,
    domain: String,
    path: String,
    size: i64,
    session: bool,
    secure: bool,
    http_only: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct VisitReport {
    visit: u8,
    scrape_elapsed_seconds: f64,
    visit_elapsed_seconds: f64,
    network_path_stable: Option<bool>,
    status: Option<u64>,
    title: String,
    final_url: String,
    cookies_before: Vec<CookieRecord>,
    cookies_after: Vec<CookieRecord>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ContinuityReport {
    requested_url: String,
    browser_mode: &'static str,
    proxy: Option<String>,
    mode: DiagnosticMode,
    reopen_requested: bool,
    reopen_triggered: bool,
    same_profile_reopened: bool,
    network_path_matched_across_visits: Option<bool>,
    valid_run: Option<bool>,
    total_elapsed_seconds: f64,
    visits: Vec<VisitReport>,
}

struct VisitOutcome {
    report: VisitReport,
    network_identity_before: Option<String>,
    network_identity_after: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let arguments = parse_arguments()?;
    let total_started = Instant::now();
    let profile = tempfile::Builder::new()
        .prefix("stealth-oxide-continuity-")
        .tempdir()?;
    let mut outcomes = vec![run_visit(&arguments, profile.path(), 1).await?];
    let reopen_triggered = match arguments.reopen {
        ReopenPolicy::Never => false,
        ReopenPolicy::Always => true,
        ReopenPolicy::On403 => outcomes[0].report.status == Some(403),
    };
    if reopen_triggered {
        if matches!(arguments.mode, DiagnosticMode::ClearSelected) {
            clear_selected_cookies(&arguments, profile.path()).await?;
        }
        if matches!(arguments.mode, DiagnosticMode::FreshProfile) {
            let fresh_profile = tempfile::Builder::new()
                .prefix("stealth-oxide-fresh-")
                .tempdir()?;
            outcomes.push(run_visit(&arguments, fresh_profile.path(), 2).await?);
        } else {
            outcomes.push(run_visit(&arguments, profile.path(), 2).await?);
        }
    }
    let network_path_matched_across_visits = network_path_matches(&outcomes);
    let valid_run = arguments
        .network_check_url
        .as_ref()
        .map(|_| network_path_matched_across_visits == Some(true));
    let visits = outcomes.into_iter().map(|outcome| outcome.report).collect();

    let report = ContinuityReport {
        requested_url: arguments.url,
        browser_mode: if arguments.headful {
            "headful"
        } else {
            "headless"
        },
        proxy: arguments.proxy_label,
        mode: arguments.mode,
        reopen_requested: !matches!(arguments.reopen, ReopenPolicy::Never),
        reopen_triggered,
        same_profile_reopened: reopen_triggered
            && !matches!(arguments.mode, DiagnosticMode::FreshProfile),
        network_path_matched_across_visits,
        valid_run,
        total_elapsed_seconds: total_started.elapsed().as_secs_f64(),
        visits,
    };
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

async fn run_visit(arguments: &Arguments, profile: &Path, visit: u8) -> Result<VisitOutcome> {
    let visit_started = Instant::now();
    let browser = BrowserSession::launch_with(
        PlatformProfile::Linux.profile(),
        ExampleLaunch {
            headful: arguments.headful,
            proxy: arguments.proxy.clone(),
            user_data_dir: Some(profile.to_path_buf()),
            ..ExampleLaunch::default()
        },
    )
    .await?;
    let page = browser.new_blank_page().await?;
    page.inner().execute(EnableParams::default()).await?;
    let mut responses = page
        .inner()
        .event_listener::<EventResponseReceived>()
        .await?;
    let network_identity_before = if let Some(url) = &arguments.network_check_url {
        Some(read_network_identity(&page, url).await?)
    } else {
        None
    };
    let cookies_before = cookies_for(page.inner(), &arguments.url).await?;
    let scrape_started = Instant::now();
    let navigation_error = page.goto(&arguments.url).await.err();
    tokio::time::sleep(arguments.wait).await;
    let snapshot: Option<Value> = page
        .inner()
        .evaluate(
            r#"({
                finalUrl: location.href,
                status: performance.getEntriesByType('navigation')[0]?.responseStatus ?? null,
                title: document.title
            })"#,
        )
        .await
        .ok()
        .and_then(|result| result.into_value().ok());
    let mut network_status = None;
    while let Ok(Some(event)) =
        tokio::time::timeout(Duration::from_millis(10), responses.next()).await
    {
        if network_status.is_none()
            && format!("{:?}", event.r#type) == "Document"
            && same_navigation_url(&event.response.url, &arguments.url)
        {
            network_status = u64::try_from(event.response.status).ok();
        }
    }
    let status =
        network_status.or_else(|| snapshot.as_ref().and_then(|value| value["status"].as_u64()));
    if status.is_none()
        && let Some(error) = navigation_error
    {
        return Err(error.into());
    }
    let cookies_after = cookies_for(page.inner(), &arguments.url).await?;
    let scrape_elapsed_seconds = scrape_started.elapsed().as_secs_f64();
    let network_identity_after = if let Some(url) = &arguments.network_check_url {
        Some(read_network_identity(&page, url).await?)
    } else {
        None
    };
    let network_path_stable = network_identity_before
        .as_ref()
        .zip(network_identity_after.as_ref())
        .map(|(before, after)| before == after);
    browser.close().await?;

    Ok(VisitOutcome {
        report: VisitReport {
            visit,
            scrape_elapsed_seconds,
            visit_elapsed_seconds: visit_started.elapsed().as_secs_f64(),
            network_path_stable,
            status,
            title: snapshot
                .as_ref()
                .and_then(|value| value["title"].as_str())
                .unwrap_or_default()
                .to_string(),
            final_url: sanitize_url(
                snapshot
                    .as_ref()
                    .and_then(|value| value["finalUrl"].as_str())
                    .unwrap_or(&arguments.url),
            ),
            cookies_before,
            cookies_after,
        },
        network_identity_before,
        network_identity_after,
    })
}

async fn read_network_identity(page: &common::ExamplePage, url: &str) -> Result<String> {
    page.goto(url).await?;
    let value: Value = page
        .inner()
        .evaluate("document.body?.innerText?.trim() ?? ''")
        .await?
        .into_value()?;
    value
        .as_str()
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .context("network check returned an empty identity")
}

fn network_path_matches(outcomes: &[VisitOutcome]) -> Option<bool> {
    let identities = outcomes
        .iter()
        .flat_map(|outcome| {
            [
                outcome.network_identity_before.as_ref(),
                outcome.network_identity_after.as_ref(),
            ]
            .into_iter()
            .flatten()
        })
        .collect::<Vec<_>>();
    let first = identities.first()?;
    Some(identities.iter().all(|identity| *identity == *first))
}

async fn clear_selected_cookies(arguments: &Arguments, profile: &Path) -> Result<()> {
    if arguments.clear_cookies.is_empty() {
        bail!("clear-selected mode requires at least one --clear-cookie");
    }
    let browser = BrowserSession::launch_with(
        PlatformProfile::Linux.profile(),
        ExampleLaunch {
            headful: arguments.headful,
            proxy: arguments.proxy.clone(),
            user_data_dir: Some(profile.to_path_buf()),
            ..ExampleLaunch::default()
        },
    )
    .await?;
    let page = browser.new_blank_page().await?;
    for name in &arguments.clear_cookies {
        let params = DeleteCookiesParams::builder()
            .name(name)
            .url(&arguments.url)
            .build()
            .map_err(anyhow::Error::msg)?;
        page.inner().execute(params).await?;
    }
    browser.close().await
}

async fn cookies_for(page: &chromiumoxide::Page, url: &str) -> Result<Vec<CookieRecord>> {
    let cookies = page
        .execute(GetCookiesParams::builder().url(url).build())
        .await?
        .result
        .cookies;
    Ok(cookies.into_iter().map(CookieRecord::from).collect())
}

impl From<Cookie> for CookieRecord {
    fn from(cookie: Cookie) -> Self {
        Self {
            name: cookie.name,
            domain: cookie.domain,
            path: cookie.path,
            size: cookie.size,
            session: cookie.session,
            secure: cookie.secure,
            http_only: cookie.http_only,
        }
    }
}

fn sanitize_url(value: &str) -> String {
    let Ok(mut url) = url::Url::parse(value) else {
        return "<non-url>".to_string();
    };
    url.set_query(None);
    url.set_fragment(None);
    url.to_string()
}

fn same_navigation_url(observed: &str, requested: &str) -> bool {
    let (Ok(observed), Ok(requested)) = (url::Url::parse(observed), url::Url::parse(requested))
    else {
        return false;
    };
    observed.scheme() == requested.scheme()
        && observed.host_str() == requested.host_str()
        && observed.port_or_known_default() == requested.port_or_known_default()
        && observed.path() == requested.path()
}

fn parse_arguments() -> Result<Arguments> {
    let mut url = None;
    let mut wait = 5;
    let mut headful = true;
    let mut proxy_file = None;
    let mut reopen = ReopenPolicy::Never;
    let mut mode = DiagnosticMode::Preserve;
    let mut clear_cookies = Vec::new();
    let mut network_check_url = None;
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
            "--headless" => headful = false,
            "--proxy-file" => {
                proxy_file = Some(arguments.next().context("--proxy-file requires a path")?)
            }
            "--reopen" => reopen = ReopenPolicy::Always,
            "--reopen-on-403" => reopen = ReopenPolicy::On403,
            "--mode" => {
                mode = match arguments
                    .next()
                    .context("--mode requires a value")?
                    .as_str()
                {
                    "preserve" => DiagnosticMode::Preserve,
                    "clear-selected" => DiagnosticMode::ClearSelected,
                    "fresh-profile" => DiagnosticMode::FreshProfile,
                    value => bail!(
                        "unknown mode {value}; use preserve, clear-selected, or fresh-profile"
                    ),
                }
            }
            "--clear-cookie" => clear_cookies.push(
                arguments
                    .next()
                    .context("--clear-cookie requires a cookie name")?,
            ),
            "--network-check-url" => {
                network_check_url = Some(
                    arguments
                        .next()
                        .context("--network-check-url requires a value")?,
                )
            }
            "--help" | "-h" => {
                println!(
                    "profile_continuity --url <URL> [--wait <SECONDS>] [--headless] \
                     [--proxy-file <PATH>] [--reopen | --reopen-on-403] \
                     [--mode preserve|clear-selected|fresh-profile] \
                     [--clear-cookie <NAME> ...] [--network-check-url <URL>]"
                );
                std::process::exit(0);
            }
            value => bail!("unknown argument: {value}; use --help"),
        }
    }
    let url = url.context("--url is required")?;
    let parsed = url::Url::parse(&url).context("invalid --url value")?;
    if !matches!(parsed.scheme(), "http" | "https") {
        bail!("continuity URL must use http or https");
    }
    if let Some(value) = &network_check_url {
        let parsed = url::Url::parse(value).context("invalid --network-check-url value")?;
        if !matches!(parsed.scheme(), "http" | "https") {
            bail!("network check URL must use http or https");
        }
    }
    let (proxy, proxy_label) = if let Some(path) = proxy_file {
        let proxies = read_proxies(&path)?;
        let (index, proxy) = proxies
            .into_iter()
            .enumerate()
            .choose(&mut rand::rng())
            .context("proxy file contained no usable entries")?;
        (Some(proxy), Some(format!("proxy-{}", index + 1)))
    } else {
        (None, None)
    };
    Ok(Arguments {
        url,
        wait: Duration::from_secs(wait),
        headful,
        proxy,
        proxy_label,
        reopen,
        mode,
        clear_cookies,
        network_check_url,
    })
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

#[cfg(test)]
mod tests {
    use super::same_navigation_url;

    #[test]
    fn matches_the_requested_document_without_query_noise() {
        assert!(same_navigation_url(
            "https://example.com/path?challenge=1",
            "https://example.com/path"
        ));
        assert!(!same_navigation_url(
            "https://captcha.example.com/path",
            "https://example.com/path"
        ));
        assert!(!same_navigation_url(
            "https://example.com/challenge",
            "https://example.com/path"
        ));
    }
}
