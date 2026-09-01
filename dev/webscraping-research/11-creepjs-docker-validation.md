# CreepJS Docker validation

Date: 2026-08-26

This is a fresh CreepJS GPU-path run inside the disposable
`stealth-oxide/xvfb-research:latest` container. The container used Xvfb, Mesa
LLVMpipe, ANGLE OpenGL, and the native ANGLE identity profile without passing
`/dev/dri` or a physical GPU:

```text
LIBGL_ALWAYS_SOFTWARE=true
MESA_LOADER_DRIVER_OVERRIDE=llvmpipe
ANGLE_GL_VENDOR=AMD
ANGLE_GL_RENDERER=AMD Radeon HD 3200 Graphics
```

The browser connected to CreepJS over CDP. It waited 20 seconds, collected
the CreepJS object values, and captured a full-page screenshot.

Important comparison boundary: this runner launched Chromium directly and
did not apply `StealthConfig`, `TargetCoordinator`, or the profile user-agent
override. It validates the container's native rendering layer, not the full
stealth-oxide launch path.

## Evidence

- [Full CreepJS screenshot](../screen-gpu-lab/artifacts/docker-xvfb/creepjs-docker-angle-profile.png)
- [Extracted CreepJS result](../screen-gpu-lab/artifacts/docker-xvfb/creepjs-docker-angle-profile.json)
- [Runner log](../screen-gpu-lab/artifacts/docker-xvfb/creepjs-docker-angle-profile.log)

## What worked

CreepJS reported:

```text
hasSwiftShader: false
WebGL GPU confidence: high
WebGL GPU grade: A
GPU classification: AMD Radeon HD 3000s Graphics
WebGL vendor: AMD
WebGL renderer: AMD Radeon HD 3200 Graphics
Worker GPU: AMD Radeon HD 3200 Graphics
WebGL lied: false
Navigator lied: false
Worker lied: 0
```

The screenshot's WebGL and Worker panels both show the AMD identity. This
validates that CPU Mesa rendering plus the native ANGLE identity profile is
visible through CreepJS, including the worker surface.

## What did not work yet

The raw container run still showed:

```text
44% like headless
67% headless
0% stealth
chromium: true
```

The extracted result also showed:

```text
userAgent: ... HeadlessChrome/151.0.0.0 ...
hasHeadlessUA: true
hasHeadlessWorkerUA: true
noTaskbar: true
screen: 800 x 600
avail: 800 x 600
```

Therefore this run validates the Docker GPU architecture only. It does not
validate zero headless detection. The remaining signals are primarily the
headless browser mode/UA and the small default viewport/work-area setup, not
SwiftShader or a WebGL renderer mismatch.

This `67% headless` value must not be called a regression against the earlier
`0% headless` result. The earlier result was collected through the patched
`site_diagnostic` path, which applied the profile user-agent and stealth
configuration; this run intentionally did not. A valid apples-to-apples
Docker headless comparison still needs Chromium in the container connected to
the Rust launcher with `StealthConfig` applied before navigation.

## Decision

The CPU-only container path is worth continuing with. Do not attempt a custom
Chromium build solely to solve SwiftShader: CreepJS no longer sees SwiftShader
in this configuration. The next experiment should combine this same Docker
GPU setup with the actual stealth-oxide profile/target coordinator and the
tested native screen/work-area configuration. Headless UA/mode signals must be
measured separately from GPU identity.
