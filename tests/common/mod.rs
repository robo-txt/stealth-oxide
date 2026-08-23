#![allow(dead_code)]

use anyhow::Result;
use chromiumoxide::cdp::browser_protocol::network::{EventResponseReceived, ResourceType};
use chromiumoxide::cdp::browser_protocol::page::GetFrameTreeParams;
use chromiumoxide::cdp::browser_protocol::system_info::{GetInfoParams, GetInfoReturns};
use chromiumoxide::{Browser, BrowserConfig, Page};
use futures::StreamExt;
use stealth_oxide::{BrowserProfile, Patch, StealthConfig};
use tokio::time::{Duration, timeout};

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
        self.new_page_with_stealth(url, &self.stealth).await
    }

    pub async fn new_page_with_stealth(
        &self,
        url: &str,
        stealth: &StealthConfig,
    ) -> Result<TestPage> {
        let page = self.browser.new_page("about:blank").await?;
        stealth.apply(&page).await?;
        let mut responses = page.event_listener::<EventResponseReceived>().await?;
        let main_frame_id = page
            .execute(GetFrameTreeParams {})
            .await?
            .result
            .frame_tree
            .frame
            .id;
        page.goto(url).await?;
        let mut document_responses = Vec::new();
        loop {
            match timeout(Duration::from_millis(250), responses.next()).await {
                Ok(Some(event))
                    if event.r#type == ResourceType::Document
                        && event.frame_id.as_ref() == Some(&main_frame_id) =>
                {
                    document_responses.push(event);
                }
                Ok(Some(_)) => {}
                Ok(None) | Err(_) => break,
            }
        }
        let status = document_responses.last().map(|event| event.response.status);
        let final_url = document_responses
            .last()
            .map(|event| event.response.url.clone());
        let redirect_statuses = document_responses
            .iter()
            .rev()
            .skip(1)
            .map(|event| event.response.status)
            .collect::<Vec<_>>();
        Ok(TestPage {
            page,
            navigation: NavigationInfo {
                status,
                final_url,
                redirect_statuses,
            },
        })
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

pub struct TestPage {
    page: Page,
    navigation: NavigationInfo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigationInfo {
    pub status: Option<i64>,
    pub final_url: Option<String>,
    pub redirect_statuses: Vec<i64>,
}

impl TestPage {
    pub fn inner(&self) -> &Page {
        &self.page
    }

    pub fn navigation(&self) -> &NavigationInfo {
        &self.navigation
    }

    pub async fn goto(&self, url: &str) -> chromiumoxide::error::Result<()> {
        self.page.goto(url).await?;
        Ok(())
    }
}

fn env_enabled(name: &str) -> bool {
    matches!(std::env::var(name).as_deref(), Ok("1") | Ok("true"))
}
