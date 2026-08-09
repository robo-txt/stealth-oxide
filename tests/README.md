# Tests and browser diagnostics

Deterministic public-API tests live under `tests/unit/` and compile as the
single `unit` integration-test target. They form part of the normal release
gate and do not launch a browser.

Environment-dependent browser probes live under `tests/bypass/` and compile as
the single `bypass` integration-test target. Despite the folder name, these are
diagnostics: they measure browser consistency and do not promise to bypass a
third-party security system. They are ignored by default because they may need
Chromium, network access, or the repository's desktop container.

Run one diagnostic in the container with:

```bash
docker compose run --rm stealth-oxide \
  cargo test --test bypass bypass::creepjs_headless -- --ignored --nocapture
```

Run the complete CreepJS diagnostic group with:

```bash
docker compose run --rm stealth-oxide \
  cargo test --test bypass creepjs_ -- --ignored --nocapture
```

These tests inspect a third-party website and can break when its markup changes.
They are evidence-gathering tools, not acceptance criteria for the crate.
