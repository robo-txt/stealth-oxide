#![allow(dead_code)]

use anyhow::Result;
use chromiumoxide::cdp::browser_protocol::system_info::{GetInfoParams, GetInfoReturns};
use chromiumoxide::{Browser, BrowserConfig, Page};
use futures::StreamExt;
use stealth_oxide::{BrowserProfile, Patch, StealthConfig};

pub struct TestBrowser {
    browser: Browser,
    stealth: StealthConfig,
}

impl TestBrowser {
    pub async fn launch(profile: BrowserProfile) -> Result<Self> {
        let mut builder = BrowserConfig::builder()
            .hide()
            .arg(("user-agent", profile.navigator().user_agent.as_str()));

        if env_enabled("STEALTH_OXIDE_HEADFUL") {
            builder = builder.with_head();
        }
        if env_enabled("STEALTH_OXIDE_USE_MESA") {
            builder = builder
                .arg(("use-gl", "angle"))
                .arg(("use-angle", "gl"))
                .arg("ignore-gpu-blocklist")
                .arg("enable-gpu-rasterization");
        }
        if env_enabled("STEALTH_OXIDE_DOWNLINK_MAX") {
            builder = builder.arg("enable-network-information-downlink-max");
        }

        let config = builder.build().map_err(anyhow::Error::msg)?;
        let (browser, mut handler) = Browser::launch(config).await?;
        tokio::spawn(async move {
            while let Some(event) = handler.next().await {
                if let Err(error) = event {
                    eprintln!("chromiumoxide handler error: {error:?}");
                }
            }
        });

        let stealth = if env_enabled("STEALTH_OXIDE_USE_NATIVE_SCREEN") {
            StealthConfig::from_profile(profile).use_native(Patch::Screen)
        } else {
            StealthConfig::from_profile(profile)
        };

        Ok(Self { browser, stealth })
    }

    pub async fn new_page(&self, url: &str) -> Result<TestPage> {
        let page = self.browser.new_page("about:blank").await?;
        self.stealth.apply(&page).await?;
        page.goto(url).await?;
        Ok(TestPage(page))
    }

    pub async fn version(&self) -> Result<String> {
        Ok(self.browser.version().await?.product)
    }

    pub async fn system_info(&self) -> Result<GetInfoReturns> {
        Ok(self.browser.execute(GetInfoParams {}).await?.result)
    }

    pub async fn close(mut self) -> Result<()> {
        self.browser.close().await?;
        Ok(())
    }
}

pub struct TestPage(Page);

impl TestPage {
    pub fn inner(&self) -> &Page {
        &self.0
    }

    pub async fn goto(&self, url: &str) -> chromiumoxide::error::Result<()> {
        self.0.goto(url).await?;
        Ok(())
    }
}

fn env_enabled(name: &str) -> bool {
    matches!(std::env::var(name).as_deref(), Ok("1") | Ok("true"))
}
