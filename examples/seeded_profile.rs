use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use chromiumoxide::{Browser, BrowserConfig};
use futures::StreamExt;
use serde::Serialize;
use serde_json::Value;
use stealth_oxide::{SeedReport, StealthConfig, apply_profile_seeds};
use tokio::time::sleep;

#[path = "common/profile_seed.rs"]
mod profile_seed;
use profile_seed::{GeneratedProfile, load_seed_documents};

#[derive(Debug)]
struct Arguments {
    url: String,
    seed_files: Vec<PathBuf>,
    no_seeds: bool,
    keep_profile: bool,
    wait: Duration,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Report {
    requested_url: String,
    final_url: String,
    profile_directory: String,
    profile_kept: bool,
    seeds: SeedReport,
    applied_patches: usize,
    browser_cookie_names: Vec<String>,
    visible_storage: Value,
    creepjs_ratings: Option<Value>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let arguments = parse_arguments()?;
    let seeds = if arguments.no_seeds {
        Vec::new()
    } else {
        load_seed_documents(&arguments.seed_files, &arguments.url)?
    };
    let mut generated_profile = GeneratedProfile::create(arguments.keep_profile)?;

    let config = BrowserConfig::builder()
        .user_data_dir(generated_profile.path())
        .with_head()
        .build()
        .map_err(anyhow::Error::msg)?;
    let (mut browser, mut handler) = Browser::launch(config).await?;
    tokio::spawn(async move {
        while let Some(event) = handler.next().await {
            if let Err(error) = event {
                eprintln!("chromiumoxide handler error: {error:?}");
            }
        }
    });

    let page = browser.new_page("about:blank").await?;
    let applied = StealthConfig::recommended().apply(&page).await?;
    let seed_report = apply_profile_seeds(&page, &seeds).await?;
    page.goto(&arguments.url).await?;
    if !arguments.wait.is_zero() {
        sleep(arguments.wait).await;
    }

    let snapshot: Value = page
        .evaluate(
            r#"
            (async () => {
                const databases = indexedDB.databases
                    ? await indexedDB.databases()
                    : [];
                const rating = selector => {
                    const text = document.querySelector(selector)?.textContent || '';
                    const match = text.match(/(\d+)%/);
                    return match ? Number(match[1]) : null;
                };
                const creepjs = {
                    headless: rating('.headless-rating'),
                    likeHeadless: rating('.like-headless-rating'),
                    stealth: rating('.stealth-rating')
                };
                return {
                    visibleStorage: {
                        cookieNames: document.cookie.split(';')
                            .map(value => value.trim().split('=')[0])
                            .filter(Boolean),
                        localStorage: Object.fromEntries(Object.entries(localStorage)),
                        indexedDbNames: databases.map(database => database.name).filter(Boolean)
                    },
                    creepjsRatings: Object.values(creepjs).some(value => value !== null)
                        ? creepjs
                        : null
                };
            })()
            "#,
        )
        .await?
        .into_value()?;

    let browser_cookie_names = browser
        .get_cookies()
        .await?
        .into_iter()
        .map(|cookie| cookie.name)
        .collect();
    let report = Report {
        requested_url: arguments.url,
        final_url: page.url().await?.unwrap_or_default(),
        profile_directory: generated_profile.path().display().to_string(),
        profile_kept: arguments.keep_profile,
        seeds: seed_report,
        applied_patches: applied.applied().len(),
        browser_cookie_names,
        visible_storage: snapshot["visibleStorage"].clone(),
        creepjs_ratings: snapshot["creepjsRatings"]
            .as_object()
            .map(|_| snapshot["creepjsRatings"].clone()),
    };

    browser.close().await?;
    generated_profile.cleanup().await?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn parse_arguments() -> Result<Arguments> {
    let mut url = "https://example.com".to_string();
    let mut seed_files = Vec::new();
    let mut no_seeds = false;
    let mut keep_profile = false;
    let mut wait_seconds = 3_u64;
    let mut arguments = std::env::args().skip(1);

    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--url" => url = arguments.next().context("--url requires a value")?,
            "--seed" => seed_files.push(PathBuf::from(
                arguments.next().context("--seed requires a path")?,
            )),
            "--no-seeds" => no_seeds = true,
            "--keep-profile" => keep_profile = true,
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

    let parsed = url::Url::parse(&url).context("invalid --url value")?;
    if !matches!(parsed.scheme(), "http" | "https") {
        bail!("URL must use http or https");
    }
    if no_seeds && !seed_files.is_empty() {
        bail!("--no-seeds cannot be combined with --seed");
    }

    Ok(Arguments {
        url,
        seed_files,
        no_seeds,
        keep_profile,
        wait: Duration::from_secs(wait_seconds),
    })
}

fn print_help() {
    println!(
        "seeded_profile [OPTIONS]\n\n\
         Options:\n\
           --url <URL>       Target URL (default: https://example.com)\n\
           --seed <PATH>     Merge a JSON seed file; repeatable\n\
           --no-seeds        Generate a clean profile for comparison\n\
           --wait <SECONDS>  Wait after navigation (default: 3)\n\
           --keep-profile    Keep the generated user-data directory\n\
           -h, --help        Print help"
    );
}
