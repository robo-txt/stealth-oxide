# Chromium GPU pipeline

## Native values versus metadata

Chromium has multiple GPU representations:

- GPU device vendor and device IDs;
- software-renderer classification;
- `GL_VENDOR`;
- `GL_RENDERER`;
- `GL_VERSION`;
- GL extensions;
- ANGLE backend and display type;
- WebGL context values;
- WebGPU adapter values.

Chromium's GPU collector creates a real GL context and reads the native GL
strings and extensions from it. In the software GL path it also uses internal
software vendor/device IDs and identifies the driver vendor as `SwANGLE`.

Sources:

- [Chromium GPU info structure](https://chromium.googlesource.com/chromium/src/+/HEAD/gpu/config/gpu_info.h)
- [Chromium GPU information collection](https://chromium.googlesource.com/chromium/src/+/HEAD/gpu/config/gpu_info_collector.cc)
- [Chromium GPU debugging](https://chromium.googlesource.com/chromium/src/+/show/main/docs/gpu/debugging_gpu_related_code.md)

Changing only Chromium's GPU metadata would therefore be insufficient. WebGL
must receive a coherent native context profile as well.

## Multi-process behavior

Chromium's WebGL client sends commands to a GPU service process; the actual
driver calls occur in that process. Chromium's own debugging documentation
describes this split and provides separate client/service logging controls.

This explains why the `LD_PRELOAD` experiment was unreliable: a wrapper that
works in a simple GLX client is not necessarily a safe interception point for
Chromium's ANGLE GPU process.

## ANGLE is the relevant boundary

ANGLE translates OpenGL ES calls to a platform backend. On Linux, the current
Chromium setup used in the lab selected ANGLE's OpenGL backend. ANGLE's native
context string overrides operate at the correct boundary for WebGL identity,
but they do not create a complete hardware simulation by themselves.

Sources:

- [ANGLE project overview](https://chromium.googlesource.com/angle/angle)
- [Chromium headless GPU documentation](https://chromium.googlesource.com/chromium/src/+/refs/heads/main/docs/gpu/using-gpu-hardware-in-headless-chrome.md)

## Required validation surfaces

For each candidate GPU profile, collect all of the following:

```text
about:gpu / SystemInfo
WebGL 1
WebGL 2
WEBGL_debug_renderer_info
supported extensions
numeric limits
shader precision
rendered pixel hashes
canvas output
WebGPU, if available
page and iframe contexts
dedicated/shared/service-worker contexts
```

The result must be recorded as a matrix, not reduced to whether one renderer
string changed.
