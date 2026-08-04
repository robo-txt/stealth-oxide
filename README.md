# stealth-oxide

`stealth-oxide` is a Rust library built on `chromiumoxide`. It applies coherent browser profiles through typed Chrome DevTools Protocol commands before the first site navigation.

The crate provides Linux, Windows, and macOS Chrome presets, configurable CDP patch groups, launch options, and validation for common cross-surface contradictions. It does not inject JavaScript into page APIs.

## Installation

Until the crate is published to crates.io, depend on the Git repository:

```toml
[dependencies]
stealth-oxide = { git = "https://github.com/robo-txt/stealth-oxide.git" }
tokio = { version = "1", features = ["full"] }
anyhow = "1"
```

Chromium or Chrome must be installed on the host. Container support is provided by the included `Dockerfile` and `compose.yaml`.

## Quick start

Use the Linux preset when Chromium runs in the included Linux container:

```rust
use anyhow::Result;
use stealth_oxide::{PlatformProfile, StealthBrowser, StealthConfig};

#[tokio::main]
async fn main() -> Result<()> {
    let config = StealthConfig::builder()
        .platform(PlatformProfile::Linux)
        .headful(true)
        .mesa(true)
        .build()?;

    let browser = StealthBrowser::launch_with(config).await?;
    let page = browser.new_page("https://example.com").await?;

    println!("{}", page.inner().content().await?);
    browser.close().await?;
    Ok(())
}
```

Identity-affecting CDP commands are applied to `about:blank` before `new_page` navigates to the requested URL.

## Platform presets

```rust
use stealth_oxide::PlatformProfile;

let linux = PlatformProfile::Linux.profile();
let windows = PlatformProfile::Windows.profile();
let macos = PlatformProfile::MacOS.profile();
```

A preset describes a browser identity; it does not change the operating system underneath Chromium. Use the Linux preset in Linux containers. Windows and macOS presets can still expose native Linux values in workers, fonts, graphics, or platform-only browser features when run on Linux.

## Customize a profile

Start with a coherent preset and override only the values needed by the application:

```rust
use anyhow::Result;
use stealth_oxide::{BrowserProfileBuilder, PlatformProfile};

fn profile() -> Result<stealth_oxide::BrowserProfile> {
    BrowserProfileBuilder::new(PlatformProfile::Linux.profile())
        .name("linux-ca")
        .locale("en-CA")
        .timezone("America/Toronto")
        .screen(2560, 1440)
        .available_screen(2560, 1400)
        .device_scale_factor(1.25)
        .color_scheme("dark")
        .reduced_motion("no-preference")
        .touch(false, 0)
        .build()
}
```

The builder keeps the first navigator language synchronized when `locale` changes. Validation rejects invalid screen dimensions, non-positive scale factors, contradictory platform identity, mismatched locale/language values, and impossible touch settings.

Pass a custom profile into the browser configuration:

```rust
let config = StealthConfig::builder()
    .profile(profile()?)
    .headful(true)
    .mesa(true)
    .build()?;
```

## Select CDP patch groups

All supported patch groups are enabled by default. They can be selected explicitly:

```rust
use stealth_oxide::{PatchSet, PlatformProfile, StealthConfig};

let patches = PatchSet::all()
    .identity(true)
    .locale_and_timezone(true)
    .screen(true)
    .device_environment(false);

let config = StealthConfig::builder()
    .platform(PlatformProfile::Linux)
    .patches(patches)
    .build()?;
# Ok::<(), anyhow::Error>(())
```

The `identity` group intentionally keeps the user agent, `Accept-Language`, navigator platform, and UA Client Hints together. Disabling individual values inside that group would make it easy to produce contradictory browser surfaces.

Use `PatchSet::none()` when the caller wants an unmodified page and then opt into groups individually.

## Launch options

```rust
use stealth_oxide::LaunchOptions;

let launch = LaunchOptions::default()
    .headful(true)
    .mesa(true)
    .speech_dispatcher(false);
```

- `headful` launches a normal browser window. A display server is required in a container.
- `mesa` selects Chromium's ANGLE-over-OpenGL path and enables GPU rasterization. The included container supplies Xvfb and Mesa.
- `speech_dispatcher` enables Chromium's native Linux Speech Dispatcher integration. The service must already be installed and running. It is disabled by default because an eSpeak catalog does not match Windows or macOS profiles.

Use the options independently or pass them together:

```rust
let config = StealthConfig::builder()
    .platform(PlatformProfile::Linux)
    .launch_options(launch)
    .build()?;
```

## Backward-compatible API

Existing callers can continue passing a profile directly:

```rust
use stealth_oxide::profiles::chrome_linux::chrome_linux;

let browser = StealthBrowser::launch(chrome_linux()).await?;
```

This compatibility path reads the existing `STEALTH_OXIDE_HEADFUL`, `STEALTH_OXIDE_USE_MESA`, and `STEALTH_OXIDE_SPEECH_DISPATCHER` environment variables. New library integrations should prefer `StealthConfig` and `launch_with` because typed options are local to a browser instance instead of process-wide.

## Testing

### Probe URLs through proxies

The `url_probe` example opens real Chromium pages and reports the requested URL, final URL, HTTP navigation status, title, load state, elapsed time, user agent, UA Client Hints platform, rendered DOM size, and a SHA-256 hash of visible body text as JSON. `contentBytes` is the UTF-8 byte size of the serialized DOM after page scripts run, not the compressed network-transfer size. `bodySha256` supports comparisons without printing potentially sensitive response content.

Probe more than one URL directly:

```bash
cargo run --example url_probe -- \
  --url https://example.com \
  --url https://httpbin.org/headers \
  --profile linux
```

Probe every URL through each supplied proxy:

```bash
cargo run --example url_probe -- \
  --url https://example.com \
  --url https://httpbin.org/ip \
  --proxy http://127.0.0.1:8080 \
  --proxy socks5://127.0.0.1:1080 \
  --profile linux \
  --timeout 60
```

Proxy files containing `host:port:username:password` on each line can be passed without placing credentials in the shell command:

```bash
cargo run --example url_probe -- \
  --url https://httpbin.org/ip \
  --proxy-file /path/to/proxies.txt \
  --profile linux \
  --timeout 60
```

Supplied proxies are identified as `proxy-1`, `proxy-2`, and so on in JSON output. Proxy addresses and credentials are not printed.

To select one random entry from a proxy file and open one URL:

```bash
cargo run --example random_proxy_probe -- \
  --url https://example.com \
  --proxy-file /path/to/proxies.txt \
  --headful \
  --mesa \
  --wait 30
```

The optional `--wait` value keeps the loaded page open for the given number of
seconds before collecting the result. The result identifies the selection only
as `proxy-N`, reports how many entries were available, and never prints the
selected address or credentials.

Chromiumoxide handles HTTP proxy authentication through CDP when credentials are included:

```bash
cargo run --example url_probe -- \
  --url https://example.com \
  --proxy http://username:password@proxy.example:8080
```

Credentials are removed from Chromium's `--proxy-server` argument and are not included in the JSON proxy label. Avoid putting secrets directly on a shared shell command line; construct `ProxyConfig::new(...).credentials(...)` in application code when credential exposure through process history is a concern.

```rust
use stealth_oxide::{PlatformProfile, ProxyConfig, StealthConfig};

let proxy = ProxyConfig::new("http://proxy.example:8080")?
    .credentials("username", "password");

let config = StealthConfig::builder()
    .platform(PlatformProfile::Linux)
    .proxy(proxy)
    .build()?;
# Ok::<(), anyhow::Error>(())
```

In the included desktop container, add `--headful --mesa`:

```bash
docker compose run --rm stealth-oxide \
  cargo run --example url_probe -- \
  --url https://example.com \
  --profile linux \
  --headful \
  --mesa
```

This probe confirms browser connectivity and reports observable navigation results. It does not prove that a third-party fraud or bot-management system classifies a session as human. Use it only with URLs and proxies you are authorized to test.

### Test suite

Run fast profile and parameter tests:

```bash
cargo test
```

Build and run the desktop container tests:

```bash
docker compose build
docker compose run --rm stealth-oxide
```

Real-browser integration tests are ignored by default because they require a working Chromium process and display environment. Run an individual test with `--ignored --nocapture` to inspect its browser-visible output.

## Scope and limitations

CDP overrides are target- and session-scoped. A value observed in a page is not automatically guaranteed to match dedicated workers, shared workers, service workers, out-of-process frames, or native platform services.

The project validates known relationships and includes real-browser probes, but no configuration guarantees a particular classification by a third-party fraud or bot-management service. Only test services and properties you own or are authorized to assess.
