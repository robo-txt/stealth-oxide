#![allow(dead_code)]

use anyhow::{Result, bail};
use chromiumoxide::auth::Credentials;
use chromiumoxide::cdp::browser_protocol::system_info::{GetInfoParams, GetInfoReturns};
use chromiumoxide::{Browser, BrowserConfig, Page};
use futures::StreamExt;
use stealth_oxide::{BrowserProfile, Patch, StealthConfig};

#[derive(Clone)]
pub struct ExampleProxy {
    server: String,
    username: Option<String>,
    password: Option<String>,
}

impl ExampleProxy {
    pub fn parse(value: &str) -> Result<Self> {
        if !value.contains("://") {
            let fields = value.split(':').collect::<Vec<_>>();
            if fields.len() != 4 || fields.iter().any(|field| field.is_empty()) {
                bail!("scheme-less proxies must use host:port:username:password");
            }
            return Ok(Self {
                server: format!("http://{}:{}", fields[0], fields[1]),
                username: Some(fields[2].to_string()),
                password: Some(fields[3].to_string()),
            });
        }

        let mut parsed = url::Url::parse(value)?;
        if !matches!(parsed.scheme(), "http" | "https" | "socks4" | "socks5") {
            bail!("proxy scheme must be http, https, socks4, or socks5");
        }
        if parsed.host().is_none() || parsed.port().is_none() {
            bail!("proxy URL must include a host and port");
        }
        if parsed.path() != "/" || parsed.query().is_some() || parsed.fragment().is_some() {
            bail!("proxy URL cannot contain a path, query, or fragment");
        }

        let username = (!parsed.username().is_empty()).then(|| parsed.username().to_string());
        let password = parsed.password().map(str::to_string);
        parsed
            .set_username("")
            .map_err(|_| anyhow::anyhow!("failed to remove proxy username"))?;
        parsed
            .set_password(None)
            .map_err(|_| anyhow::anyhow!("failed to remove proxy password"))?;

        if username.is_some() != password.is_some() {
            bail!("proxy username and password must be provided together");
        }

        Ok(Self {
            server: parsed.as_str().trim_end_matches('/').to_string(),
            username,
            password,
        })
    }

    fn credentials(&self) -> Option<Credentials> {
        self.username
            .clone()
            .zip(self.password.clone())
            .map(|(username, password)| Credentials { username, password })
    }
}

#[derive(Default)]
pub struct ExampleLaunch {
    pub headful: bool,
    pub mesa: bool,
    pub native: bool,
    pub proxy: Option<ExampleProxy>,
    pub user_data_dir: Option<std::path::PathBuf>,
}

pub struct BrowserSession {
    browser: Browser,
    stealth: Option<StealthConfig>,
    proxy_credentials: Option<Credentials>,
    _temporary_user_data_dir: Option<tempfile::TempDir>,
}

impl BrowserSession {
    pub async fn launch(profile: BrowserProfile) -> Result<Self> {
        Self::launch_with(profile, ExampleLaunch::default()).await
    }

    pub async fn launch_with(profile: BrowserProfile, launch: ExampleLaunch) -> Result<Self> {
        let temporary_user_data_dir = if launch.user_data_dir.is_some() {
            None
        } else {
            Some(
                tempfile::Builder::new()
                    .prefix("stealth-oxide-")
                    .tempdir()?,
            )
        };
        let user_data_dir = launch.user_data_dir.as_deref().unwrap_or_else(|| {
            temporary_user_data_dir
                .as_ref()
                .expect("temporary profile must exist")
                .path()
        });
        let mut builder = BrowserConfig::builder().hide().user_data_dir(user_data_dir);
        if !launch.native {
            builder = builder.arg(("user-agent", profile.navigator().user_agent.as_str()));
        }
        if launch.headful {
            builder = builder.with_head();
        }
        if launch.mesa {
            builder = builder
                .arg(("use-gl", "angle"))
                .arg(("use-angle", "gl"))
                .arg("ignore-gpu-blocklist")
                .arg("enable-gpu-rasterization");
        }
        if let Some(proxy) = &launch.proxy {
            builder = builder.arg(("proxy-server", proxy.server.as_str()));
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

        let stealth = if launch.native {
            None
        } else if env_enabled("STEALTH_OXIDE_USE_NATIVE_SCREEN") {
            Some(StealthConfig::from_profile(profile).use_native(Patch::Screen))
        } else {
            Some(StealthConfig::from_profile(profile))
        };
        let proxy_credentials = launch.proxy.and_then(|proxy| proxy.credentials());

        Ok(Self {
            browser,
            stealth,
            proxy_credentials,
            _temporary_user_data_dir: temporary_user_data_dir,
        })
    }

    pub async fn new_page(&self, url: &str) -> Result<ExamplePage> {
        let page = self.new_blank_page().await?;
        page.goto(url).await?;
        Ok(page)
    }

    pub async fn new_blank_page(&self) -> Result<ExamplePage> {
        let page = self.browser.new_page("about:blank").await?;
        if let Some(credentials) = &self.proxy_credentials {
            page.authenticate(credentials.clone()).await?;
        }
        if let Some(stealth) = &self.stealth {
            stealth.apply(&page).await?;
        }
        Ok(ExamplePage(page))
    }

    pub async fn version(&self) -> Result<String> {
        Ok(self.browser.version().await?.product)
    }

    pub async fn system_info(&self) -> Result<GetInfoReturns> {
        Ok(self.browser.execute(GetInfoParams {}).await?.result)
    }

    pub async fn close(mut self) -> Result<()> {
        self.browser.close().await?;
        self.browser.wait().await?;
        Ok(())
    }
}

pub struct ExamplePage(Page);

impl ExamplePage {
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
