# Research-only next steps

No implementation should be started until these questions are answered.

## Phase 1: stabilize CPU-only Chromium

Compare the following independently:

1. LLVMpipe through Mesa/ANGLE OpenGL.
2. Softpipe through Mesa/ANGLE OpenGL.
3. ANGLE SwiftShader, only as a control case.
4. Headless Ozone versus X11/Xvfb compositor-backed execution.
5. Chromium GPU sandbox enabled versus the minimum diagnostic relaxation.

For each case record GPU-process exit status, WebGL 1 availability, WebGL 2
availability, context-loss events, and `about:gpu` output.

The host forced-LLVMpipe crash is reproducible in the minimal probe and is
documented in [Phase 1 CPU stability results](07-phase1-cpu-stability.md).
The Docker follow-up found a separate display-layer boundary: this Chromium
build's ANGLE OpenGL path failed without an X display, but became stable when
run against Xvfb. See [Docker/Xvfb research](08-docker-xvfb.md).

The working research baseline is now:

```text
Docker without /dev/dri
  Xvfb display
  Mesa LLVMpipe renderer
  ANGLE OpenGL path
  optional ANGLE identity profile
```

This advances the work past the initial CPU-stability block, but only for a
minimal page WebGL probe. It does not yet establish CreepJS or worker
consistency.

## Phase 2: native identity only

Phase 2 can now proceed in the disposable Xvfb container. The first Docker
run showed that native ANGLE identity variables can coexist with stable CPU
LLVMpipe WebGL 1/2 contexts. Treat this as an experiment, not production
support, until the capability and worker gates below pass.

Use only:

```text
ANGLE_GL_VENDOR
ANGLE_GL_RENDERER
```

Preserve ANGLE's native version, extensions, limits, shader precision, and
pixel behavior. Test a renderer string that matches the native ANGLE format
expected by Chromium, while treating the result as experimental because the
ANGLE variables are documented as debugging/testing controls.

## Phase 3: capability coherence

Compare the selected profile against a reference browser/GPU and decide which
values are genuinely compatible:

- WebGL 1/2 support;
- extensions;
- max texture size and related limits;
- shader precision;
- compressed texture formats;
- readback pixel hashes;
- canvas output;
- WebGPU adapter values;
- GPU process and `SystemInfo` metadata.

Do not override a value merely because it belongs to the reference GPU. A
capability that is advertised but not implemented is a stronger contradiction.

## Phase 4: worker matrix

The first dedicated-worker navigator pass is complete. Page and worker values
match for the main profile fields, but raw `navigator.webdriver` differs
(`false` in the page and absent/null in the worker). SharedWorker and
service-worker responses did not complete in the dump-dom harness, so this is
not yet a pass/fail result for those target types.

The first CDP target pass is now complete. Dedicated and shared workers
returned stable WebGL with the same AMD identity as the page. A service-worker
target was observed, but its session disappeared before it could be resumed or
measured. Next, keep the service worker alive and measure target lifecycle and
session attachment directly through CDP. Compare all GPU fields and CreepJS
worker panels. This remains a separate acceptance gate from the main-page
renderer result.

## Phase 5: Docker validation

The first Docker validation pass is complete for the minimal page probe. The
next pass must use the actual browser launcher and record:

- whether `/dev/dri` is absent;
- Mesa and ANGLE versions;
- Chromium version and command line;
- Ozone/display backend;
- GPU-process status;
- CreepJS screenshot and extracted scores;
- native WebGL and worker matrix.

The CDP target pass is now the required Docker measurement path for worker
targets. The `--dump-dom` probe remains useful only for quick smoke tests.

The first full CreepJS Docker validation is complete. CPU Mesa rendering plus
the ANGLE identity profile produced AMD WebGL/worker identity, no SwiftShader,
and WebGL grade A, but CreepJS still reported 44% like-headless and 67%
headless because the run retained HeadlessChrome and 800x600 geometry. The
next pass must combine this GPU setup with native screen/work-area settings.

The current evidence says Xvfb is required for the tested ANGLE OpenGL path;
Docker alone and `--ozone-platform=headless` did not provide the needed EGL
display in this image. Do not add a physical GPU solely to solve that failure
until a compositor-backed CPU path has been measured and rejected.

## Decision gate

Proceed to a Chromium/ANGLE build patch only if:

- CPU-only WebGL is stable;
- ANGLE native identity is visible;
- page and workers are consistent;
- CreepJS no longer sees SwiftShader/Mesa/LLVMpipe contradictions;
- extensions, limits, precision, and pixels remain coherent.

If those conditions fail, a custom Chromium build is not automatically the
answer; the failure must first identify whether the missing layer is ANGLE
capability modeling, Mesa driver behavior, or Chromium GPU metadata.
