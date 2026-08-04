use std::time::Duration;

use anyhow::{Context, Result};
use serde_json::Value;
use tokio::time::timeout;

mod common;
use common::TestBrowser as StealthBrowser;
use stealth_oxide::profiles::chrome_windows::chrome_windows;

#[tokio::test]
#[ignore = "requires the desktop container and STEALTH_OXIDE_SPEECH_DISPATCHER=1"]
async fn native_fonts_and_speech_are_browser_visible() -> Result<()> {
    assert!(
        matches!(
            std::env::var("STEALTH_OXIDE_SPEECH_DISPATCHER").as_deref(),
            Ok("1") | Ok("true")
        ),
        "set STEALTH_OXIDE_SPEECH_DISPATCHER=1 for this native experiment"
    );
    let browser = timeout(
        Duration::from_secs(20),
        StealthBrowser::launch(chrome_windows()),
    )
    .await
    .context("timed out while launching Chromium")??;
    let page = timeout(
        Duration::from_secs(20),
        browser.new_page("data:text/html,<title>font and speech probe</title>"),
    )
    .await
    .context("timed out while opening the font and speech probe")??;

    let observed: Value = timeout(
        Duration::from_secs(20),
        page.inner().evaluate(
            r#"
            (async () => {
                const loadFont = family => ({
                    family,
                    available: ['monospace', 'serif', 'sans-serif'].some(fallback => {
                        const probe = document.createElement('span');
                        probe.textContent = 'mmmmmmmmmmWWWWWWWWWWil1😀';
                        Object.assign(probe.style, {
                            position: 'absolute',
                            visibility: 'hidden',
                            whiteSpace: 'nowrap',
                            fontSize: '72px'
                        });
                        probe.style.fontFamily = fallback;
                        document.body.append(probe);
                        const fallbackWidth = probe.getBoundingClientRect().width;
                        probe.style.fontFamily = `"${family}", ${fallback}`;
                        const candidateWidth = probe.getBoundingClientRect().width;
                        probe.remove();
                        return candidateWidth !== fallbackWidth;
                    })
                });
                const readVoices = () => speechSynthesis.getVoices().map(voice => ({
                    name: voice.name,
                    lang: voice.lang,
                    localService: voice.localService,
                    default: voice.default,
                    voiceURI: voice.voiceURI
                }));
                let voices = readVoices();
                if (!voices.length) {
                    voices = await new Promise(resolve => {
                        const deadline = Date.now() + 8000;
                        const poll = () => {
                            const current = readVoices();
                            if (current.length || Date.now() >= deadline) {
                                resolve(current);
                            } else {
                                setTimeout(poll, 100);
                            }
                        };
                        speechSynthesis.addEventListener('voiceschanged', poll, { once: true });
                        poll();
                    });
                }
                return {
                    fonts: [
                        'Segoe UI', 'Arial', 'Times New Roman', 'Consolas',
                        'Segoe UI Emoji', 'DejaVu Sans', 'Liberation Mono',
                        'Noto Color Emoji'
                    ].map(loadFont),
                    voices
                };
            })()
            "#,
        ),
    )
    .await
    .context("timed out while reading font and speech surfaces")??
    .into_value()?;

    timeout(Duration::from_secs(20), browser.close())
        .await
        .context("timed out while closing Chromium")??;

    let voices = observed["voices"]
        .as_array()
        .context("voices was not an array")?;
    println!("observed fonts: {:#}", observed["fonts"]);
    println!(
        "observed {} speech voices; first five: {:#?}",
        voices.len(),
        &voices[..voices.len().min(5)]
    );
    assert!(!voices.is_empty(), "Speech Dispatcher exposed no voices");
    assert!(
        voices
            .iter()
            .any(|voice| voice["lang"].as_str().unwrap_or_default().starts_with("en")),
        "Speech Dispatcher exposed no English voice"
    );

    if matches!(
        std::env::var("STEALTH_OXIDE_FONT_ISOLATION").as_deref(),
        Ok("1") | Ok("true")
    ) {
        let fonts = observed["fonts"]
            .as_array()
            .context("fonts was not an array")?;
        let loaded = |family: &str| {
            fonts.iter().any(|font| {
                font["family"] == family && font["available"].as_bool().unwrap_or(false)
            })
        };
        assert!(loaded("Segoe UI"));
        assert!(loaded("Arial"));
        assert!(loaded("Times New Roman"));
        assert!(!loaded("DejaVu Sans"));
        assert!(!loaded("Liberation Mono"));
        assert!(!loaded("Noto Color Emoji"));
    }

    Ok(())
}
