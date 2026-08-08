//! Read-only page/worker environment audit.

use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use stealth_oxide::environment::{
    Finding, Observation, check_device_memory, compare_page_worker, compare_voice_language,
};
use stealth_oxide::redaction;
use stealth_oxide::{PlatformProfile, environment::FindingSeverity};

mod common;
use common::BrowserSession;

#[tokio::main]
async fn main() -> Result<()> {
    let mut url = "https://example.com/".to_string();
    let mut downlink_max = false;
    for argument in std::env::args().skip(1) {
        match argument.as_str() {
            "--downlink-max" => downlink_max = true,
            "--help" | "-h" => {
                println!("environment_audit [URL] [--downlink-max]");
                return Ok(());
            }
            value if value.starts_with('-') => bail!("unknown option: {value}"),
            value => url = value.to_string(),
        }
    }
    let parsed = url::Url::parse(&url).context("invalid URL")?;
    if !matches!(parsed.scheme(), "http" | "https") {
        bail!("audit URL must use http or https");
    }

    let browser = BrowserSession::launch_with(
        PlatformProfile::Linux.profile(),
        common::ExampleLaunch {
            network_information_downlink_max: downlink_max,
            ..common::ExampleLaunch::default()
        },
    )
    .await?;
    let page = browser.new_blank_page().await?;
    page.goto(&url).await?;
    let probe = tokio::time::timeout(Duration::from_secs(5), page.inner().evaluate(PROBE))
        .await
        .context("environment audit timed out")??
        .into_value::<Value>()?;

    let page_memory = number_observation(&probe["page"]["deviceMemory"]);
    let page_heap = number_observation(&probe["page"]["heapLimit"]);
    let worker_memory = number_observation(&probe["worker"]["deviceMemory"]);
    let mut findings = check_device_memory(&page_memory, &page_heap);
    if let Some(finding) = compare_page_worker(
        &page_memory,
        &worker_memory,
        "worker-device-memory-mismatch",
        "page and worker device-memory values disagree",
    ) {
        findings.push(finding);
    }
    if let Some(finding) = compare_voice_language(
        &string_observation(&probe["page"]["defaultVoiceLanguage"]),
        probe["page"]["language"].as_str().unwrap_or(""),
    ) {
        findings.push(finding);
    }

    let report = json!({
        "requestedUrl": redaction::url(&url),
        "readOnly": true,
        "networkInformationDownlinkMaxFlag": downlink_max,
        "observations": probe,
        "findings": findings.iter().map(finding_json).collect::<Vec<_>>(),
        "verdict": "evidence-only",
    });
    browser.close().await?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn number_observation(value: &Value) -> Observation<f64> {
    value
        .as_f64()
        .map_or(Observation::Unsupported, Observation::Observed)
}

fn string_observation(value: &Value) -> Observation<String> {
    value
        .as_str()
        .filter(|value| !value.is_empty())
        .map(|value| Observation::Observed(value.to_string()))
        .unwrap_or(Observation::Unavailable)
}

fn finding_json(finding: &Finding) -> Value {
    json!({
        "severity": match finding.severity {
            FindingSeverity::Info => "info",
            FindingSeverity::Warning => "warning",
            FindingSeverity::Contradiction => "contradiction",
            _ => "unknown",
        },
        "code": finding.code,
        "message": finding.message,
    })
}

const PROBE: &str = r#"
(async () => {
  const speech = typeof speechSynthesis === 'object' ? speechSynthesis : null;
  let voices = speech ? speech.getVoices() : [];
  if (speech && !voices.length) {
    voices = await new Promise(resolve => {
      const done = () => resolve(speech.getVoices());
      speech.addEventListener('voiceschanged', done, { once: true });
      setTimeout(done, 750);
    });
  }
  const worker = await new Promise(resolve => {
    const source = `postMessage({deviceMemory: navigator.deviceMemory ?? null, language: navigator.language, platform: navigator.platform})`;
    const handle = new Worker(URL.createObjectURL(new Blob([source], {type: 'text/javascript'})));
    handle.onmessage = event => { resolve(event.data); handle.terminate(); };
    setTimeout(() => { resolve({deviceMemory: null, language: null, platform: null}); handle.terminate(); }, 1000);
  });
  return {
    page: {
      deviceMemory: navigator.deviceMemory ?? null,
      hardwareConcurrency: navigator.hardwareConcurrency ?? null,
      heapLimit: performance.memory?.jsHeapSizeLimit ? performance.memory.jsHeapSizeLimit / 1073741824 : null,
      language: navigator.language ?? null,
      platform: navigator.platform ?? null,
      timezone: Intl.DateTimeFormat().resolvedOptions().timeZone ?? null,
      speechSupported: !!speech,
      voiceCount: voices.length,
      defaultVoiceLanguage: voices.find(voice => voice.default)?.lang ?? null
    },
    worker
  };
})()
"#;
