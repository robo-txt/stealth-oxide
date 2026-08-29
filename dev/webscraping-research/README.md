# Web-scraping browser research

Research notes for running Chromium in a CPU-only Docker container while
exposing a coherent, hardware-style GPU identity to WebGL and CreepJS.

Date: 2026-08-24

## Current conclusion

The strongest native lead is ANGLE's built-in identity override:

```text
ANGLE_GL_VENDOR
ANGLE_GL_RENDERER
ANGLE_GL_VERSION
```

These values are read inside ANGLE's native `Context`, not injected into the
page by JavaScript. A host-accelerated Chromium test successfully exposed
`AMD / AMD Radeon HD 3200 Graphics` in both WebGL 1 and WebGL 2.

The CPU-only path is now stable in the disposable Docker/Xvfb experiment:
Mesa/LLVMpipe renders without `/dev/dri`, and ANGLE can expose the selected
native identity in page WebGL 1 and 2. The no-display Docker path still fails
to initialize ANGLE OpenGL, so Xvfb is currently part of the tested design.
This is a research result, not yet a production fix: worker/service-worker
propagation, CreepJS panels, capabilities, and pixel parity remain open.

## Documents

- [ANGLE native identity](01-angle-native-identity.md)
- [Mesa and CPU rendering](02-mesa-cpu-rendering.md)
- [Chromium GPU pipeline](03-chromium-gpu-pipeline.md)
- [Workers and service workers](04-worker-service-worker-propagation.md)
- [Lab results](05-lab-results.md)
- [Research-only next steps](06-next-research-plan.md)
- [Phase 1 CPU stability results](07-phase1-cpu-stability.md)
- [Docker/Xvfb research](08-docker-xvfb.md)
- [Docker worker propagation](09-worker-propagation-results.md)
- [CDP target-attachment results](10-cdp-target-probe-results.md)
- [CreepJS Docker validation](11-creepjs-docker-validation.md)
- [CreepJS Docker + StealthConfig validation](12-creepjs-docker-stealthconfig.md)

## Scope boundary

These are research notes. No ANGLE environment variables have been wired into
the Rust library, and no production GPU behavior should be considered fixed.
The existing files under `dev/screen-gpu-lab/` are isolated experiments.
