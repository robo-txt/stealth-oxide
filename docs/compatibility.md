# Compatibility

This document separates modeled browser identities from environments exercised
by automated browser tests. A built-in profile does not transform the host
operating system, GPU, fonts, voices, or platform-only features.

## Support matrix

| Dimension | Policy |
| --- | --- |
| Rust | 1.86 minimum; latest stable is also tested |
| Chromiumoxide | 0.9.x, beginning with 0.9.1 |
| Built-in browser identity | Google Chrome 151.0.7922.71 |
| Runtime/profile matching | Same Chrome major version |
| Required browser-test host | Debian 12 Linux container |
| Required browser-test runtime | Chromium 151.x |

Cargo resolves `chromiumoxide = "0.9.1"` within the compatible 0.9 release
line. The lockfile is used for reproducible repository and release checks.

## Chromium versions

All built-in profiles currently model Chrome 151.0.7922.71. The public
`ProfileVersion` metadata and `compare_browser_versions` helper let an
application compare a profile with the `product` value returned by CDP
`Browser.getVersion`.

A same-major result is the supported configuration. Other Chromium majors may
work, but are not guaranteed. A custom identity has no version claim unless its
builder is given explicit version metadata.

The required container CI verifies that its installed runtime remains Chromium
151.x before running browser diagnostics. The Debian package is not pinned to a
full patch-version artifact, so a future package-repository update to another
major deliberately fails the compatibility gate until maintainers review and
update the profile, tests, and policy together.

## Host platforms

| Host or profile | Profile available | Unit tested | Native browser tested in required CI |
| --- | ---: | ---: | ---: |
| Linux | Yes | Yes | Yes |
| Windows | Yes | Yes | No |
| macOS | Yes | Yes | No |

Windows and macOS are modeled profiles, not claims that the crate's browser
integration suite runs on native Windows or macOS hosts. Until native CI jobs
exist, Linux is the only browser-tested host.

## Compatibility updates

- Profile identity data is reviewed when the tested Chromium major changes.
- User-Agent and User-Agent Client Hint versions are updated together.
- MSRV changes and profile identity changes are recorded in `CHANGELOG.md`.
- The MSRV will not increase in a patch release.
- Display text from errors is intended for people and is not a stable machine
  interface; consumers should use typed error variants.

Chromium Beta, Chromium-based Edge, ungoogled-chromium, and browser versions
outside the documented major are not currently guaranteed.
