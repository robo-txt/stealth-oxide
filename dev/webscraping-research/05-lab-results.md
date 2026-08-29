# Lab results

These results came from the isolated `dev/screen-gpu-lab/` experiments on
2026-08-24. They are host observations, not Docker production results.

## Host facts

The host has an actual AMD Renoir GPU and an active `amdgpu` driver. Native
Mesa/ANGLE therefore reports a real hardware renderer on the host. This is not
representative of a Docker container with no `/dev/dri` device.

## Native CPU Mesa baseline

With:

```text
MESA_LOADER_DRIVER_OVERRIDE=llvmpipe
LIBGL_ALWAYS_SOFTWARE=true
```

`glxinfo` reported:

```text
Vendor: Mesa
Device: llvmpipe (LLVM 22.1.8, 256 bits)
OpenGL vendor string: Mesa
OpenGL renderer string: llvmpipe (LLVM 22.1.8, 256 bits)
```

Chromium's probe reported the corresponding Mesa/LLVMpipe ANGLE identity.
Chromium's GPU process also exited with status 139 during the probe, even when
no identity shim or ANGLE identity override was present. This means forced
LLVMpipe stability is an independent issue that must be solved first.

## `LD_PRELOAD` GL shim

The isolated C shim was able to change `glxinfo` output to:

```text
OpenGL vendor string: AMD
OpenGL renderer string: AMD Radeon HD 3200 Graphics
```

It did not provide a safe Chromium solution. Chromium continued to expose
Mesa/LLVMpipe or crashed its GPU process. The in-process-GPU diagnostic also
segfaulted. This prototype should not be wired into the browser runtime.

Files:

- [`screen-gpu-lab/gl-identity-shim.c`](../screen-gpu-lab/gl-identity-shim.c)
- [`screen-gpu-lab/build-gl-identity-shim.sh`](../screen-gpu-lab/build-gl-identity-shim.sh)

## ANGLE identity override on accelerated host Mesa

With:

```text
ANGLE_GL_VENDOR=AMD
ANGLE_GL_RENDERER=AMD Radeon HD 3200 Graphics
```

and the host's normal accelerated Mesa path, Chromium returned:

```text
WebGL 1: AMD / AMD Radeon HD 3200 Graphics
WebGL 2: AMD / AMD Radeon HD 3200 Graphics
```

The browser remained stable. This is the strongest result so far and proves
the override is native to ANGLE rather than a page JavaScript lie.

## ANGLE identity override with forced LLVMpipe

The same variables reached Chromium's native context under forced LLVMpipe:
one run returned the requested AMD identity for WebGL 2. However, WebGL 1 was
unavailable and the GPU process exited with status 139. Because the forced
LLVMpipe baseline showed the same process failure, the result is inconclusive
until the CPU renderer startup is stabilized.

Overriding ANGLE's version string produced additional invalid GL queries and
capability failures. Version spoofing is therefore excluded from the next
research phase.

## Existing CreepJS observations

The current repository's Mesa/ANGLE CreepJS artifact showed:

```text
hasSwiftShader: false
hasVvpScreenRes: true
44% like headless
0% headless
0% stealth
```

That result was on the host's real AMD GPU, not CPU-only Docker. It cannot be
used as proof that the Docker target is fixed.

Artifacts:

- [`screen-gpu-lab/artifacts/creepjs-mesa-angle.json`](../screen-gpu-lab/artifacts/creepjs-mesa-angle.json)
- [`screen-gpu-lab/artifacts/host-facts.txt`](../screen-gpu-lab/artifacts/host-facts.txt)
