# Runtime lab results

Captured 2026-08-24 on the Omarchy host. Artifacts are in the ignored
`artifacts/` directory.

## Host

- Wayland session under Hyprland, with Xwayland `DISPLAY=:0` available.
- Primary monitor: 1920×1080, scale 1.25.
- GPU: AMD Renoir, `amdgpu`, accelerated Mesa 26.1.5.
- Render node: `/dev/dri/renderD128`.
- Chromium: 150.0.7871.128.
- Docker: available, rootful Docker 29.6.2.
- Xvfb, `xvfb-run`, Openbox, Weston, Cage, and `xdpyinfo` are not installed.

## Chromium-native probe

| Case | Page screen | Page work area | WebGL renderer | Result |
|---|---:|---:|---|---|
| Legacy headless | 800×600 | 800×600 | SwiftShader | baseline headless geometry and software GPU |
| Legacy headless + Mesa/ANGLE | 800×600 | 800×600 | AMD Radeon via ANGLE/OpenGL ES | GPU changes, geometry does not |
| Headless + `--screen-info` | 1920×1080 | 1920×1040 | SwiftShader | native work-area geometry works |
| Modern headless + `--screen-info` | 1920×1080 | 1920×1040 | SwiftShader | same geometry result |
| Modern headless + `--screen-info` + Mesa/ANGLE | 1920×1080 | 1920×1040 | AMD Radeon via ANGLE/OpenGL ES | both target signals change successfully |

The final row is the strongest candidate runtime baseline for a future
implementation experiment. It uses only Chromium launch controls and native
screen configuration; it does not patch page JavaScript.

## Existing stealth-oxide CreepJS comparison

| Case | `like headless` | Direct headless | Stealth | SwiftShader | Remaining soft signals |
|---|---:|---:|---:|---:|---|
| Current legacy headless | 50% | 0% | 0% | true | viewport/screen equality and no-taskbar |
| Existing Mesa/ANGLE opt-in | 44% | 0% | 0% | false | viewport/screen equality and no-taskbar |

Both cases had no context mismatches and no reported fingerprint lies.

## Docker result

Exposing only `/dev/dri/renderD128` allowed the container to start Chromium and
honor `--screen-info`, but WebGL was unavailable. Mounting the host Xwayland
socket did not make ANGLE usable; Chromium still reported that it could not
open the X display. Docker therefore needs a deliberate display/driver image
setup and is not currently the lowest-risk path.

## Headful regression

The existing ignored headful screen regression was run against the real
Hyprland display with native-screen mode. It failed at the assertion that
`visualViewport.width` and `innerWidth` differ by no more than one CSS pixel.
This indicates a real scale/viewport coordination issue on the 1.25-scaled
host and must be investigated before treating headful as a reference baseline.

## Proposed next phase — approval required

1. Add a lab-only CDP/Chromium launch experiment for modern headless with
   native `--screen-info` and Mesa/ANGLE flags.
2. Feed the resulting runtime into the existing CreepJS diagnostic and capture
   JSON plus screenshot evidence.
3. Investigate the headful 1.25-scale viewport mismatch using a lab probe,
   without relaxing the regression assertion.
4. Defer Xvfb installation and Docker image work unless the host experiment
   cannot provide a stable result.
5. Only after those results, decide whether the smallest change belongs in
   `BrowserConfig` launch arguments, native CDP screen coordination, or neither.
