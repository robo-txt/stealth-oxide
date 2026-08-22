# Tests and browser diagnostics

Deterministic public-API tests live under `tests/unit/` and compile as the
single `unit` integration-test target. They form part of the normal release
gate and do not launch a browser.

Environment-dependent browser probes live under `tests/triggers/` and compile as
the single `triggers` integration-test target. These are
diagnostics: they measure browser consistency and do not promise to bypass a
third-party security system. They are ignored by default because they may need
Chromium, network access, or the repository's desktop container.

Run one diagnostic in the container with:

```bash
  docker compose run --rm stealth-oxide \
  cargo test --test triggers triggers::headless_emulation -- --ignored --nocapture
```

Run all trigger diagnostics with:

```bash
docker compose run --rm stealth-oxide \
  cargo test --test triggers triggers:: -- --ignored --nocapture
```

These tests inspect browser APIs and environment surfaces. They are
evidence-gathering tools, not acceptance criteria for the crate.
