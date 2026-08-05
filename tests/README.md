# Browser diagnostics

The `creepjs_*` targets are diagnostic integration tests, not public usage
examples. They compile during normal `cargo test --all-targets` runs but are
ignored because they require Chromium, network access, and the repository's
desktop container.

Run one diagnostic in the container with:

```bash
docker compose run --rm stealth-oxide \
  cargo test --test creepjs_headless -- --ignored --nocapture
```

Run the complete CreepJS diagnostic group with:

```bash
docker compose run --rm stealth-oxide \
  cargo test creepjs_ -- --ignored --nocapture
```

These tests inspect a third-party website and can break when its markup changes.
They are evidence-gathering tools, not acceptance criteria for the crate.
