# Contributing

Rust 1.86 is the minimum supported toolchain. Use the latest stable toolchain
for formatting and Clippy.

Before submitting a change, run:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo test --doc --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
```

Browser diagnostics require Chromium and, for desktop environment checks, the
Docker Compose environment described in the README. Third-party fingerprinting
sites are diagnostics rather than product guarantees.

Changes to a built-in identity must keep the User-Agent, reduced Client Hints,
full Client Hints, and `ProfileVersion` metadata coherent. Public API changes
must explain their compatibility impact and update `CHANGELOG.md`.
