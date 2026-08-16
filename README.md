<p align="center">
  <img src="https://raw.githubusercontent.com/robo-txt/stealth-oxide/main/docs/images/stealth-oxide-logo.png" alt="stealth-oxide logo" width="180">
</p>

<h1 align="center">stealth-oxide</h1>

<p align="center">
  Typed, configurable Chromium profiles and CDP emulation for Rust.
</p>

Typed, configurable browser profiles for
[`chromiumoxide`](https://crates.io/crates/chromiumoxide), applied through the
Chrome DevTools Protocol before site scripts run.

`stealth-oxide` gives Rust applications Playwright-Stealth-style control without
taking over the browser lifecycle:

- Start with a coherent Linux, macOS, or Windows desktop profile.
- Apply every patch from the selected preset or enable patches individually.
- Preserve native Chromium values where emulation would be less accurate.
- Validate locale, timezone, identity, screen, media, and touch consistency
  before sending CDP commands.
- Keep browser launch, proxies, authentication, navigation, and shutdown in
  application code.

The public configuration API separates browser identity, native-value
preservation, network observation, request-header policy, and navigation retry
decisions. Nothing retries, intercepts, or changes a session implicitly.

## Browser-test container captures

The project container applies its coherent Windows desktop profile before page
scripts run. Both examples wait for the report content and document height to
stabilize before capturing the complete page. Click either preview for the
full-resolution PNG.

<table>
  <tr>
    <th>CreepJS</th>
    <th>Sannysoft</th>
  </tr>
  <tr>
    <td>
      <a href="https://github.com/robo-txt/stealth-oxide/blob/main/docs/images/creepjs-full-page.png">
        <img src="https://raw.githubusercontent.com/robo-txt/stealth-oxide/main/docs/images/creepjs-full-page.png" alt="Full-page CreepJS report captured with stealth-oxide" width="420">
      </a>
    </td>
    <td>
      <a href="https://github.com/robo-txt/stealth-oxide/blob/main/docs/images/sannysoft-full-page.png">
        <img src="https://raw.githubusercontent.com/robo-txt/stealth-oxide/main/docs/images/sannysoft-full-page.png" alt="Full-page Sannysoft report captured with stealth-oxide" width="420">
      </a>
    </td>
  </tr>
</table>

Observed network addresses are redacted before the PNGs are written. These are
reproducible environment snapshots, not guarantees about how a third-party
security or fraud-detection service will classify a browser.

## Installation

Add the crate and its browser-runtime dependencies:

```toml
[dependencies]
stealth-oxide = "0.1.0"
chromiumoxide = "0.9.1"
anyhow = "1"
futures = "0.3"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

Rust 1.86 or newer is required by the resolved Chromiumoxide dependency graph.

## Compatibility

The built-in profiles model Google Chrome 150.0.7871.128. Linux is the only
host with required real-browser CI coverage; Windows and macOS profiles model
browser-visible values but are not yet exercised on native CI runners. See the
[compatibility policy](docs/compatibility.md) for the complete Rust,
Chromiumoxide, Chromium, and host-platform support matrix.

Applications can compare a built-in profile with the product string returned
by CDP `Browser.getVersion` without giving this crate control of the browser
lifecycle:

```rust
use stealth_oxide::{
    CompatibilityStatus, PlatformProfile, compare_browser_versions,
};

let profile = PlatformProfile::Windows.profile();
let status = compare_browser_versions(
    profile.version(),
    "Chrome/150.0.7871.128",
);

assert_eq!(
    status,
    CompatibilityStatus::Compatible { chrome_major: 150 }
);
```

A customized user agent or Client Hint set clears inherited version metadata
unless the builder is given replacement metadata explicitly.

Profile seeding is optional and disabled by default. Enable it explicitly only
when an application needs reproducible test cookies or origin storage:

```toml
[dependencies]
stealth-oxide = {
    version = "0.1.0",
    features = ["seeding"]
}
```

## Quick start

Create an `about:blank` page, keep Chromiumoxide's event handler running, apply
the profile, and only then navigate to the destination:

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

### Persistent-profile language

For the most native language behavior, create
`ChromeLanguageConfig::from_profile(&profile)` before Chrome starts. Pass its
`chrome_argument()` to `BrowserConfigBuilder`, as shown above, and merge its
`preference_patch()` into the selected profile's `Preferences` JSON while
Chrome is stopped. The patch sets `intl.app_locale` and
`intl.selected_languages`; it deliberately does not write
`intl.accept_languages`, which Chromium derives from the selected-language
preference.

`StealthConfig::apply` still supplies CDP `acceptLanguage` as a target-scoped
fallback for temporary profiles. Chrome remains responsible for formatting the
HTTP header, including quality weights. The library does not edit Preferences
on disk because doing so while Chrome is running can lose user changes or be
overwritten by Chrome.

Platform selection is always explicit: choose `Linux`, `MacOS`, or `Windows`.
A preset describes the requested browser identity; it cannot transform the
underlying operating system, GPU, fonts, voices, or platform-only features.

### New target coverage

CDP emulation is target-scoped. Enable `TargetCoordinator` before navigation
when the destination can create dedicated or shared workers, related iframes,
or popups. The application must continuously drive the returned event stream;
the coordinator does not create a hidden runtime task. Service workers are
excluded from pausing because Chromiumoxide 0.9.1 does not expose public
arbitrary-session command routing and Chrome rejects its deprecated nested
service-worker session path. They continue with native host values and should
be reported as a coverage gap.

```rust,no_run
use futures::StreamExt;
use stealth_oxide::{PlatformProfile, StealthConfig, TargetCoordinator};

# async fn configure(page: chromiumoxide::Page) -> Result<(), Box<dyn std::error::Error>> {
let stealth = StealthConfig::for_platform(PlatformProfile::Linux);
stealth.apply(&page).await?;

let coordinator = TargetCoordinator::new(&stealth)?;
let mut targets = coordinator.enable(&page).await?;
let target_page = page.clone();
tokio::spawn(async move {
    while let Some(event) = targets.next().await {
        if let Err(error) = coordinator.apply(&target_page, &event).await {
            eprintln!("target configuration failed: {error}");
        }
    }
});

page.goto("https://example.com").await?;
# Ok(())
# }
```

Chrome 150 accepts worker-target UA, language, locale, timezone, and Client Hint
overrides, but it preserves the host value of `WorkerNavigator.platform`. A
Windows profile running on Linux therefore remains detectably hybrid even with
target coordination. Use the Linux profile on Linux or a native Windows browser
when page/worker platform equality is required.

### Passive network audit

`NetworkAudit` correlates Chrome's native `Network` events without enabling the
`Fetch` domain or pausing requests. Attach a bounded observer before navigation:

```rust,no_run
use stealth_oxide::NetworkAuditHandle;

// `page` is an existing chromiumoxide Page.
let audit = NetworkAuditHandle::attach(&page).await?;
page.goto("https://example.com").await?;
let audit = audit.stop().await;
println!("observed requests: {}", audit.summary().requests);
```

For lower-level integrations, `NetworkAudit` remains available: callers can
feed events from the same page/session into the corresponding `observe_*`
methods directly.

For complete lifecycle correlation, also subscribe to `requestWillBeSent`, both
extra-info events, `requestServedFromCache`, `loadingFinished`, and
`loadingFailed`. Extra-info observations are retained as ordered sequences
because CDP does not promise their ordering around redirects. Reports omit
bodies, cookies, authorization data, remote addresses, URL credentials, query
strings, and fragments, and retain at most 100 request lifecycles.

The audit also tracks origin-scoped `Accept-CH` observations. Call
`validate_client_hint_negotiation(&audit)` to check whether high-entropy hints
followed policy observed earlier in the audit. Findings are warnings rather
than contradictions when no prior opt-in was seen, since a persistent Chrome
profile can retain Client Hint preferences from an earlier session. The
library observes this negotiation and never adds `Sec-CH-UA-*` headers itself.

Network auditing is read-only: it validates what Chrome transmitted but does
not repair mismatches or rewrite requests. Applications that need explicit
header additions can enable the optional `interceptor` feature and use the
validated `HeaderPolicy` builder. Credential, cookie, hop-by-hop, and framing
headers remain unavailable to that policy.

## Granular control

Every patch can use an override, preserve Chromium's native value, or be
disabled:

```rust
use stealth_oxide::{Patch, PatchState, PlatformProfile, StealthConfig};

let config = StealthConfig::for_platform(PlatformProfile::Windows)
    .disable(Patch::Screen)
    .use_native(Patch::Touch)
    .timezone("America/Toronto")
    .languages(["en-CA", "en"])
    .locale("en-CA");

assert_eq!(config.patch_state(Patch::Screen), PatchState::Disabled);
assert_eq!(config.patch_state(Patch::Touch), PatchState::Native);
```

Available patches are:

- `Identity`: user agent, navigator platform, languages, and UA Client Hints
- `Locale`: Intl locale
- `Timezone`: IANA timezone
- `Screen`: screen dimensions and device scale factor
- `MediaFeatures`: color scheme, reduced motion, forced colors, color gamut,
  and monochrome depth
- `Touch`: touch enablement and maximum touch points

### Override reference

All configuration overrides are builder methods on `StealthConfig`:

| Area | Override methods |
| --- | --- |
| Complete identity | `identity`, `identity_patch` |
| User agent and Client Hints | `user_agent`, `client_hints` |
| Navigator | `navigator_platform`, `languages` |
| Locale | `locale`, `locale_patch` |
| Timezone | `timezone`, `timezone_patch` |
| Screen | `screen`, `screen_patch`, `screen_size`, `device_scale_factor` |
| Media features | `media_features`, `media_features_patch`, `color_scheme`, `reduced_motion`, `forced_colors`, `color_gamut` |
| Touch | `touch`, `touch_patch` |
| Validation behavior | `consistency_policy` |

The complete `MediaFeaturesConfig` also exposes `monochrome`, and
`ScreenConfig` can be supplied when the complete typed screen value is needed.
For any patch, `enable`, `disable`, and `use_native` control whether the
selected preset value is overridden, omitted, or preserved from Chromium.

For reusable custom profiles, `BrowserProfileBuilder` supports every modeled
profile field:

| Area | Profile builder methods |
| --- | --- |
| Metadata | `name` |
| Identity | `user_agent`, `navigator_platform`, `client_hints`, `languages` |
| Locale and timezone | `locale`, `timezone` |
| Screen | `screen`, `available_screen`, `device_scale_factor` |
| Media features | `color_scheme`, `reduced_motion`, `forced_colors`, `color_gamut`, `monochrome` |
| Touch | `touch` |

Call `build()` to validate the customized profile, then pass it to
`StealthConfig::from_profile`.

Start with no patches and opt in explicitly:

```rust
use stealth_oxide::{Patch, StealthConfig};

let config = StealthConfig::none()
    .enable(Patch::Identity)
    .enable(Patch::Locale)
    .enable(Patch::Timezone);
```

## Platform presets and typed values

```rust
use stealth_oxide::{
    ColorGamut, ColorScheme, ForcedColors, PlatformProfile, ReducedMotion,
    StealthConfig,
};

let config = StealthConfig::for_platform(PlatformProfile::Windows)
    .screen_size(2560, 1440)
    .device_scale_factor(1.25)
    .color_scheme(ColorScheme::Dark)
    .reduced_motion(ReducedMotion::NoPreference)
    .forced_colors(ForcedColors::None)
    .color_gamut(ColorGamut::Srgb)
    .touch(false, 0);
```

Complete typed patch values can also be supplied with `identity_patch`,
`locale_patch`, `timezone_patch`, `screen_patch`, `media_features_patch`, and
`touch_patch`.

## Consistency policies

Strict validation is the default. It rejects known contradictions before any
CDP command runs:

```rust
use stealth_oxide::{ConsistencyPolicy, PlatformProfile, StealthConfig};

let config = StealthConfig::for_platform(PlatformProfile::Linux)
    .navigator_platform("Win32")
    .consistency_policy(ConsistencyPolicy::Warn);

let issues = config.validation_issues();
assert!(!issues.is_empty());
```

- `Strict` rejects known contradictions before applying patches.
- `Warn` applies valid CDP values and returns issues in `ApplyReport`.
- `Permissive` applies the requested values and skips coherence checks. CDP
  parameter requirements and Chromium errors still apply.

Application is sequential, not transactional. If Chromium rejects a later
patch, the error records which earlier patches completed successfully.

## Proxies

Proxy configuration and credentials belong to the application and
Chromiumoxide. `stealth-oxide` never receives or stores them.

```rust,no_run
use chromiumoxide::auth::Credentials;
use chromiumoxide::{Browser, BrowserConfig};
use futures::StreamExt;
use stealth_oxide::{PlatformProfile, StealthConfig};

# async fn example() -> anyhow::Result<()> {
let config = BrowserConfig::builder()
    .hide()
    .arg(("proxy-server", "http://proxy.example:8080"))
    .build()
    .map_err(anyhow::Error::msg)?;
let (mut browser, mut handler) = Browser::launch(config).await?;
tokio::spawn(async move {
    while let Some(event) = handler.next().await {
        if let Err(error) = event {
            eprintln!("chromiumoxide handler error: {error:?}");
        }
    }
});

let page = browser.new_page("about:blank").await?;
page.authenticate(Credentials {
    username: "username".into(),
    password: "password".into(),
})
.await?;
StealthConfig::for_platform(PlatformProfile::Linux)
    .apply(&page)
    .await?;
page.goto("https://example.com").await?;
browser.close().await?;
# Ok(())
# }
```

## Chrome-native networking and retries

The default stealth path configures Chromium but does not intercept ordinary
requests. Chrome retains ownership of resource discovery, concurrency,
priority, redirects, connection pooling, cookies, cache, service workers,
HTTP/2 and HTTP/3 multiplexing, transport fallback, and native transport
retries. The optional interceptor feature is application infrastructure and is
not enabled by any stealth preset.

`NavigationRetryPolicy` makes an application-level decision for a completed
top-level navigation; it never sends or replays a request itself. Only `GET`,
`HEAD`, and `OPTIONS` are automatically eligible, and only `429` and `503`
responses are retry candidates. `Retry-After` supports both delta-seconds and
HTTP-date values. `403` and unsafe or unknown methods are never automatically
retried.

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

Callers that act on `RetryAfter` should wait and navigate the same page again,
preserving the browser process, user-data profile, proxy, cookies, cache, and
storage. Subresources must remain under Chrome's native retry behavior.
Retries, reloads, challenge handling, and profile lifecycle otherwise remain
application concerns. `stealth-oxide` never automatically revisits a URL after
a blocked response.

## Runnable examples

Each example focuses on one public workflow:

```bash
cargo run --example basic
cargo run --example custom_configuration
cargo run --example custom_profile
cargo run --features seeding --example profile_seeding
docker compose run --rm stealth-oxide cargo run --example creepjs_screenshot
docker compose run --rm stealth-oxide cargo run --example sannysoft_screenshot
```

- `basic`: launch Chromium, apply the recommended profile, and navigate.
- `custom_configuration`: select native, disabled, and overridden patches.
- `custom_profile`: build and apply a typed Windows desktop profile.
- `profile_seeding`: install repeatable test state with the optional `seeding`
  feature.
- `creepjs_screenshot` and `sannysoft_screenshot`: reproduce the full-page
  container captures shown above.

Library users must also opt in at runtime by calling `apply_profile_seeds`:

```rust,no_run
# #[cfg(feature = "seeding")]
# async fn seed(page: &chromiumoxide::Page) -> stealth_oxide::Result<()> {
use stealth_oxide::{CookieSeed, ProfileSeed, apply_profile_seeds};

let seeds = [ProfileSeed::new().cookie(
    CookieSeed::new("test-session", "value", "https://example.com/")
        .secure(true)
        .http_only(true),
)];

apply_profile_seeds(page, &seeds).await?;
# Ok(())
# }
```

Omit the `seeding` feature—or simply do not call `apply_profile_seeds`—to seed
nothing. Passing an empty slice is also a no-op.

CreepJS probes live under `tests/` as ignored browser diagnostics. See
[`tests/README.md`](tests/README.md) for the container commands.

Authorized protected-site evaluations are tracked separately in the
[`site evaluation matrix`](docs/site-evaluation-matrix.md). The matrix records
the tested commit, browser/runtime environment, repeated outcomes, and redacted
evidence. It deliberately distinguishes destination-content verification from
fingerprint-only diagnostics and does not treat an HTTP `200` as proof that a
challenge was passed.

The reproducible evaluation fixtures live under [`dev/evals/`](dev/evals/).
See the [site evaluation matrix](docs/site-evaluation-matrix.md) for the
recording format, repeatability requirements, and evidence rules.

Recent network-fidelity work adds bounded retry decisions, redacted passive
network auditing, transmitted-identity and Client-Hint validation, native
Chrome language startup, Chrome-major compatibility gates, and expanded
redirect/cache/worker/concurrency coverage. The [user-control and alignment
plan](docs/user-control-alignment-plan.md) documents the configuration contract,
validation rules, and next API slices.

## Container development

The repository Docker environment supplies Chromium, Xvfb, Openbox, a taskbar,
and Mesa llvmpipe for integration tests. These assets are excluded from the
published crate archive.

```bash
docker compose build
docker compose run --rm stealth-oxide
```

## Verification

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
cargo package
```

Real-browser integration tests are ignored by default because they require a
working Chromium process and, for desktop tests, the container environment.

## Scope and limitations

- Patches use CDP rather than page-world JavaScript overrides.
- CDP commands are target- and session-scoped.
- Applying a configuration to one page does not automatically cover workers,
  popups, service workers, or new targets; `TargetCoordinator` covers supported
  directly related targets only while its event stream is driven.
- Native screen work area, physical GPU identity, fonts, voices, and platform
  features can depend on the host environment.
- Successful navigation does not establish a favorable classification by a
  third-party fraud or bot-management service.
- Only test sites and proxies you are authorized to use.
- Error display strings are human-readable and are not a stable machine
  interface; match typed error variants instead.
