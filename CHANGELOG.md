# Changelog

All notable changes to this project will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.4.5] — 2026-04-20

### Changed

- **Phase 5 hoisting hardened for production compatibility.**
  - Implemented alpha-renaming for hoisted locals to prevent shadowing collisions
    in flattened state-machine scope.
  - Preserved cross-segment variable visibility while fixing repeated-visit
    initialization behavior for jump-driven loops.
  - Kept strict-mode diagnostics for both forward-goto hazard classes (Case A and
    Case B), including compile-fail UI coverage.
- **Regression coverage expanded for control-flow edge cases.**
  - Added integration tests for reinitialization-on-loop semantics.
  - Added integration tests for same-name local shadowing in labeled segments.
  - Added integration tests to ensure labeled-segment locals remain visible across
    subsequent segments.
  - Updated `trybuild` expectations for strict skipped-segment diagnostics.
- **Project release hardening baseline added.**
  - Added CI workflow with OS + toolchain matrix (stable + MSRV), formatting,
    linting, tests, and docs build.
  - Added dependency automation via Dependabot.
  - Added repository governance docs: contributing, security policy, code of
    conduct, and releasing guide.
  - Added issue templates and PR template for maintainable contribution flow.
  - Added repository editor defaults and macOS file hygiene ignore.
- **Crate metadata and documentation alignment.**
  - Added `homepage`, `documentation`, and `include` metadata fields in
    `Cargo.toml`.
  - Updated README usage/dependency examples and maintainer workflow references.
  - Corrected changelog compare/release links to the canonical repository.

---

## [0.4.0] — 2026-04-20

### Changed

- Production hardening pass:
  - added CI matrix (stable + MSRV, multi-OS),
  - added contribution/security/release governance docs,
  - added issue/PR templates and dependency update automation.
- Phase 5 hoisting behavior hardened to preserve compatibility while fixing
  shadowing-collision and re-initialization edge cases.

---

## [0.3.5] — 2026-04-20

### Changed

- **Crate renamed to `gotobykkrwhofrags`.** The package name on crates.io and the
  corresponding `use` path change from `goto::goto` to `gotobykkrwhofrags::goto`.
  No API or behaviour changes.

---

## [0.3.0] — 2026-04-20

### Fixed

- **Phase 5 hoisting — documented scan-past-expression-statements behaviour.**
  The Phase 5 loop continues past non-`let`, non-`goto!()` statements (expression
  statements), meaning a `let` that follows an expression but precedes the first
  `goto!()` in a segment is still hoisted. This was always the intended behaviour, but
  it was undocumented and the code lacked a comment, making the logic look like a bug.
  A clarifying comment has been added; the semantics are unchanged.

- **`has_side_effects` — whitelist known pure macros.**
  Previously every macro invocation in a `let` initializer triggered a strict-mode
  compile error, including `vec![]`, `matches!`, `concat!`, and `stringify!`. These
  macros have no observable side effects, so flagging them surprised users. They are now
  whitelisted; all other macro invocations remain conservatively treated as non-trivial.

- **`combine_errors` — avoid unnecessary clone.**
  `errors.iter()` required each `syn::Error` to clone its token stream inside
  `to_compile_error()`. Changed to `errors.into_iter()` so the owned value is consumed
  directly instead.

- **`GotoInClosureFinder` — rename inherent `visit_expr` to `check_expr`.**
  The struct had an inherent method named `visit_expr` and also implemented the
  `syn::visit::Visit` trait, which defines a method with the same name. Both compiled
  fine (the trait impl delegated to the inherent method), but the duplicate name was
  confusing. The inherent method is now named `check_expr`; the trait impl's
  `visit_expr` calls `self.check_expr(…)`.

- **Phase count in doc comment — updated to 8.**
  The `#[goto]` attribute's doc comment previously listed 7 transformation passes;
  the strict-mode phase (Phase 4) added in 0.2 brought the total to 8. The doc comment
  now correctly states 8 passes.

---

## [0.2.0]

### Added

- **`#[goto(debug)]`** — prepends `println!("jumping to {}", "<label>")` to every
  `goto!()` replacement at compile time, logging each state transition to stdout at
  runtime. Intended for development tracing; has no effect on function results or
  signature.
- **`#[goto(strict)]`** — promotes two classes of forward-goto side-effect hazard to
  compile errors:
  - *Case A*: a `let` binding with a non-trivial initializer (function call, method
    call, or macro invocation) appears *after* a forward `goto!()` in the same segment.
    The code is unreachable, but its presence is misleading.
  - *Case B*: a `let` binding with a non-trivial initializer appears in a labelled
    segment that can be bypassed entirely by a forward `goto!()`. Because that binding
    would be hoisted to function entry by the macro, its initializer runs
    unconditionally — even on code paths that never visit the segment.
- Attribute arguments are now parsed as a **comma-separated list**, so `debug` and
  `strict` can be combined: `#[goto(debug, strict)]`.
- Compile-fail test suite via `trybuild` covering both strict-mode error cases and
  confirming that valid strict-mode code continues to compile.

---

## [0.1.0] — 2026-04-20

### Added

- `#[goto]` attribute macro enabling `label!(name)` and `goto!(name)` in any function.
- Backward goto (loop) and forward goto (skip) patterns.
- Multiple-label dispatch table pattern.
- Support for `goto!()` inside `if`/`else` branches and `match` arms.
- Support for generic functions with type parameters and where clauses.
- Support for void functions (no return type).
- Support for `unsafe fn`.
- Compile-time error for undefined labels, with the label name in the message.
- Compile-time error for duplicate label names within a function.
- Compile-time error for `goto!()` inside a closure body, with a clear suggestion.
- Compile-time error for malformed `label!()` syntax (non-identifier argument).
- `unreachable!()` wildcard arm that includes the invalid state value in the panic message.
- `#![forbid(unsafe_code)]` — the macro implementation itself contains no unsafe code.
- Full crate metadata: license (MIT), MSRV (1.61), keywords, categories, authors.
- Comprehensive integration test suite covering all supported patterns.
- Full API documentation on docs.rs.

[Unreleased]: https://github.com/ckakkar/goto---Rust/compare/v0.4.5...HEAD
[0.4.5]: https://github.com/ckakkar/goto---Rust/compare/v0.4.2...v0.4.5
[0.4.1]: https://github.com/ckakkar/goto---Rust/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/ckakkar/goto---Rust/compare/v0.3.5...v0.4.0
[0.3.5]: https://github.com/ckakkar/goto---Rust/compare/v0.3.0...v0.3.5
[0.3.0]: https://github.com/ckakkar/goto---Rust/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/ckakkar/goto---Rust/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/ckakkar/goto---Rust/releases/tag/v0.1.0
