# Goto

A procedural macro that brings classic C-style `goto` statements to Rust.

Rust intentionally omits `goto` by design, emphasizing structured control flow (`loop`, `while`, `for`, `match`). However, sometimes you might want to break those conventions for esoteric patterns, porting legacy code, or simply to experiment. The `goto` crate provides the `#[goto]` macro, enabling `label!(name)` and `goto!(name)` from within any function.

## How it works

Behind the scenes, the `#[goto]` attribute macro creatively rewrites the function body into a **state machine**.
1. **Splits into Segments**: It slices the function into numbered segments exactly at each `label!()` call.
2. **Variable Hoisting**: `let` bindings that appear before their segment's first goto are hoisted to the top so that variables remain in scope across `goto` jumps.
3. **State Transitions**: `goto!(name)` calls are replaced with a state update and a loop jump: `{ __goto_state = target_idx; continue 'goto_loop; }`.
4. **Execution Loop**: The final result is neatly wrapped inside a `loop` over a `match` statement covering all segments.

## Installation

Add this block to your `Cargo.toml` if it's referenced as a local path:

```toml
[dependencies]
goto = { path = "path/to/goto" }
```

*(Note: Adjust the path or version depending on how you distribute this crate).*

## Usage

Simply apply the `#[goto]` attribute to a function, and then use `label!(name)` and `goto!(name)` anywhere inside its body.

### 1. Backward `goto` (Loops)

You can build loops using backward gotos:

```rust
use goto::goto;

#[goto]
fn count_up(limit: i32) -> i32 {
    let mut n = 0;
    label!(top);
    n += 1;
    if n < limit {
        goto!(top); // Jumps back to `label!(top)`
    }
    n
}

assert_eq!(count_up(5), 5);
```

### 2. Forward `goto` (Skipping logic)

You can jump forward to easily skip certain logic branches:

```rust
use goto::goto;

#[goto]
fn skip_middle() -> Vec<&'static str> {
    let mut out = vec!["first"];
    goto!(end); // Jumps forward, skipping the middle push
    out.push("middle");
    
    label!(end);
    out.push("last");
    out
}

assert_eq!(skip_middle(), vec!["first", "last"]);
```

### 3. Multiple Labels (State Selection)

```rust
use goto::goto;

#[goto]
fn fizzbuzz_once(n: i32) -> &'static str {
    if n % 15 == 0 { goto!(fizzbuzz); }
    if n % 3 == 0  { goto!(fizz); }
    if n % 5 == 0  { goto!(buzz); }
    goto!(neither);

    label!(fizzbuzz);
    return "FizzBuzz";
    label!(fizz);
    return "Fizz";
    label!(buzz);
    return "Buzz";
    label!(neither);
    return "neither";
}
```

## Known Limitations
- Does not easily penetrate closures, as each closure requires its own `#[goto]` implementation scope.
- Overusing it might introduce pasta-like code structures (the famous "spaghetti code").
- Certain intricate borrows or references in variables that span across jumps might trigger complications with the rust borrow checker since the code is transformed into a `loop` and `match` branch structure.
