# Mesa and CPU-only rendering

## Mesa's software path

Mesa documents `LIBGL_ALWAYS_SOFTWARE=true` as forcing software rendering.
`MESA_LOADER_DRIVER_OVERRIDE` selects a different Mesa driver, including
software drivers such as LLVMpipe in suitable configurations.

Sources:

- [Mesa environment variables](https://docs.mesa3d.org/envvars.html)
- [Mesa LLVMpipe driver](https://docs.mesa3d.org/drivers/llvmpipe.html)
- [Mesa driver and platform overview](https://docs.mesa3d.org/systems.html)

The native software baseline observed in this environment was:

```text
Vendor: Mesa
Renderer: llvmpipe (LLVM 22.1.8, 256 bits)
```

Chromium exposed the corresponding ANGLE form:

```text
Google Inc. (Mesa)
ANGLE (Mesa, llvmpipe (LLVM 22.1.8 256 bits), OpenGL ES 3.2)
```

That is why merely using Mesa does not meet the target. Mesa is the graphics
stack; LLVMpipe is still visibly a software renderer.

## Mesa variables that are not a complete identity solution

Mesa supports version and extension controls, including:

- `MESA_EXTENSION_OVERRIDE`;
- `MESA_GL_VERSION_OVERRIDE`;
- `MESA_GLES_VERSION_OVERRIDE`;
- `MESA_GLSL_VERSION_OVERRIDE`.

Mesa explicitly warns that version and shading-language overrides may advertise
features the driver does not really implement. They are documented as
developer/debugging controls, not as a hardware identity emulator.

## Gallium identity layer

Mesa's Gallium `pipe_screen` interface has separate identity methods such as
`get_name`, `get_vendor`, and `get_device_vendor`, plus device IDs and
capability queries.

Source:

- [Mesa Gallium screen interface](https://docs.mesa3d.org/gallium/screen.html)

This makes a profiled Gallium driver conceptually possible: LLVMpipe could do
the rendering while a driver layer supplied a consistent profile. However, it
would need to coordinate identity, extensions, limits, shader precision, and
capabilities. It is not a string-only configuration change.

## DRM shim assessment

Mesa's `drm-shim` can emulate a hardware DRM device so hardware drivers can be
initialized for testing. Mesa documents it as a driver/compiler testing tool,
and rendering is generally no-op'd unless a simulator-backed shim is used.

Source:

- [Mesa DRM shim documentation](https://docs.mesa3d.org/ci/drm-shim.html)

It is useful research infrastructure but is not currently a production
CPU-rendering solution for Chromium.

## CPU stability requirement

Before any GPU identity work is evaluated, the following must be stable in the
target Docker image:

1. Chromium GPU process startup.
2. WebGL 1 context creation.
3. WebGL 2 context creation.
4. shader compilation and precision queries.
5. canvas readback and pixel output.
6. dedicated, shared, and service-worker WebGL contexts.

The current host experiment does not satisfy item 1 reliably under forced
LLVMpipe, so that is the first CPU-only research blocker.
