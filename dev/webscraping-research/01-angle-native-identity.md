# ANGLE native GPU identity

## Finding

Current ANGLE source supports environment-variable overrides for the native
GL strings:

```text
ANGLE_GL_VENDOR
ANGLE_GL_RENDERER
ANGLE_GL_VERSION
```

ANGLE reads these while initializing its native `Context`. The renderer
override replaces the renderer string that ANGLE returns; it is not a
JavaScript property patch. The same code builds the vendor and version strings
used by the context.

Sources:

- [ANGLE `Context.cpp`](https://chromium.googlesource.com/angle/angle/+/refs/heads/main/src/libANGLE/Context.cpp)
- [ANGLE debugging tips: forcing GL vendor and renderer strings](https://chromium.googlesource.com/angle/angle/+/dd58a72292bcd10dd4387863e7bc99c9807e5a60/doc/DebuggingTips.md)
- [ANGLE project overview](https://chromium.googlesource.com/angle/angle)

## What this does not change

The override does not automatically change:

- supported extensions;
- numeric limits;
- shader precision;
- texture and framebuffer behavior;
- pixel output;
- GPU hardware/software classification in every Chromium subsystem;
- WebGPU adapter information;
- worker-specific browser APIs.

Therefore, the renderer string is only one layer of a GPU profile. CreepJS
must be used to test whether the remaining native surfaces contradict it.

## Experiments

On the host's accelerated Mesa path, Chromium was launched with:

```text
ANGLE_GL_VENDOR=AMD
ANGLE_GL_RENDERER=AMD Radeon HD 3200 Graphics
--enable-gpu --use-gl=angle --use-angle=gl
```

The native probe returned these values in both contexts:

```text
WebGL 1: AMD / AMD Radeon HD 3200 Graphics
WebGL 2: AMD / AMD Radeon HD 3200 Graphics
```

This is a successful proof that the override reaches Chromium's native ANGLE
WebGL surface without JavaScript injection.

## Version warning

The first experiment also set `ANGLE_GL_VERSION=OpenGL ES 3.2 Chromium`. That
caused invalid GL queries, a transform-feedback capability failure, WebGL
initialization failure, and a GPU-process crash. The safer rule is to preserve
ANGLE's native version until a complete capability profile has been modeled.

The renderer-only CPU experiment returned the requested identity for WebGL 2,
but WebGL 1 was unavailable. Because the same forced-LLVMpipe baseline also
crashed Chromium's GPU process, this result cannot yet distinguish an ANGLE
identity problem from a CPU-renderer stability problem.

## Implication

The next native experiment should use ANGLE environment variables as a
process-startup configuration, not an `LD_PRELOAD` wrapper and not a CDP
JavaScript patch. It should initially override only vendor and renderer, while
preserving native version, extensions, limits, precision, and pixels.
