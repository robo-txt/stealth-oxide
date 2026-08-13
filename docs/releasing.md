# Releasing

1. Confirm the working tree is clean and CI is green.
2. Verify the compatibility policy and tested Chromium version.
3. Update `CHANGELOG.md`, the package version, and relevant profile metadata.
4. Run all feature combinations and the full validation suite.
5. Inspect `cargo package --list --locked` for unintended files or secrets.
6. Build a clean external consumer against the packaged source.
7. Run `cargo publish --dry-run --locked`.
8. Commit the release, create a tag, publish, and verify crates.io and docs.rs.

Release validation commands:

```bash
cargo fmt --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets --all-features
cargo test --locked --doc --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps --all-features
cargo package --list --locked
cargo publish --dry-run --locked
```
