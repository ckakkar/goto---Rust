# Releasing

This document describes the release process for `gotobykkrwhofrags`.

## Pre-release checklist

1. Ensure `CHANGELOG.md` has an entry for the new version.
2. Update `Cargo.toml` version.
3. Verify links and examples in `README.md`.
4. Run:

```bash
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
cargo package
```

5. Check package contents:

```bash
cargo package --list
```

## Publish

```bash
cargo publish
```

## Post-release

1. Tag release in git (`vX.Y.Z`).
2. Push tag.
3. Add release notes from `CHANGELOG.md`.
