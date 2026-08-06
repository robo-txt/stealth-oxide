use anyhow::{Result, bail};
use serde_json::{Value, json};
use stealth_oxide::PlatformProfile;

mod common;
use common::{BrowserSession, ExampleLaunch};

const ENDPOINT: &str = "https://tls.peet.ws/api/all";

#[tokio::main]
async fn main() -> Result<()> {
    let headful = parse_arguments()?;
    let browser = BrowserSession::launch_with(
        PlatformProfile::Linux.profile(),
        ExampleLaunch {
            headful,
            ..ExampleLaunch::default()
        },
    )
    .await?;
    let page = browser.new_page(ENDPOINT).await?;
    let body: String = page
        .inner()
        .evaluate("document.body.innerText")
        .await?
        .into_value()?;
    let payload: Value = serde_json::from_str(&body)?;
    let report = json!({
        "browserMode": if headful { "headful" } else { "headless" },
        "httpVersion": payload["http_version"],
        "userAgent": payload["user_agent"],
        "tls": {
            "ja3": payload["tls"]["ja3"],
            "ja3Hash": payload["tls"]["ja3_hash"],
            "ja4": payload["tls"]["ja4"],
            "ja4R": payload["tls"]["ja4_r"],
            "peetprint": payload["tls"]["peetprint"],
            "peetprintHash": payload["tls"]["peetprint_hash"],
        },
        "http2": {
            "akamaiFingerprint": payload["http2"]["akamai_fingerprint"],
            "akamaiFingerprintHash": payload["http2"]["akamai_fingerprint_hash"],
            "sentFrames": payload["http2"]["sent_frames"],
        }
    });

    browser.close().await?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn parse_arguments() -> Result<bool> {
    let mut headful = false;
    for argument in std::env::args().skip(1) {
        match argument.as_str() {
            "--headful" => headful = true,
            "--help" | "-h" => {
                println!("transport_diagnostic [--headful]");
                std::process::exit(0);
            }
            value => bail!("unknown argument: {value}; use --help"),
        }
    }
    Ok(headful)
}
