# Docker/Xvfb CPU-rendering research

Date: 2026-08-25

This phase tested the disposable research image
`stealth-oxide/xvfb-research:latest`, derived from
`webscraper-eval/rust-stealth-oxided:latest`. The container did not receive
`/dev/dri` or a physical GPU. It installed Mesa userspace libraries, Mesa
drivers, `mesa-utils`, and Xvfb only for the experiment.

The probe was the small WebGL 1/2 page at
[`cpu-stability-probe.html`](../screen-gpu-lab/cpu-stability-probe.html).
Artifacts are under
[`artifacts/docker-xvfb`](../screen-gpu-lab/artifacts/docker-xvfb/).

## Matrix

| Case | Display/backend | CPU driver | WebGL 1/2 | Reported identity | Result |
|---|---|---|---|---|---|
| Docker control | Chromium headless | SwiftShader | available/available | SwiftShader | stable control |
| No display | headless/Ozone and ANGLE OpenGL | LLVMpipe requested | unavailable/unavailable | none | EGL initialization failed |
| Xvfb | X11 + ANGLE OpenGL | LLVMpipe | available/available | Mesa/X.org + llvmpipe | stable |
| Xvfb profile | X11 + ANGLE OpenGL | LLVMpipe | available/available | AMD / AMD Radeon HD 3200 Graphics | stable |
| Xvfb softpipe request | X11 + ANGLE OpenGL | softpipe requested | available/available | Mesa/X.org + llvmpipe | resolved to LLVMpipe here |

## Important failure boundary

In the base container, forcing Mesa/LLVMpipe with ANGLE OpenGL failed before a
usable context was created. Chromium logged `Could not open the default X
display` and `EGL_NOT_INITIALIZED`, and both WebGL versions were unavailable.
Using `--ozone-platform=headless` did not remove this dependency for this
Chromium build and ANGLE path.

That result does not show that CPU Mesa is unusable. It shows that this path
needs a display server. Adding Xvfb supplied the X11/EGL surface without
passing a physical GPU into the container.

## Xvfb baseline

The Xvfb LLVMpipe artifact reported:

```text
WebGL 1: available=true, contextLost=false
WebGL 2: available=true, contextLost=false
vendor:   Google Inc. (Mesa/X.org)
renderer: ANGLE (Mesa/X.org, llvmpipe (LLVM 15.0.6 256 bits), OpenGL 4.5)
```

The Chromium log contained expected D-Bus warnings but no GPU-process crash,
EGL initialization failure, or WebGL context-loss event.

## Native ANGLE identity profile

The same Xvfb/LLVMpipe run was repeated with only these process environment
variables added:

```text
ANGLE_GL_VENDOR=AMD
ANGLE_GL_RENDERER=AMD Radeon HD 3200 Graphics
```

The result was stable in both page WebGL contexts:

```text
WebGL 1: available=true, contextLost=false
WebGL 2: available=true, contextLost=false
vendor:   AMD
renderer: AMD Radeon HD 3200 Graphics
```

The test intentionally did not set `ANGLE_GL_VERSION`; ANGLE continued to
provide the native WebGL/OpenGL version and capability surface. This keeps the
experiment narrower: identity is changed while extensions, limits, shader
precision, and pixels remain implementation-owned until separately measured.

This is a promising architecture for a CPU-only container: Xvfb provides the
display primitive, Mesa/LLVMpipe performs rendering, and ANGLE supplies a
native identity override. It is not yet a CreepJS fix. The current evidence
only covers page WebGL availability and basic vendor/renderer fields.

## Remaining acceptance tests

Before any production wiring, repeat the profile in the actual browser
launcher and compare it against a reference profile for:

- WebGL extensions, limits, shader precision, and pixel hashes;
- Chromium GPU metadata and GPU-process stability;
- page, iframe, dedicated-worker, shared-worker, and service-worker contexts;
- CreepJS WebGL, service-worker, and headless panels;
- screen/work-area behavior and any WebGPU exposure;
- repeated runs to detect context loss or process crashes.

The Xvfb Docker files are research fixtures. They do not authorize wiring
ANGLE environment variables into the Rust library yet.

## Sources

- [Chromium: using GPU hardware in headless Chrome](https://chromium.googlesource.com/chromium/src/+/refs/heads/main/docs/gpu/using-gpu-hardware-in-headless-chrome.md)
- [ANGLE debugging tips](https://chromium.googlesource.com/angle/angle/+/dd58a72292bcd10dd4387863e7bc99c9807e5a60/doc/DebuggingTips.md)
- [ANGLE context identity handling](https://chromium.googlesource.com/angle/angle/+/refs/heads/main/src/libANGLE/Context.cpp)
- [Mesa environment variables](https://docs.mesa3d.org/envvars.html)
- [Docker GPU resources](https://docs.docker.com/engine/containers/gpu/)
