# CDP target-attachment probe results

Date: 2026-08-26

This pass used a Rust `chromiumoxide` client connected to the disposable
Chromium container over CDP. The container used Xvfb, Mesa LLVMpipe, and the
native ANGLE identity profile:

```text
LIBGL_ALWAYS_SOFTWARE=true
MESA_LOADER_DRIVER_OVERRIDE=llvmpipe
ANGLE_GL_VENDOR=AMD
ANGLE_GL_RENDERER=AMD Radeon HD 3200 Graphics
```

The client enabled `Target.setAutoAttach` with
`waitForDebuggerOnStart=true`, recorded attachment events, resumed paused
targets, and then read asynchronous worker results from the page.

## Results

Chromium created these relevant targets:

| Target type | Created | Resumed successfully | Result |
|---|---:|---:|---|
| `worker` | yes | yes | dedicated worker returned WebGL |
| `shared_worker` | not observed in the event stream | n/a | page still returned a shared-worker WebGL result |
| `service_worker` | yes | no | session was already gone when resume was sent |

The dedicated and shared workers both reported:

```text
WebGL available: true
contextLost: false
vendor: AMD
renderer: AMD Radeon HD 3200 Graphics
```

Their ordinary navigator fields also matched the page for UA, platform,
language, languages, hardware concurrency, device memory, locale, and
timezone. As in the earlier raw worker test, `navigator.webdriver` was
`false` on the page and absent/null in workers.

The page-side result is available at
[`cdp-target-probe.json`](../screen-gpu-lab/artifacts/docker-xvfb/cdp-target-probe.json).

## Service-worker lifecycle finding

The CDP stream did report a `service_worker` target for the registered script,
but sending `Runtime.runIfWaitingForDebugger` through the attached session
returned `No session with given id`. The service worker therefore cannot be
treated as covered merely because a Target event was observed.

This is a lifecycle/attachment result, not proof that service workers cannot
be configured. The next experiment should keep the service worker alive with
a controlled request or message, use CDP target-created/target-destroyed
timestamps, and test the current Chromiumoxide flattened-session path. It
must also distinguish a target that is short-lived from a target that was
never configured.

## Implication for stealth-oxide

The current `TargetCoordinator` intentionally excludes `service_worker` from
its auto-attach filter and from its supported target types. This probe shows
that dedicated and shared worker GPU identity already propagates through the
native ANGLE profile, while service workers require a separate lifecycle
design. Adding `service_worker` to the existing list without solving the
session lifetime would be premature.

No production Rust code was changed by this experiment. The probe source and
runner are research fixtures under `dev/webscraping-research/`.

## Sources

- [Chrome DevTools Protocol Target domain](https://chromedevtools.github.io/devtools-protocol/tot/Target/)
- [Chrome DevTools Protocol SystemInfo domain](https://chromedevtools.github.io/devtools-protocol/tot/SystemInfo/)
- [Chromium: using GPU hardware in headless Chrome](https://chromium.googlesource.com/chromium/src/+/refs/heads/main/docs/gpu/using-gpu-hardware-in-headless-chrome.md)
