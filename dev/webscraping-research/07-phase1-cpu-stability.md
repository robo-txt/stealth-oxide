# Phase 1 CPU-only Chromium stability research

Date: 2026-08-25

This phase tested the host Chromium/Mesa stack before any native GPU identity
override was evaluated. The probe is:

- [`cpu-stability-probe.html`](../screen-gpu-lab/cpu-stability-probe.html)
- [`run-cpu-stability-matrix.sh`](../screen-gpu-lab/run-cpu-stability-matrix.sh)

The probe records WebGL 1/2 availability, context-loss state, renderer/vendor,
extension count, and selected limits. Chromium logs record GPU-process exits.

## Results

| Case | WebGL 1 | WebGL 2 | Native renderer | Stability |
|---|---:|---:|---|---|
| Headless native | yes | yes | SwiftShader | stable |
| Headless new native | yes | yes | SwiftShader | stable |
| Headless + LLVMpipe | no | yes | Mesa / llvmpipe | failed |
| Headless + Softpipe request | no usable result | partial/failed | resolved to llvmpipe | failed |
| ANGLE SwiftShader control | yes | yes | SwiftShader | stable |
| X11 + LLVMpipe | no usable result | partial/failed | Mesa / llvmpipe | failed |

The native control values came from the generated artifacts under
`dev/screen-gpu-lab/artifacts/cpu-stability/`.

## LLVMpipe behavior

With:

```text
LIBGL_ALWAYS_SOFTWARE=true
MESA_LOADER_DRIVER_OVERRIDE=llvmpipe
--enable-gpu --use-gl=angle --use-angle=gl
```

Chromium produced a WebGL2 context with:

```text
vendor: Google Inc. (Mesa)
renderer: ANGLE (Mesa, llvmpipe (LLVM 22.1.8 256 bits), OpenGL ES 3.2)
maxTextureSize: 16384
maxViewportDims: 16384 x 16384
extensionCount: 32
```

WebGL1 was unavailable. Chromium logs showed:

```text
GPU state invalid after WaitForGetOffsetInRange
GPU process exited unexpectedly: exit_code=139
The GPU process has crashed
WebGL: CONTEXT_LOST_WEBGL
```

The same behavior reproduced with Chromium's normal sandbox enabled. The
failure is therefore not explained by the `--no-sandbox` diagnostic setting.

## Softpipe behavior

On this host, asking Mesa for `softpipe` through:

```text
MESA_LOADER_DRIVER_OVERRIDE=softpipe
LIBGL_ALWAYS_SOFTWARE=true
```

still produced:

```text
Vendor: Mesa
Device: llvmpipe (LLVM 22.1.8, 256 bits)
```

This means the current Mesa installation either maps the request to LLVMpipe
or does not expose Softpipe through this GL path. It is not an independent
backend comparison yet.

## SwiftShader control

The ANGLE SwiftShader control produced stable WebGL1 and WebGL2 contexts, but
reported the expected SwiftShader renderer. It is useful as a control for
Chromium stability, not as the desired production renderer.

## Display backends

The host does not have `Xvfb`, `xvfb-run`, Weston, or Cage installed. The
available X11 probe therefore still used Chromium headless mode and did not
constitute a compositor-backed execution test. A Docker/Xvfb comparison needs
to be performed in a dedicated test image rather than inferred from this host.

## Phase 1 conclusion

The current CPU-only Mesa/ANGLE path is not stable enough for GPU identity
research. A native ANGLE identity override cannot be accepted until WebGL1 and
WebGL2 both survive repeated context creation without GPU-process crashes or
context loss.

The next research target is a minimal Docker image and a minimal Chromium
probe that can distinguish:

1. Mesa/LLVMpipe driver failure;
2. ANGLE OpenGL backend failure;
3. headless/Ozone display failure;
4. Chromium GPU-process sandbox or shared-memory failure;
5. container resource limits.
