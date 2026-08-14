# Releasing

## One-time setup for `0.1.0`

crates.io requires a crate's first version to be published with an API token;
trusted publishing can only be configured after the crate exists.

1. Sign in to crates.io with the maintainer GitHub account and verify its email.
2. Create a crates.io token restricted to publishing new crates.
3. In GitHub, create an environment named `release`, require the repository
   owner as its reviewer, prevent administrator bypass, restrict deployment to
   tags matching `v*`, and store the token as the environment secret
   `CARGO_REGISTRY_TOKEN`.
4. Never store the token as a repository-level secret or in a local file.

Immediately after `0.1.0` exists on crates.io, configure its trusted publisher:

- GitHub owner: `robo-txt`
- GitHub repository: `stealth-oxide`
- Workflow: `release.yml`
- Environment: `release`

Then replace token authentication in the workflow with
`rust-lang/crates-io-auth-action`, grant only `id-token: write`, delete the
GitHub secret, and revoke the bootstrap token on crates.io.

## Per-release procedure

1. Confirm the working tree is clean and CI is green.
2. Verify the compatibility policy and tested Chromium version.
3. Update `CHANGELOG.md`, the package version, and relevant profile metadata.
4. Run all feature combinations and the full validation suite.
5. Inspect `cargo package --list --locked` for unintended files or secrets.
6. Build a clean external consumer against the packaged source.
7. Run `cargo publish --dry-run --locked`.
8. Merge the release commit to `main` and verify Required CI.
9. Create a signed annotated tag and push only that tag:

   ```bash
   git switch main
   git pull --ff-only
   git tag -s v0.1.0 -m "stealth-oxide v0.1.0"
   git push origin v0.1.0
   ```

10. Approve the protected `release` environment deployment. The workflow
    publishes to crates.io and creates the matching GitHub release.
11. Verify crates.io, docs.rs, and installation by a clean external consumer.

Published crate versions are immutable. If a release is defective, yank it
with `cargo yank --version VERSION` and publish a corrected patch version;
never attempt to replace an existing version.

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
