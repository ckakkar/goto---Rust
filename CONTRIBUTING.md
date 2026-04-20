# Contributing

Thanks for contributing to `gotobykkrwhofrags`.

## Development setup

1. Install Rust (MSRV is 1.61, development is usually done on stable).
2. Clone the repository.
3. Run:

```bash
cargo check --all-targets
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
```

## Testing expectations

- Add or update tests for behavior changes.
- For compile diagnostics, use `tests/ui` and `trybuild`.
- Keep tests minimal and targeted.

## Pull requests

- Keep changes focused and easy to review.
- Update docs and changelog when behavior changes.
- Ensure CI is green before requesting review.

## Commit style

- Use clear, imperative commit messages.
- Prefer messages that explain intent, not only mechanics.

## Versioning and release notes

- This crate follows SemVer.
- Breaking behavior changes require a major version bump.
