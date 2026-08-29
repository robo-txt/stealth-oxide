# CreepJS Docker validation with StealthConfig

Date: 2026-08-26

This is the apples-to-apples follow-up to
[the raw Docker validation](11-creepjs-docker-validation.md). Chromium ran in
the disposable Xvfb container with CPU Mesa/LLVMpipe and native ANGLE GPU
identity. The Rust CDP client then applied the Linux `StealthConfig` and
`TargetCoordinator` before navigating to CreepJS.

## Evidence

- [Full CreepJS screenshot](../screen-gpu-lab/artifacts/docker-xvfb/creepjs-docker-angle-profile.png)
- [Extracted result](../screen-gpu-lab/artifacts/docker-xvfb/creepjs-docker-angle-profile.json)
- [Probe log](../screen-gpu-lab/artifacts/docker-xvfb/creepjs-docker-angle-profile.log)

## Score comparison

| Run | Like headless | Headless | Stealth |
|---|---:|---:|---:|
| Raw Docker Chromium | 44% | 67% | 0% |
| Docker Chromium + StealthConfig | 38% | 33% | 0% |

The score improvement confirms that the profile was applied. The raw run's
`67% headless` was not a GPU regression; it was the expected result of using
the unpatched HeadlessChrome identity.

## GPU result

The StealthConfig run continued to report:

```text
hasSwiftShader: false
WebGL GPU confidence: high
WebGL GPU grade: A
GPU classification: AMD Radeon HD 3000s Graphics
Worker GPU: AMD Radeon HD 3200 Graphics
WebGL lied: false
```

The screenshot shows the AMD GPU in both the WebGL and Worker panels. The
CPU-only rendering path therefore remained stable after applying the profile.

## Remaining signals

CreepJS still reported:

```text
chromium: true
hasHeadlessUA: false
hasHeadlessWorkerUA: true
noTaskbar: true
navigator lied: true
```

The page UA was changed to the profile's normal `Chrome/151` form, but the
Worker panel still displayed a `HeadlessChrome/151` UA. This is the clearest
remaining worker inconsistency in this run. The screen override also reports
1920x1080 available work area, so this test did not yet model a taskbar/work
area inset.

## Conclusion

The Docker architecture is validated for CPU rendering and coherent page/
worker GPU identity. StealthConfig improves, but does not eliminate, CreepJS
headless detection. The next implementation/research gate is worker UA
propagation—especially service-worker coverage—followed by a deliberate
screen/work-area profile. No custom Chromium build is justified by the GPU
results alone.
