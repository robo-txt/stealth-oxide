use anyhow::Result;
use chromiumoxide::auth::Credentials;
use chromiumoxide::cdp::browser_protocol::system_info::{GetInfoParams, GetInfoReturns};
use chromiumoxide::{Browser, BrowserConfig};
use futures::StreamExt;

use crate::config::{LaunchOptions, StealthConfig};
use crate::page::StealthPage;
use crate::profiles::BrowserProfile;

pub struct StealthBrowser {
    browser: Browser,
    profile: BrowserProfile,
    patches: crate::config::PatchSet,
    proxy_credentials: Option<Credentials>,
}

impl StealthBrowser {
    pub async fn launch(profile: BrowserProfile) -> Result<Self> {
        let config = StealthConfig {
            profile,
            launch: LaunchOptions::from_legacy_environment(),
            patches: crate::config::PatchSet::default(),
        };
        Self::launch_with(config).await
    }

    pub async fn launch_with(config: StealthConfig) -> Result<Self> {
        config.validate()?;
        let StealthConfig {
            profile,
            launch,
            patches,
        } = config;
        // Apply launch-scoped identity inputs before any page or worker target
        // exists. Page-scoped CDP user-agent overrides do not reach workers.
        let mut config_builder = BrowserConfig::builder().hide();
        if patches.identity {
            config_builder =
                config_builder.arg(("user-agent", profile.navigator.user_agent.as_str()));
        }

        if launch.speech_dispatcher {
            config_builder = config_builder.arg("enable-speech-dispatcher");
        }

        if launch.headful {
            config_builder = config_builder.with_head();
        }

        if launch.mesa {
            config_builder = config_builder
                .arg(("use-gl", "angle"))
                .arg(("use-angle", "gl"))
                .arg("ignore-gpu-blocklist")
                .arg("enable-gpu-rasterization");
        }

        if let Some(proxy) = &launch.proxy {
            config_builder = config_builder.arg(("proxy-server", proxy.server.as_str()));
        }

        let config = config_builder.build().map_err(anyhow::Error::msg)?;

        let (browser, mut handler) = Browser::launch(config).await?;

        tokio::spawn(async move {
            while let Some(event) = handler.next().await {
                if let Err(err) = event {
                    eprintln!("chromiumoxide handler error: {err:?}");
                }
            }
        });

        let proxy_credentials = launch.proxy.and_then(|proxy| {
            proxy
                .username
                .zip(proxy.password)
                .map(|(username, password)| Credentials { username, password })
        });

        Ok(Self {
            browser,
            profile,
            patches,
            proxy_credentials,
        })
    }

    pub async fn new_page(&self, url: &str) -> Result<StealthPage> {
        let page = self.browser.new_page("about:blank").await?;

        if let Some(credentials) = &self.proxy_credentials {
            page.authenticate(credentials.clone()).await?;
        }

        let stealth_page = StealthPage::new(page);
        stealth_page
            .apply_profile_with(&self.profile, &self.patches)
            .await?;
        stealth_page.goto(url).await?;

        //patches apply here
        Ok(stealth_page)
    }

    pub fn profile(&self) -> &BrowserProfile {
        &self.profile
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
