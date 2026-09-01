# Screen/GPU runtime lab

This is an isolated experiment area for the remaining CreepJS soft signals:

- `hasSwiftShader`
- `noTaskbar`
- `hasVvpScreenRes`

It intentionally does not modify the Rust library, profiles, patches, or
production tests. Runtime output is written below `artifacts/`, which is
ignored by Git.

## Sequence

Run these from the repository root:

```bash
cd dev/screen-gpu-lab
./capture-host.sh
./run-screen-info-probes.sh
./run-creepjs-matrix.sh
```

The first script records the host facts needed to interpret GPU and screen
results. The second uses Chromium's native `--dump-dom` path to compare the
legacy headless, Mesa/ANGLE, and native `--screen-info` configurations without
adding a CDP or JavaScript patch to stealth-oxide. The third runs the existing
diagnostic example against CreepJS for the current and Mesa cases.

The headful case is deliberately not automated by this first lab script: it
opens a visible browser window and must be run only after confirming the local
display/session details. The command is documented in the generated host
report and can be approved separately.

The CPU-only stability matrix is separate:

```bash
bash run-cpu-stability-matrix.sh
```

It records WebGL 1/2 availability, context-loss state, limits, renderer
strings, process exit status, and Chromium logs under `artifacts/cpu-stability/`.

## Native identity prototype

The optional shim experiment keeps LLVMpipe rendering active while replacing
only native `GL_VENDOR` and `GL_RENDERER` responses:

```bash
./build-gl-identity-shim.sh
```

Run Chromium with `LD_PRELOAD` and the same CPU-only Mesa environment, then
inspect the probe output. This is deliberately expected to be incomplete:
extensions, numeric limits, shader precision, and pixels are not changed. A
renderer string that changes while those surfaces remain LLVMpipe is a
regression/diagnostic result, not a successful GPU profile.

## Interpretation

`screen.availHeight` is compared with `screen.height`, while WebGL vendor and
renderer are compared across the page's main context and worker context. A
lower CreepJS `like headless` score is not by itself proof of a more realistic
browser; direct `webdriver`/HeadlessChrome signals, WebGL consistency, and
screen/viewport coherence must remain clean.
