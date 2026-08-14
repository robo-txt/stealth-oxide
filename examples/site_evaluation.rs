use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use chromiumoxide::{Browser, BrowserConfig};
use futures::StreamExt;
use serde_json::{Value, json};
use stealth_oxide::{
    ChromeLanguageConfig, CompatibilityStatus, PlatformProfile, StealthConfig,
    compare_browser_versions,
};

#[derive(Debug)]
struct Arguments {
    url: String,
    expected_text: Vec<String>,
    minimum_html_bytes: usize,
    wait: Duration,
}

#[tokio::main]
async fn main() -> Result<()> {
    let arguments = parse_arguments()?;
    let profile = PlatformProfile::Linux.profile();
    let language = ChromeLanguageConfig::from_profile(&profile);
    let browser_config = BrowserConfig::builder()
        .hide()
        .arg(language.chrome_argument())
        .build()
        .map_err(anyhow::Error::msg)?;
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
    StealthConfig::from_profile(profile).apply(&page).await?;
    page.goto(&arguments.url).await?;
    tokio::time::sleep(arguments.wait).await;

    let html = page.content().await?;
    let title = page.get_title().await?.unwrap_or_default();
    let final_url = page.url().await?.unwrap_or_default();
    let observation: Value = page
        .evaluate(
            r#"(() => {
                const navigation = performance.getEntriesByType('navigation')[0];
                const body = document.body?.innerText || '';
                return {
                    body,
                    responseStatus: navigation?.responseStatus ?? null,
                    challengeElements: document.querySelectorAll(
                        'iframe[src*="captcha" i], #challenge-form, .cf-challenge, [data-sitekey]'
                    ).length
                };
            })()"#,
        )
        .await?
        .into_value()?;

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
    let challenge_detected = challenge_in_title
        || challenge_on_error_page
        || challenge_url
        || (challenge_element_count > 0 && !expected_text_found);
    let http_ok = status.is_some_and(|status| (200..300).contains(&status));
    let success = http_ok
        && expected_text_found
        && html.len() >= arguments.minimum_html_bytes
        && !challenge_detected;

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
    let mut minimum_html_bytes = 100;
    let mut wait = Duration::from_secs(3);

    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--url" => url = Some(arguments.next().context("--url requires a value")?),
            "--expect" => {
                expected_text.push(arguments.next().context("--expect requires a value")?)
            }
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
            other => bail!("unknown argument: {other}"),
        }
    }

    let url = url.context(
        "usage: site_evaluation --url URL [--expect TEXT] [--min-html-bytes N] [--wait SECONDS]",
    )?;
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        bail!("--url must use http:// or https://");
    }

    Ok(Arguments {
        url,
        expected_text,
        minimum_html_bytes,
        wait,
    })
}
