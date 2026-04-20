# gotobykkrwhofrags

[![Crates.io](https://img.shields.io/crates/v/gotobykkrwhofrags.svg)](https://crates.io/crates/gotobykkrwhofrags)
[![Docs.rs](https://docs.rs/gotobykkrwhofrags/badge.svg)](https://docs.rs/gotobykkrwhofrags)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![MSRV: 1.61](https://img.shields.io/badge/MSRV-1.61-orange.svg)](https://blog.rust-lang.org/2022/05/19/Rust-1.61.0.html)

> C-style `goto` for Rust — safe, zero-`unsafe`, compile-time rewritten.

Apply `#[goto]` to any function and use `label!(name)` / `goto!(name)` inside its body.
The macro desugars entirely at compile time into a state-machine loop. There is no
`unsafe` code, no runtime overhead beyond the loop itself, and no linker hacks.

---

## Table of Contents

- [Installation](#installation)
- [Quick Start](#quick-start)
- [How It Works](#how-it-works)
- [Usage Guide](#usage-guide)
  - [Backward goto — loops](#backward-goto--loops)
  - [Forward goto — skipping code](#forward-goto--skipping-code)
  - [Multiple labels — dispatch tables](#multiple-labels--dispatch-tables)
  - [Goto in if/else and match](#goto-in-ifelse-and-match)
  - [Generic functions](#generic-functions)
  - [Debug mode](#debug-mode)
  - [Strict mode](#strict-mode)
- [API Reference](#api-reference)
  - [`#[goto]`](#goto-1)
  - [`label!(name)`](#labelname)
  - [`goto!(name)`](#gotoname)
- [Compile Errors](#compile-errors)
- [Known Limitations](#known-limitations)
- [Compatibility](#compatibility)
- [Performance](#performance)
- [License](#license)

---

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
gotobykkrwhofrags = "0.3.8"
```

---

## Quick Start

```rust
use gotobykkrwhofrags::goto;

#[goto]
fn count_up(limit: i32) -> i32 {
    let mut n = 0;
    label!(top);
    n += 1;
    if n < limit { goto!(top); }
    n
}

assert_eq!(count_up(5), 5);
```

---

## How It Works

`#[goto]` is a **compile-time source rewrite** — the function body you write never
executes as written. The macro performs eight transformation passes:

### Pass 1 — Segment splitting

The body is split into numbered segments at every `label!()` call. Segment 0 is the
code before the first label; segment *N* starts at the *N*th label.

```
fn foo() {           │  Segment 0: [stmt_a, stmt_b]
    stmt_a;          │  Segment 1: [stmt_c]          ← label!(here)
    stmt_b;          │  Segment 2: [stmt_d, stmt_e]  ← label!(there)
    label!(here);    │
    stmt_c;          │
    label!(there);   │
    stmt_d;          │
    stmt_e;          │
}
```

### Pass 2 — Duplicate label detection

Two `label!(foo)` calls in the same function produce a compile error pinpointing the
duplicate.

### Pass 3 — Label-to-index mapping

Each label name is mapped to its segment index in a compile-time `HashMap`.

### Pass 4 — Strict-mode hazard detection *(optional)*

When `#[goto(strict)]` is active, the macro scans for two classes of forward-goto
side-effect hazard and emits compile errors for each. See [Strict mode](#strict-mode).

### Pass 5 — Variable hoisting

`let` bindings that appear before the first `goto!()` in each segment are lifted above
the state machine so they remain in scope across all segments. Bindings that appear
*after* a `goto!()` are left in place (their initializers would never run anyway).

The scan continues past non-`let`, non-`goto!()` statements (expression statements), so
a `let` that appears *after* an expression statement but *before* the first `goto!()` in
the same segment is still hoisted. The expression runs normally inside its state-machine
arm; the variable must be visible to later segments, so hoisting is required.

### Pass 6 — `goto!()` replacement

Every `goto!(name)` becomes `{ __goto_state = N; continue 'goto_loop; }`, where `N`
is the segment index for `name`. Undefined labels produce a compile error at the
`goto!()` site.

When `#[goto(debug)]` is active, a `println!("jumping to {}", "name")` is prepended
to each replacement so every jump is logged at runtime.

### Pass 7 — Tail expression conversion

Implicit return expressions (tail expressions without a semicolon) are converted to
explicit `return` statements so they remain valid inside a `match` arm.

### Pass 8 — Code generation

The transformed segments are assembled into:

```rust
fn your_function(/* … */) -> T {
    /* hoisted let bindings */
    let mut __goto_state: usize = 0;
    'goto_loop: loop {
        match __goto_state {
            0 => { /* segment 0 */ __goto_state = 1; continue 'goto_loop; }
            1 => { /* segment 1 */ __goto_state = 2; continue 'goto_loop; }
            /* … */
            _ => unreachable!("invalid goto state {} — …", __goto_state),
        }
    }
}
```

---

## Usage Guide

### Backward goto — loops

Build loops using a backward jump:

```rust
use gotobykkrwhofrags::goto;

#[goto]
fn sum_to(n: i32) -> i32 {
    let mut total = 0;
    let mut i = 1;
    label!(accumulate);
    total += i;
    i += 1;
    if i <= n { goto!(accumulate); }
    total
}

assert_eq!(sum_to(4), 10); // 1 + 2 + 3 + 4
```

### Forward goto — skipping code

Jump forward to skip over a block:

```rust
use gotobykkrwhofrags::goto;

#[goto]
fn skip_middle() -> Vec<&'static str> {
    let mut out = vec!["first"];
    goto!(end);
    out.push("middle"); // never reached
    label!(end);
    out.push("last");
    out
}

assert_eq!(skip_middle(), vec!["first", "last"]);
```

> **Note:** `let` bindings that appear *before* a forward `goto!()` in the same segment
> are hoisted to function entry and will run even on the skipped path. See
> [Known Limitations](#known-limitations) and [Strict mode](#strict-mode).

### Multiple labels — dispatch tables

Multiple labels create a dispatch table — a common pattern in hand-written interpreters,
state machines, and ports of legacy code:

```rust
use gotobykkrwhofrags::goto;

#[goto]
fn fizzbuzz_once(n: i32) -> &'static str {
    if n % 15 == 0 { goto!(fizzbuzz); }
    if n % 3  == 0 { goto!(fizz); }
    if n % 5  == 0 { goto!(buzz); }
    goto!(neither);

    label!(fizzbuzz); return "FizzBuzz";
    label!(fizz);     return "Fizz";
    label!(buzz);     return "Buzz";
    label!(neither);  return "neither";
}
```

### Goto in if/else and match

`goto!()` works anywhere an expression is valid, including inside `if`/`else` branches
and `match` arms:

```rust
use gotobykkrwhofrags::goto;

#[goto]
fn classify(x: i32) -> &'static str {
    if x > 0 { goto!(positive); }
    if x < 0 { goto!(negative); }
    return "zero";

    label!(positive); return "positive";
    label!(negative); return "negative";
}
```

```rust
use gotobykkrwhofrags::goto;

#[goto]
fn from_code(code: u8) -> &'static str {
    match code {
        0 => goto!(ok),
        1 => goto!(err),
        _ => goto!(unknown),
    }

    label!(ok);      return "OK";
    label!(err);     return "Error";
    label!(unknown); "Unknown"
}
```

### Generic functions

`#[goto]` composes naturally with generic parameters and where clauses:

```rust
use gotobykkrwhofrags::goto;

#[goto]
fn linear_search<T: PartialEq>(haystack: &[T], needle: &T) -> bool {
    let mut i = 0;
    label!(check);
    if i >= haystack.len() { goto!(not_found); }
    if &haystack[i] == needle { goto!(found); }
    i += 1;
    goto!(check);

    label!(found);     return true;
    label!(not_found); false
}
```

### Debug mode

`#[goto(debug)]` prints `jumping to <label>` to stdout on every `goto!()` invocation.
The function still produces the same result; only a side-channel log is added. This is
useful for tracing control flow during development without modifying the function body.

```rust
use gotobykkrwhofrags::goto;

#[goto(debug)]
fn collatz_steps(mut n: u64) -> u64 {
    let mut steps = 0;
    label!(check);
    if n == 1 { goto!(done); }          // prints "jumping to done"
    if n % 2 == 0 { goto!(even); }     // prints "jumping to even"
    n = 3 * n + 1;
    steps += 1;
    goto!(check);                        // prints "jumping to check"

    label!(even);
    n /= 2;
    steps += 1;
    goto!(check);                        // prints "jumping to check"

    label!(done);
    steps
}
```

Remove the `debug` argument before shipping — it is intended for local development only.

### Strict mode

`#[goto(strict)]` turns forward-goto side-effect hazards into **compile errors**. This
catches two patterns that silently misbehave under the default mode:

#### Case A — initializer after a forward goto

A `let` binding with a non-trivial initializer that appears *after* a forward `goto!()`
in the same segment is unreachable, but its presence is misleading and almost certainly
a bug:

```rust
// Does not compile with #[goto(strict)]:
#[goto(strict)]
fn example(x: i32) -> i32 {
    goto!(end);
    let _y = expensive_io(); // ERROR: unreachable after a forward goto!()
    label!(end);
    x
}
```

Fix: move the `let` after `label!(end)`, or remove it.

#### Case B — hoisted initializer in a bypassed segment

When a segment is jumped over entirely, its hoistable `let` bindings are lifted to
function entry by the macro and run unconditionally — even on the path that bypasses
that segment:

```rust
// Does not compile with #[goto(strict)]:
#[goto(strict)]
fn example() -> i32 {
    goto!(end);

    label!(middle);
    let _conn = open_db(); // ERROR: hoisted, runs at function entry even when skipped
    goto!(end);

    label!(end);
    0
}
```

Fix: move `let _conn = open_db()` to after `label!(end)`, or restructure so `open_db()`
is only called on paths that actually use `_conn`.

An initializer is considered *non-trivial* (and thus flagged) if it contains any
function call, method call, or macro invocation — **except** for known pure macros
(`vec![]`, `matches!`, `concat!`, `stringify!`), which are always accepted. Plain
integer or string literals and simple variable paths are also always accepted.

```rust
// Allowed — vec![] is whitelisted as a pure macro:
let v = vec![1, 2, 3];

// Flagged — open_db() is a function call with observable side effects:
let _conn = open_db();
```

#### Combining modes

`debug` and `strict` can be used together:

```rust
#[goto(debug, strict)]
fn my_fn() { /* … */ }
```

---

## API Reference

### `#[goto]`

```
#[goto]
#[goto(debug)]
#[goto(strict)]
#[goto(debug, strict)]
fn your_function(/* params */) -> ReturnType {
    /* body */
}
```

Attribute macro. Apply to any `fn` item — regular, `unsafe`, or generic. Rewrites the
entire function body into a state machine. The rewrite is transparent to the caller;
the function signature is unchanged.

**Arguments** (comma-separated, all optional):

| Argument | Effect |
|----------|--------|
| *(none)* | Standard behaviour. |
| `debug`  | Emits `println!("jumping to {}", "<label>")` before each state transition at runtime. Intended for development; remove before release. |
| `strict` | Promotes forward-goto side-effect hazards to compile errors. Recommended for any code where correctness under forward jumps matters. |

### `label!(name)`

```
label!(identifier);
```

Defines a jump target named `identifier`. Must appear as a **statement** (not inside an
expression position). Each name must be unique within the `#[goto]` function.

Valid: `label!(my_loop);`  
Invalid: `let x = label!(foo);` — labels cannot appear in expression position.

### `goto!(name)`

```
goto!(identifier);
```

Unconditionally transfers control to the named label. May appear in any position where
an **expression** is valid, including inside `if`, `else`, and `match` arms. The target
label must be defined somewhere in the same `#[goto]` function.

---

## Compile Errors

The macro produces clear compile-time diagnostics for all misuse:

| Situation | Error |
|-----------|-------|
| `goto!(undefined)` | ``undefined label: `undefined` `` |
| Two `label!(foo)` in same function | ``duplicate label: `foo` `` |
| `goto!()` inside a closure | `` `goto!()` inside a closure is not supported `` |
| Malformed `label!(123)` | `invalid label!() syntax: expected an identifier` |
| Unknown attribute argument | ``unknown attribute `xyz` — expected `debug` or `strict` `` |
| `strict`: non-trivial `let` after forward `goto!()` | `this initializer appears after a forward goto!() and will never run` |
| `strict`: hoisted non-trivial `let` in bypassed segment | `this initializer would be hoisted to function entry and run unconditionally` |

---

## Known Limitations

### Closures

`goto!()` inside a closure body is **not supported** and produces a compile error. The
label lives in the outer function's state machine, which the closure cannot reach.

**Workaround:** restructure the code so the jump happens in the outer function, or
extract the logic into a named inner function decorated with `#[goto]`.

### Variable hoisting and initializer side effects

When a forward `goto!()` jumps past `let` bindings, those bindings' initializers still
run at function entry (they were hoisted to keep the variables in scope). If an
initializer has observable side effects (I/O, allocation, etc.) and is placed after a
`goto!()`, it will execute even when the surrounding code is skipped.

To avoid this:

- Use `#[goto(strict)]` — it turns both hoisting hazard patterns into compile errors.
- Use uninitialised declarations (`let x;`) and assign inside each segment where needed.
- Restructure so side-effecting initialisers appear before any `goto!()`.

### Borrow checker conflicts

Because all segments live inside the same `loop`/`match`, the borrow checker treats
every segment as potentially reachable from every other. A borrow that is safe in
sequential code may be rejected if it crosses a label boundary. The usual fix is to
introduce explicit `{}` scoping or to clone the value.

### `label!()` inside nested blocks

`label!()` must appear at the **top level** of the function body. A `label!()` inside
an `if`, `match`, `loop`, or closure block is not recognized as a segment boundary and
will be left as an unresolved macro, causing a compile error.

### `const fn`

The generated state machine uses `loop` and `match`, which are valid in `const fn`
context since Rust 1.46, but this combination has not been systematically tested.
Use in `const fn` is at your own risk.

---

## Compatibility

| Feature | Status |
|---------|--------|
| Regular `fn` | ✅ Supported |
| `unsafe fn` | ✅ Supported |
| Generic parameters and where clauses | ✅ Supported |
| `goto!()` in `if`/`else` | ✅ Supported |
| `goto!()` in `match` arm | ✅ Supported |
| Void functions (no return type) | ✅ Supported |
| `#[goto(debug)]` | ✅ Supported |
| `#[goto(strict)]` | ✅ Supported |
| `#[goto(debug, strict)]` | ✅ Supported |
| `async fn` | ⚠️ Untested |
| `const fn` | ⚠️ Untested |
| `goto!()` inside a closure | ❌ Compile error (by design) |
| `label!()` inside a nested block | ❌ Compile error |
| Minimum Rust version (MSRV) | **1.61** |

---

## Performance

The state machine the macro generates is intentionally simple. In straightforward cases
— a single backward goto with no cross-segment variable sharing, for example — LLVM
optimises it to the same assembly as an equivalent hand-written `loop`. In more complex
cases with many labels and wide `match` arms, there may be a small overhead; profile if
it matters.

`#[goto(debug)]` adds one `println!` call per `goto!()` invocation. This involves a
heap allocation and a write syscall per jump; do not use it in performance-sensitive
paths or production builds.

The compile-time cost of the macro itself is proportional to the size of the function
body and is negligible in practice.

---

## License

MIT © 2026 Cyrus Kakkar. See [LICENSE](LICENSE) for full text.
