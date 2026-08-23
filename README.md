# stealth-oxide

Typed Chromium profiles and Chrome DevTools Protocol configuration for Rust.

`stealth-oxide` gives applications explicit, configurable control over the
browser-visible values that Chromium exposes to a page. It works with
[`chromiumoxide`](https://crates.io/crates/chromiumoxide) and applies CDP
configuration to an existing page before navigation.

Use it for authorized browser automation, compatibility testing, diagnostics,
and reproducible QA environments.

## What you can do

- Start from coherent Linux, macOS, or Windows Chrome profiles.
- Apply identity, locale, timezone, screen, media-feature, and touch settings.
- Override only the surfaces your application needs while preserving native
  Chromium values everywhere else.
- Build and validate custom browser profiles with typed values.
- Detect contradictions before sending CDP commands.
- Coordinate configuration across new pages, iframes, popups, and workers.
- Observe and redact transmitted network identity without intercepting requests.
- Validate User-Agent, language, and Client Hint consistency at the network
  boundary.
- Add validated request headers with the optional `interceptor` feature.
- Make bounded, application-controlled decisions for retryable top-level
  navigations.
- Seed reproducible cookies, local storage, and IndexedDB state with the
  optional `seeding` feature.

The application remains in control of Chromium launch, browser lifetime,
navigation, proxies, authentication, storage, and shutdown.

## Installation

```toml
[dependencies]
chromiumoxide = "0.9.1"
stealth-oxide = "0.1.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

The crate currently models Chrome `151.0.7922.138` and requires Rust 1.86 or
newer.

Optional capabilities are feature-gated:

```toml
[dependencies.stealth-oxide]
version = "0.1.1"
features = ["interceptor", "seeding"]
```

## Quick start

Create a page, apply a profile while it is still blank, and then navigate:

```rust,no_run
use anyhow::Result;
use chromiumoxide::{Browser, BrowserConfig};
use futures::StreamExt;
use stealth_oxide::{ChromeLanguageConfig, PlatformProfile, StealthConfig};

#[tokio::main]
async fn main() -> Result<()> {
    let profile = PlatformProfile::Linux.profile();
    let language = ChromeLanguageConfig::from_profile(&profile);
    let browser_config = BrowserConfig::builder()
        .hide()
        .arg(language.chrome_argument())
        .build()
        .map_err(anyhow::Error::msg)?;

    let (mut browser, mut handler) = Browser::launch(browser_config).await?;
    tokio::spawn(async move {
        while let Some(event) = handler.next().await {
            if let Err(error) = event {
                eprintln!("chromiumoxide handler error: {error:?}");
            }
        }
    });

    let page = browser.new_page("about:blank").await?;
    let report = StealthConfig::from_profile(profile).apply(&page).await?;
    page.goto("https://example.com").await?;

    println!("applied patches: {:?}", report.applied());
    browser.close().await?;
    Ok(())
}
```

Apply the configuration before navigation so the first page scripts observe
the selected values.

## Profiles and patches

Built-in profiles are selected explicitly:

```rust
use stealth_oxide::{PlatformProfile, StealthConfig};

let linux = StealthConfig::for_platform(PlatformProfile::Linux);
let macos = StealthConfig::for_platform(PlatformProfile::MacOS);
let windows = StealthConfig::for_platform(PlatformProfile::Windows);
```

Each profile supplies typed values for:

| Area | Examples |
| --- | --- |
| Identity | User-Agent, navigator platform, languages, User-Agent Client Hints |
| Locale | `Intl` locale and language configuration |
| Timezone | IANA timezone |
| Screen | Dimensions, available area, scale factor, orientation |
| Media features | Color scheme, reduced motion, forced colors, gamut, monochrome |
| Touch | Touch availability and maximum touch points |

Patches can be enabled, disabled, or kept native independently:

```rust
use stealth_oxide::{Patch, PlatformProfile, StealthConfig};

let config = StealthConfig::for_platform(PlatformProfile::Windows)
    .disable(Patch::Screen)
    .use_native(Patch::Touch)
    .timezone("America/Toronto")
    .languages(["en-CA", "en"])
    .locale("en-CA");
```

For an explicit opt-in configuration, start with no patches:

```rust
use stealth_oxide::{Patch, StealthConfig};

let config = StealthConfig::none()
    .enable(Patch::Identity)
    .enable(Patch::Locale)
    .enable(Patch::Timezone);
```

## Custom profiles and validation

`BrowserProfileBuilder` lets applications create a reusable profile with the
same typed fields as the built-in presets. `StealthConfig` validates known
contradictions by default before issuing CDP commands.

```rust
use stealth_oxide::{ConsistencyPolicy, PlatformProfile, StealthConfig};

let config = StealthConfig::for_platform(PlatformProfile::Linux)
    .navigator_platform("Win32")
    .consistency_policy(ConsistencyPolicy::Warn);

let issues = config.validation_issues();
assert!(!issues.is_empty());
```

Choose the validation policy that matches the workflow:

- `Strict` rejects known contradictions before applying patches.
- `Warn` applies valid values and returns consistency issues in the report.
- `Permissive` applies requested values subject to CDP and Chromium rules.

The returned `ApplyReport` records applied, native, skipped, and warning
states, making configuration behavior observable to the caller.

## New-target coordination

Configuration is applied to a specific CDP target. Use `TargetCoordinator`
when a page can create workers, popups, related pages, or other targets that
need the same configuration:

```rust,no_run
use futures::StreamExt;
use stealth_oxide::{PlatformProfile, StealthConfig, TargetCoordinator};

# async fn configure(page: chromiumoxide::Page) -> Result<(), Box<dyn std::error::Error>> {
let config = StealthConfig::for_platform(PlatformProfile::Linux);
config.apply(&page).await?;

let coordinator = TargetCoordinator::new(&config)?;
let mut events = coordinator.enable(&page).await?;
let target_page = page.clone();

tokio::spawn(async move {
    while let Some(event) = events.next().await {
        if let Err(error) = coordinator.apply(&target_page, &event).await {
            eprintln!("target configuration failed: {error}");
        }
    }
});

page.goto("https://example.com").await?;
# Ok(())
# }
```

The application must keep driving the event stream. This makes target
coverage explicit instead of hiding a runtime task inside the library.

## Network observation

`NetworkAudit` is a passive, bounded observer for Chromium network events. It
can summarize requests, redirects, failures, cache behavior, and transmitted
identity without pausing or rewriting ordinary requests:

```rust,no_run
use chromiumoxide::Page;
use stealth_oxide::NetworkAuditHandle;

# async fn audit(page: &Page) -> Result<(), Box<dyn std::error::Error>> {
let audit = NetworkAuditHandle::attach(page).await?;
page.goto("https://example.com").await?;
let report = audit.stop().await;

println!("observed requests: {}", report.summary().requests);
# Ok(())
# }
```

Reports are redacted and bounded. They omit request bodies, credentials,
cookies, remote addresses, URL query strings, and fragments. The audit can
also validate transmitted User-Agent, language, and Client Hint values against
the selected profile.

For applications that need explicit request-header additions, enable the
optional `interceptor` feature and use its validated `HeaderPolicy` builder.

## Controlled navigation retries

`NavigationRetryPolicy` makes a decision for a completed top-level navigation;
it does not perform the retry itself:

```rust
use std::time::{Duration, SystemTime};
use stealth_oxide::{NavigationMethod, NavigationRetryPolicy, RetryDecision};

let policy = NavigationRetryPolicy::new()
    .max_attempts(3)
    .max_elapsed(Duration::from_secs(120));

let decision = policy.decide(
    429,
    NavigationMethod::Get,
    Some("15"),
    1,
    Duration::from_secs(2),
    SystemTime::now(),
    0.5,
);

assert_eq!(decision, RetryDecision::RetryAfter(Duration::from_secs(15)));
```

Applications decide whether and how to act on `RetryDecision`, preserving
ownership of browser state, cookies, proxies, and navigation lifecycle.

## Reproducible browser state

Enable the `seeding` feature to install test cookies, local storage, and
IndexedDB records through typed seed definitions:

```toml
[dependencies]
stealth-oxide = { version = "0.1.1", features = ["seeding"] }
```

See [`examples/profile_seeding.rs`](examples/profile_seeding.rs) for a complete
example.

## Examples and tests

Runnable examples are included in the package:

```bash
cargo run --example example
cargo run --example custom_configuration
cargo run --example custom_profile
cargo run --features seeding --example profile_seeding
```

Run the deterministic test suite with:

```bash
cargo test --all-targets --all-features
```

Browser diagnostics are ignored by default because they require a local
Chromium process. See [`tests/README.md`](tests/README.md) for trigger commands.

## Scope

`stealth-oxide` configures Chromium through CDP; it does not replace
Chromiumoxide or manage the browser lifecycle. The selected profile describes
browser-visible values, while the host operating system still supplies native
fonts, graphics, voices, and other platform resources.

Use the library only for systems and destinations you are authorized to test.

## License

MIT
