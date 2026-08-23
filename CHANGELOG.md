# Changelog

All notable user-visible changes to this project are documented here.

## [Unreleased]

- Added native hardware-concurrency, permission, and geolocation controls.
- Added validation and live-trigger coverage for browser-context permissions
  and page geolocation overrides.

## [0.1.2] - 2026-08-22

### Changed

- Simplified the published package and refreshed the GitHub README around the
  crate's supported browser-configuration workflows.
- Renamed browser diagnostics under `tests/triggers/` and removed evaluation-
  only report fixtures from the package.

### Removed

- Repository-only container tooling and documentation assets from the release
  workflow and published package.

## [0.1.1] - 2026-08-16

### Added

- Public built-in Chrome version metadata and runtime comparison helpers.
- An explicit Rust, Chromiumoxide, Chromium, and host-platform compatibility
  policy.
- A bounded, read-only `NetworkAuditHandle` for CDP network diagnostics.

### Changed

- Updated the built-in profile and required container runtime to Chrome
  151.0.7922.138.
- Pinned and checksum-verified the Chrome for Testing archive and added
  retries for transient downloads.
- Updated the README, compatibility policy, and examples for the supported
  runtime and package workflow.

### Removed

- Evaluation-only examples and stale challenge-detector references from the
  published package.

## [0.1.0] - 2026-08-13

### Added

- Typed Linux, macOS, and Windows desktop browser profiles.
- Configurable CDP identity, locale, timezone, screen, media, and touch patches.
- Optional profile seeding and request-header policy features.
- Consistency validation and structured partial-application reporting.
