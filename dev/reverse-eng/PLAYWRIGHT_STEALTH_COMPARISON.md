# Playwright stealth vs. stealth-oxide

## Scope and provenance

This review compares the source checked out in `dev/reverse-eng/playwright-stealth`
with the current `stealth-oxide` workspace.

The imported source is the TypeScript-compatible Node path used by Playwright:

- `packages/playwright-extra`: TypeScript plugin/lifecycle adapter.
- `packages/puppeteer-extra-plugin-stealth`: stealth plugin and evasion payloads.

The evasion payloads in this upstream snapshot are JavaScript, but the packages
ship TypeScript-facing declarations and the Playwright adapter is TypeScript.
The source checkout is upstream commit `39248f1f5deeb21b1e7eb6ae07b8ef73f1231ab9`
(`Publish`, 2023-03-01). The npm package named `playwright-stealth` is not the
implementation being compared; it is a placeholder package.

This is an implementation comparison for authorized automation, diagnostics,
and compatibility testing. It is not a claim that either project defeats modern
bot detection.

## Executive summary

`playwright-extra` adapts Puppeteer-style plugin lifecycle hooks to Playwright
and translates `evaluateOnNewDocument` to Playwright `addInitScript`
(`packages/playwright-extra/src/extra.ts:76-102,221-269` and
`packages/playwright-extra/src/puppeteer-compatiblity-shim/index.ts:178-182`).
The stealth plugin then enables 16 evasion modules by default (there are 17
evasion directories in this checkout; `navigator.vendor` is available but not
in the default set)
(`packages/puppeteer-extra-plugin-stealth/index.js:70-91`). Most modules patch
page JavaScript prototypes, getters, proxies, or missing objects before site
code runs; two modules modify launch arguments and one intercepts CDP evaluation
commands.

`stealth-oxide` applies a smaller set of browser-native CDP overrides from typed
profiles. Its owned patch groups are identity, locale, timezone, screen,
media features, and touch (`src/config.rs:21-37`), applied before navigation in
a deterministic order (`src/lib.rs:66-124`). It also provides target
coordination for identity/locale/timezone (`src/targets.rs:66-223`), validation,
network identity auditing, and profile seeding. It does not own a general
JavaScript evasion bundle.

The practical distinction is:

| Dimension | playwright-stealth | stealth-oxide |
| --- | --- | --- |
| Primary layer | Page JavaScript plus launch/CDP hooks | CDP emulation and typed launch/profile configuration |
| Main goal | Remove common headless/Puppeteer surface clues | Keep browser-visible identity and environment values coherent |
| Coverage | Broad API shims, but mostly page-realm only | Narrower patch set, with explicit target/network consistency |
| Native-shape handling | Proxies, descriptors, native-looking `toString`, sanitized stacks | Usually leaves native APIs untouched; CDP changes the underlying reported values |
| Configuration | Evasions can be enabled/disabled; many defaults are fixed | Typed platform profiles, per-patch modes, strict/warn/permissive validation |
| Browser scope | Chromium-oriented; adapter can expose other Playwright engines, but stealth is Chromium-focused | Chromium/CDP via chromiumoxide |

## Evasion-by-evasion comparison

| Upstream evasion | Technique | stealth-oxide status |
| --- | --- | --- |
| `chrome.app` | Creates `window.chrome` when absent, supplies static `chrome.app` data, realistic getters/functions, and native-looking function strings (`evasions/chrome.app/index.js:19-94`) | No owned equivalent. `window.chrome` is not part of the Rust patch groups. |
| `chrome.csi` | Reconstructs deprecated timing API from `performance.timing` (`evasions/chrome.csi/index.js:33-67`) | No equivalent. |
| `chrome.loadTimes` | Reconstructs deprecated connection/timing API from Navigation Timing and paint entries (`evasions/chrome.loadTimes/index.js:31-161`) | No equivalent. |
| `chrome.runtime` | Mocks secure-origin `chrome.runtime`, static data, `connect`, and `sendMessage`; validates call signatures and emits Chrome-like errors through proxies (`evasions/chrome.runtime/index.js:25-248`) | No equivalent. |
| `defaultArgs` | Adds Chromium args to ignore extension/default-app switches that reveal an automation launch (`evasions/defaultArgs/index.js:5-39`) | Partial/delegated. The README and tests use chromiumoxide `BrowserConfig::hide()` (`README.md:97-106`, `tests/common/mod.rs:14-18`), but stealth-oxide itself does not own launch-argument sanitization. |
| `iframe.contentWindow` | Intercepts iframe creation and `srcdoc`; supplies a `contentWindow` proxy that repairs `self`, `frameElement`, and index behavior (`evasions/iframe.contentWindow/index.js:27-130`) | No equivalent. Target coordination configures targets, but does not proxy iframe objects. |
| `media.codecs` | Proxies `HTMLMediaElement.prototype.canPlayType` for selected proprietary MP4/AAC/M4A results (`evasions/media.codecs/index.js:20-85`) | No equivalent. |
| `navigator.hardwareConcurrency` | Replaces the navigator getter with a proxy returning 4 by default (`evasions/navigator.hardwareConcurrency/index.js:31-43`) | No equivalent. `hardwareConcurrency` remains native; the environment module only validates observed memory buckets and page/worker comparisons (`src/environment.rs:1-88`). |
| `navigator.languages` | Replaces the getter with a frozen language array, defaulting to `['en-US', 'en']` (`evasions/navigator.languages/index.js:27-43`) | Partial/stronger native path. `SetUserAgentOverride.acceptLanguage` and profile languages configure identity and HTTP language (`src/patches/identity.rs:60-76`); startup `--lang` and selected-language preferences are also exposed (`src/launch.rs:19-61`). |
| `navigator.permissions` | Repairs headless notification behavior for secure and insecure origins by proxying `Notification.permission` and `Permissions.query` (`evasions/navigator.permissions/index.js:23-64`) | No owned equivalent. |
| `navigator.plugins` | Builds functional `PluginArray` and `MimeTypeArray` mocks with cross-references and native-like methods (`evasions/navigator.plugins/index.js:35-95`) | No owned equivalent. |
| `navigator.vendor` | Optional getter proxy, defaulting to `Google Inc.` (`evasions/navigator.vendor/index.js:45-61`) | No equivalent. Also note this module is present in the tree but is not in the default `availableEvasions` set in this snapshot. |
| `navigator.webdriver` | Uses `--disable-blink-features=AutomationControlled`, then deletes the prototype property only for old Chrome cases (`evasions/navigator.webdriver/index.js:18-42`) | Partial/delegated. The standard examples use chromiumoxide `.hide()`, which supplies the AutomationControlled launch flag; this behavior is not implemented by stealth-oxide's own patch module. |
| `sourceurl` | Intercepts the page CDP client's `send`, strips `//# sourceURL=__puppeteer_evaluation_script__` from `Runtime.evaluate` and `Runtime.callFunctionOn` (`evasions/sourceurl/index.js:18-77`) | No equivalent. `stealth-oxide` observes/redacts network identity but does not rewrite CDP evaluation payloads. |
| `user-agent-override` | Calls `Network.setUserAgentOverride` with UA, platform, language, and generated GREASE/UA-CH metadata; optionally changes Linux to Windows and persists language preferences (`evasions/user-agent-override/index.js:65-205`) | Strong overlap, but more explicit typed modeling. `NavigatorProfile` carries UA/platform/languages/UA-CH fields (`src/profiles/mod.rs:142-181`), and the identity patch sends them through CDP (`src/patches/identity.rs:60-76`). Built-in profiles are pinned to Chrome 151 (`src/profiles/mod.rs:8-32`). |
| `webgl.vendor` | Proxies `getParameter` for both WebGL and WebGL2, changing only unmasked vendor/renderer constants (`evasions/webgl.vendor/index.js:26-53`) | No patch equivalent. Instead, the WebGL trigger verifies native top/iframe/worker parity across vendor, renderer, parameters, precision, and pixels (`tests/triggers/webgl_emulation.rs:143-165`). |
| `window.outerdimensions` | Fills missing `outerWidth/outerHeight` and sets a null default viewport at launch (`evasions/window.outerdimensions/index.js:18-39`) | Partial conceptual overlap. Screen patching uses `Emulation.setDeviceMetricsOverride` with width, height, scale, and orientation (`src/patches/screen.rs:12-35`), but it does not synthesize the OS window frame or outer dimensions. |

## Cross-cutting implementation differences

### JavaScript surface and native-shape camouflage

The upstream plugin has a dedicated utility layer for replacing properties on
prototypes, wrapping getters/functions in proxies, redirecting
`Function.prototype.toString`, and removing proxy frames from thrown error
stacks (`evasions/_utils/index.js:18-104,126-176,197-345,384-423`). This is its
central stealth technique: preserve the expected result and make the altered
function look and fail like the original.

That design also creates a high-value consistency surface. A detector can test
descriptors, own keys, invocation errors, `instanceof`, cross-realm behavior,
proxy recursion, and `Function.prototype.toString`. CreepJS explicitly exercises
these classes of checks in `src/lies/index.ts` and includes Navigator, iframe,
canvas, DOMRect, SVG, Permissions, and WebGL APIs in its search set.

`stealth-oxide` generally avoids that class of patch. It changes values through
CDP and leaves most JavaScript prototypes, descriptors, error stacks, canvas,
audio, DOM geometry, SVG, fonts, speech, and media-device APIs native. This
reduces injected-JavaScript artifacts, but it also means the corresponding
headless values are not normalized by stealth-oxide itself.

### Realm and worker coverage

The Playwright adapter installs page init scripts through `addInitScript` and
hooks pages created by browser contexts (`extra.ts:221-249`). Its stealth
modules are designed around the page/iframe realm and Puppeteer-compatible CDP
access. The source snapshot does not provide a general worker-target
coordination mechanism for every evasion.

`stealth-oxide` explicitly exposes `TargetCoordinator` and pauses new targets
before applying identity, locale, and timezone commands (`src/targets.rs:113-223`).
Its tests verify cross-realm and worker behavior, but they also document an
important limitation: Chrome 151 preserves the host worker
`WorkerNavigator.platform`, producing a known page/worker mismatch
(`tests/triggers/worker_consistency.rs:147-164`).

### Identity and consistency model

The upstream UA evasion infers values from the browser UA, generates a GREASE
brand order from the Chrome major version, and can mask Linux as Windows
(`evasions/user-agent-override/index.js:65-166`). It is pragmatic and aimed at
headless defaults.

`stealth-oxide` represents identity as a typed profile containing the UA,
platform, ordered languages, and structured UA-CH values. It validates
contradictions before applying in strict mode and can instead warn or allow
them (`src/config.rs:62-71`, `src/lib.rs:78-90`). This is better suited to
reproducible QA, but the built-in values are intentionally static and must be
kept synchronized with the actual Chromium binary and host environment.

### Screen, locale, timezone, media, and touch

This is the main area where stealth-oxide is broader than the imported stealth
bundle. Its profiles and CDP patches cover:

- `Emulation.setLocaleOverride` and `Emulation.setTimezoneOverride`
  (`src/patches/locale.rs:9-16`, `src/patches/timezone.rs:9-20`).
- Screen metrics, scale factor, and orientation (`src/patches/screen.rs:12-35`).
- CSS media features (`src/patches/media_features.rs:9-24`).
- Touch enablement and maximum touch points (`src/patches/touch.rs:9-24`).

The imported stealth plugin does not have direct equivalents for timezone,
screen orientation, CSS media features, or touch. Conversely, the plugin's
JavaScript repairs for permissions, plugins, codecs, Chrome extension objects,
and native function shape do not exist in stealth-oxide.

### WebGL, canvas, geometry, and rendering

The upstream bundle changes only WebGL vendor/renderer constants. It does not
normalize WebGL extensions, precision, parameters, pixels, Canvas 2D output,
text metrics, DOMRect geometry, or SVG metrics.

`stealth-oxide` also does not patch those surfaces. Its approach is to measure
and assert parity: the WebGL trigger compares top window, iframe, and worker
values including parameters, precision, and pixels (`tests/triggers/webgl_emulation.rs:143-165`).
This is a meaningful architectural difference: stealth-oxide currently favors
detecting contradictions over fabricating renderer output.

## Coverage matrix

| Detection surface | playwright-stealth | stealth-oxide | Result |
| --- | --- | --- | --- |
| UA string / appVersion / platform | UA + platform via CDP; UA string removes `HeadlessChrome` | UA, platform, languages, UA-CH via CDP/profile | Both cover the main identity path; oxide is more typed and validation-oriented. |
| `navigator.webdriver` | Launch flag plus legacy property deletion | Usually provided by chromiumoxide `.hide()` in caller setup | Equivalent only when the caller uses the delegated launch setting. |
| `navigator.languages` / Accept-Language | Getter patch plus CDP/prefs | CDP identity plus startup language configuration | Oxide has the stronger network/startup model. |
| `navigator.plugins` / mime types | Functional cross-referenced mocks | Native only | Playwright covers headless absence; oxide leaves a major gap. |
| Notification permissions | Origin-sensitive JS repair | Native only | Playwright covers a known headless contradiction; oxide does not. |
| `chrome.*` objects | `chrome.app`, `csi`, `loadTimes`, `runtime` mocks | Native only | Playwright covers more Chrome object absence/shape clues. |
| WebGL vendor/renderer | JS proxy for two WebGL prototypes | Native, with parity diagnostics | Different goals: spoof versus observe/validate. |
| Media codec support | `canPlayType` proxy for selected codecs | Native only | Playwright covers Chromium-vs-Chrome codec discrepancy. |
| `outerWidth/outerHeight` | JS synthesis plus viewport launch change | Device metrics override only | Partial overlap. |
| iframe `contentWindow` | Proxy for srcdoc iframe behavior | Native iframe plus target coordination | Different problem: object-shape repair versus target configuration. |
| Function/proxy/error-stack probes | Extensive utility layer | No broad JS utility layer | Large Playwright-only surface, with corresponding lie risk. |
| Canvas / audio / DOMRect / SVG / fonts / speech | Native | Native | Neither bundle comprehensively normalizes these surfaces. |
| Timezone / locale / CSS media / touch | Little or no direct coverage | Typed CDP patches | Strong stealth-oxide-only area. |
| New pages / iframes / workers | Page/context lifecycle; no universal worker patch plan | Explicit identity/locale/timezone target coordinator | Oxide has more explicit target coverage, but worker platform remains a known limitation. |
| Network identity auditing | No comparable read-only audit in this source | Redacted request/redirect/Client Hint audit | Strong stealth-oxide-only diagnostic capability. |

## Bottom line

Playwright-stealth is a compatibility-oriented collection of targeted headless
evasion recipes. Its strongest contributions are the page-realm shims for
`chrome.*`, plugins/mime types, permissions, codecs, WebGL vendor, iframe
window shape, `webdriver`, and the proxy/native-function camouflage utilities.

Stealth-oxide is not a Rust port of that bundle. It is a profile and CDP
consistency system. Its strongest contributions are coherent identity and UA-CH
configuration, native startup language handling, locale/timezone/screen/media/
touch emulation, explicit target coordination, contradiction validation, and
read-only network diagnostics.

The largest unimplemented upstream techniques in stealth-oxide are functional
plugins/mime types, permission normalization, Chrome object shims, media codec
normalization, WebGL vendor/renderer overrides, iframe content-window repair,
outer-dimension synthesis, and CDP sourceURL filtering. The largest
stealth-oxide capabilities absent from the imported bundle are typed profile
validation, timezone/locale/media/touch CDP control, target coordination, and
network identity auditing.

Any future additions should be driven by measured, cross-surface test failures:
the proxy-heavy upstream techniques can themselves produce prototype lies or
realm inconsistencies, while broad value overrides can create contradictions
with the real host GPU, fonts, workers, or network behavior.
