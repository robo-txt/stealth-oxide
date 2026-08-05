use std::collections::BTreeMap;

use chromiumoxide::Page;
use chromiumoxide::cdp::browser_protocol::network::{CookieParam, SetCookiesParams};
use chromiumoxide::cdp::browser_protocol::page::AddScriptToEvaluateOnNewDocumentParams;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{Error, Result};

/// Cookie state installed through CDP before navigation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CookieSeed {
    /// Cookie name.
    pub name: String,
    /// Cookie value.
    pub value: String,
    /// Full HTTP or HTTPS URL defining the cookie scope.
    pub url: String,
    /// Cookie path.
    #[serde(default = "default_path")]
    pub path: String,
    /// Whether the cookie requires HTTPS.
    #[serde(default)]
    pub secure: bool,
    /// Whether page JavaScript is prevented from reading the cookie.
    #[serde(default)]
    pub http_only: bool,
}

impl CookieSeed {
    /// Creates a cookie seed scoped to a full URL.
    pub fn new(name: impl Into<String>, value: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            url: url.into(),
            path: default_path(),
            secure: false,
            http_only: false,
        }
    }

    /// Sets the cookie path.
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }

    /// Controls the secure-cookie attribute.
    pub const fn secure(mut self, secure: bool) -> Self {
        self.secure = secure;
        self
    }

    /// Controls the HTTP-only cookie attribute.
    pub const fn http_only(mut self, http_only: bool) -> Self {
        self.http_only = http_only;
        self
    }
}

/// IndexedDB record created when its origin's document context starts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IndexedDbSeed {
    /// Database name.
    pub database: String,
    /// Object-store name.
    pub store: String,
    /// JSON-compatible record key.
    pub key: Value,
    /// JSON-compatible record value.
    pub value: Value,
}

impl IndexedDbSeed {
    /// Creates one IndexedDB record seed.
    pub fn new(
        database: impl Into<String>,
        store: impl Into<String>,
        key: Value,
        value: Value,
    ) -> Self {
        Self {
            database: database.into(),
            store: store.into(),
            key,
            value,
        }
    }
}

/// Storage state scoped to one exact HTTP or HTTPS origin.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OriginSeed {
    /// Exact origin containing only scheme and authority.
    pub origin: String,
    /// Local-storage keys and values.
    #[serde(default)]
    pub local_storage: BTreeMap<String, String>,
    /// IndexedDB records.
    #[serde(default)]
    pub indexed_db: Vec<IndexedDbSeed>,
}

impl OriginSeed {
    /// Creates an empty storage seed for one origin.
    pub fn new(origin: impl Into<String>) -> Self {
        Self {
            origin: origin.into(),
            local_storage: BTreeMap::new(),
            indexed_db: Vec::new(),
        }
    }

    /// Adds or replaces a local-storage entry.
    pub fn local_storage(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.local_storage.insert(key.into(), value.into());
        self
    }

    /// Adds an IndexedDB record.
    pub fn indexed_db(mut self, record: IndexedDbSeed) -> Self {
        self.indexed_db.push(record);
        self
    }
}

/// One mergeable collection of browser state used by authorized tests.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileSeed {
    /// Cookies installed before navigation.
    #[serde(default)]
    pub cookies: Vec<CookieSeed>,
    /// Origin-scoped local-storage and IndexedDB state.
    #[serde(default)]
    pub origins: Vec<OriginSeed>,
}

impl ProfileSeed {
    /// Creates an empty seed document.
    pub const fn new() -> Self {
        Self {
            cookies: Vec::new(),
            origins: Vec::new(),
        }
    }

    /// Decodes and validates one JSON seed document.
    pub fn from_json(json: &str) -> Result<Self> {
        let seed: Self = serde_json::from_str(json)?;
        seed.validate()?;
        Ok(seed)
    }

    /// Adds a cookie seed.
    pub fn cookie(mut self, cookie: CookieSeed) -> Self {
        self.cookies.push(cookie);
        self
    }

    /// Adds origin-scoped storage.
    pub fn origin(mut self, origin: OriginSeed) -> Self {
        self.origins.push(origin);
        self
    }

    /// Validates URLs, origins, names, and storage identifiers.
    pub fn validate(&self) -> Result<()> {
        for cookie in &self.cookies {
            if cookie.name.trim().is_empty() {
                return Err(Error::invalid_seed("cookie names cannot be empty"));
            }
            validate_http_url(&cookie.url, "cookie URL")?;
        }
        for origin in &self.origins {
            let parsed = validate_http_url(&origin.origin, "storage origin")?;
            if parsed.origin().ascii_serialization() != origin.origin.trim_end_matches('/') {
                return Err(Error::invalid_seed(
                    "storage origin must contain only scheme and authority",
                ));
            }
            for record in &origin.indexed_db {
                if record.database.trim().is_empty() || record.store.trim().is_empty() {
                    return Err(Error::invalid_seed(
                        "IndexedDB database and store names cannot be empty",
                    ));
                }
            }
        }
        Ok(())
    }

    /// Creates three harmless example seeds scoped to the requested URL.
    ///
    /// The returned documents contain one HTTP-only cookie, local-storage
    /// preferences, and one IndexedDB record. Applications should normally
    /// supply their own authorized test state instead.
    pub fn defaults_for(target_url: &str) -> Result<Vec<Self>> {
        let parsed = validate_http_url(target_url, "target URL")?;
        let origin = parsed.origin().ascii_serialization();
        let cookie_url = format!("{origin}/");

        Ok(vec![
            Self::new().cookie(
                CookieSeed::new(
                    "stealth_oxide_test_session",
                    "returning-test-browser",
                    cookie_url,
                )
                .secure(parsed.scheme() == "https")
                .http_only(true),
            ),
            Self::new().origin(
                OriginSeed::new(origin.clone())
                    .local_storage("stealth_oxide_theme", "dark")
                    .local_storage("stealth_oxide_onboarding", "complete"),
            ),
            Self::new().origin(OriginSeed::new(origin).indexed_db(IndexedDbSeed::new(
                "stealth_oxide_test",
                "state",
                json!("last-session"),
                json!({ "completed": true, "source": "generated-test-profile" }),
            ))),
        ])
    }
}

/// Counts of seed operations registered with Chromium.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeedReport {
    /// Number of seed documents processed.
    pub documents: usize,
    /// Number of cookies installed.
    pub cookies: usize,
    /// Number of origin records registered.
    pub origins: usize,
    /// Number of local-storage entries registered.
    pub local_storage_entries: usize,
    /// Number of IndexedDB entries registered.
    pub indexed_db_entries: usize,
}

/// Validates and installs multiple profile seeds before navigation.
///
/// Cookies are installed immediately through CDP. Origin storage is registered
/// through `Page.addScriptToEvaluateOnNewDocument` and writes only when a
/// matching origin's execution context is created.
pub async fn apply_profile_seeds(page: &Page, seeds: &[ProfileSeed]) -> Result<SeedReport> {
    for seed in seeds {
        seed.validate()?;
    }

    let cookies = seeds
        .iter()
        .flat_map(|seed| &seed.cookies)
        .map(cookie_param)
        .collect::<Result<Vec<_>>>()?;
    if !cookies.is_empty() {
        page.execute(SetCookiesParams::new(cookies))
            .await
            .map_err(|error| Error::cdp("profile seeding", error))?;
    }

    let origins = seeds
        .iter()
        .flat_map(|seed| &seed.origins)
        .collect::<Vec<_>>();
    if !origins.is_empty() {
        let serialized = serde_json::to_string(&origins)?;
        let source = storage_script(&serialized);
        page.execute(AddScriptToEvaluateOnNewDocumentParams::new(source))
            .await
            .map_err(|error| Error::cdp("profile seeding", error))?;
    }

    Ok(SeedReport {
        documents: seeds.len(),
        cookies: seeds.iter().map(|seed| seed.cookies.len()).sum(),
        origins: origins.len(),
        local_storage_entries: origins
            .iter()
            .map(|origin| origin.local_storage.len())
            .sum(),
        indexed_db_entries: origins.iter().map(|origin| origin.indexed_db.len()).sum(),
    })
}

fn cookie_param(seed: &CookieSeed) -> Result<CookieParam> {
    CookieParam::builder()
        .name(seed.name.clone())
        .value(seed.value.clone())
        .url(seed.url.clone())
        .path(seed.path.clone())
        .secure(seed.secure)
        .http_only(seed.http_only)
        .build()
        .map_err(Error::invalid_seed)
}

fn validate_http_url(value: &str, field: &str) -> Result<url::Url> {
    let parsed = url::Url::parse(value)
        .map_err(|error| Error::invalid_seed(format!("invalid {field}: {error}")))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(Error::invalid_seed(format!(
            "{field} must use http or https"
        )));
    }
    Ok(parsed)
}

fn storage_script(serialized: &str) -> String {
    format!(
        r#"
        (() => {{
            const seeds = {serialized};
            for (const seed of seeds) {{
                if (location.origin !== seed.origin) continue;
                for (const [key, value] of Object.entries(seed.localStorage)) {{
                    localStorage.setItem(key, value);
                }}
                for (const entry of seed.indexedDb) {{
                    const request = indexedDB.open(entry.database, 1);
                    request.onupgradeneeded = () => {{
                        if (!request.result.objectStoreNames.contains(entry.store)) {{
                            request.result.createObjectStore(entry.store);
                        }}
                    }};
                    request.onsuccess = () => {{
                        const transaction = request.result.transaction(entry.store, 'readwrite');
                        transaction.objectStore(entry.store).put(entry.value, entry.key);
                        transaction.oncomplete = () => request.result.close();
                    }};
                }}
            }}
        }})();
        "#
    )
}

fn default_path() -> String {
    "/".to_string()
}
