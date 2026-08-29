# Docker worker-propagation results

Date: 2026-08-25

This is the first worker pass over the Xvfb/LLVMpipe/ANGLE-profile container.
It uses a loopback HTTP page and records browser-visible identity values from
the page and a dedicated worker. The page's WebGL calls were intentionally
disabled in this pass so worker messaging and navigator propagation could be
measured independently from the GPU-context subprobe.

## Dedicated-worker result

The page and dedicated worker agreed on:

```text
userAgent
platform
language / languages
hardwareConcurrency
deviceMemory
locale
timezone
```

The only mismatch was:

```text
page:   navigator.webdriver === false
worker: navigator.webdriver === null/absent
```

This is a real consistency signal in the raw Chromium run. It is not evidence
that the ANGLE profile failed: the GPU subprobe was disabled in this pass. It
does show that a profile-level worker acceptance test must include
`navigator.webdriver` and must distinguish a missing property from `false`.

## SharedWorker and service worker

The same `--dump-dom` harness did not receive a SharedWorker response within
the timeout. A service worker registered and reached the ready state, but its
message response also timed out. These observations are not sufficient to
claim that either worker type is unsupported or inconsistent. The dump-dom
launcher is a weak measurement point for background targets because it does
not expose target-created/target-attached events or a reliable CDP lifecycle
wait.

The next worker experiment must use CDP target discovery and auto-attach. It
should capture `worker`, `shared_worker`, and `service_worker` target creation,
attach to each target, apply the profile before evaluation, and collect both
the navigator snapshot and WebGL/OffscreenCanvas snapshot. Chromium's CDP
Target domain explicitly supports target discovery and attachment, which is
the correct measurement boundary for this question.

## GPU subprobe status

An earlier version attempted OffscreenCanvas WebGL in the dedicated worker
inside the same dump-dom run and did not receive a response. Because the page
and worker messaging harness also had lifecycle issues, that timeout cannot
yet be classified as a GPU crash, context loss, unsupported worker WebGL, or a
launcher wait problem.

The GPU worker test is therefore still open. The CDP version must record:

- target creation and destruction;
- worker evaluation errors and console errors;
- WebGL 1/2 availability and context-loss state;
- vendor, renderer, version, extensions, limits, precision, and a pixel hash;
- GPU-process logs and `SystemInfo` data;
- comparison with the page context under the same ANGLE profile.

## Research conclusion

The native profile currently propagates ordinary navigator identity into a
dedicated worker, but raw worker `webdriver` behavior differs. No conclusion
has been reached for SharedWorker, service-worker, or worker WebGL GPU
identity. Those are the next acceptance gates before production changes.

Artifact:

- [`worker-propagation.html`](../screen-gpu-lab/artifacts/docker-xvfb/worker-propagation.html)

Sources:

- [Chrome DevTools Protocol Target domain](https://chromedevtools.github.io/devtools-protocol/tot/Target/)
- [Chrome DevTools Protocol SystemInfo domain](https://chromedevtools.github.io/devtools-protocol/tot/SystemInfo/)
- [Chromium: using GPU hardware in headless Chrome](https://chromium.googlesource.com/chromium/src/+/refs/heads/main/docs/gpu/using-gpu-hardware-in-headless-chrome.md)
