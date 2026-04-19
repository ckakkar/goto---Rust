# Changelog

All notable changes to this project will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

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

[Unreleased]: https://github.com/Yujiro/goto/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/Yujiro/goto/releases/tag/v0.1.0
