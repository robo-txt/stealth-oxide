# Worker and service-worker propagation

## Why this matters

CreepJS compares page and worker GPU surfaces. A profile that only changes the
main page can leave a worker exposing Mesa, LLVMpipe, or SwiftShader. That
contradiction is more suspicious than either identity by itself.

The local CreepJS sources inspected for this project read worker WebGL renderer
values and compare worker behavior with page behavior:

- [`context_repos/creepjs/src/worker/index.ts`](../../../../context_repos/creepjs/src/worker/index.ts)
- [`context_repos/creepjs/src/webgl/index.ts`](../../../../context_repos/creepjs/src/webgl/index.ts)
- [`context_repos/creepjs/src/headless/index.ts`](../../../../context_repos/creepjs/src/headless/index.ts)

## Native environment inheritance

ANGLE environment variables are process-startup configuration. Chromium's GPU
process inherits the browser launch environment, so the identity override is
structurally better suited to page and worker contexts than a page-level
JavaScript injection.

This is an architectural expectation, not yet a completed service-worker
validation. The target must still be tested explicitly because target creation,
GPU context creation, and worker startup happen at different times.

## CDP target handling

CDP can observe and attach to page, iframe, worker, shared-worker, and
service-worker targets through the Target domain. SystemInfo reports GPU data,
but it is observational; it does not provide a GPU renderer override.

Sources:

- [CDP Target domain](https://chromedevtools.github.io/devtools-protocol/tot/Target/)
- [CDP SystemInfo domain](https://chromedevtools.github.io/devtools-protocol/tot/SystemInfo/)

The current repository's target coordinator intentionally excludes service
workers from automatic attachment. Its existing JavaScript GPU patch therefore
cannot be treated as proof that service-worker WebGL is fixed.

## Required worker experiment

For one GPU profile, run the same native ANGLE environment and collect:

1. page WebGL 1 and WebGL 2;
2. iframe WebGL 1 and WebGL 2;
3. dedicated worker WebGL;
4. shared worker WebGL;
5. service-worker WebGL, where supported;
6. CreepJS worker/service-worker panel;
7. renderer, vendor, version, extensions, limits, and pixel hashes.

Any missing context must be recorded as an availability difference. It must not
be silently reported as consistency.
