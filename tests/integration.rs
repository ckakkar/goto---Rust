use goto::goto;
extern crate trybuild;

// ── Basic fixtures ────────────────────────────────────────────────────────────

#[goto]
fn count_up(limit: i32) -> i32 {
    let mut n = 0;
    label!(top);
    n += 1;
    if n < limit {
        goto!(top);
    }
    n
}

#[goto]
fn skip_middle() -> Vec<&'static str> {
    let mut out = vec!["first"];
    goto!(end);
    out.push("middle");
    label!(end);
    out.push("last");
    out
}

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

// ── Void (no return value) ────────────────────────────────────────────────────

#[goto]
fn void_goto(out: &mut Vec<&'static str>) {
    out.push("a");
    goto!(end);
    out.push("b");
    label!(end);
    out.push("c");
}

// ── Goto inside if/else arms ──────────────────────────────────────────────────

#[goto]
fn nested_if_goto(x: i32, y: i32) -> &'static str {
    if x > 0 {
        if y > 0 { goto!(both_pos); }
        goto!(x_only);
    }
    goto!(neither);

    label!(both_pos); return "both positive";
    label!(x_only);   return "x positive only";
    label!(neither);  "neither positive"
}

// ── Goto inside match arms ────────────────────────────────────────────────────

#[goto]
fn match_goto(x: i32) -> &'static str {
    match x {
        0 => goto!(zero),
        1 => goto!(one),
        _ => {}
    }
    goto!(other);

    label!(zero);  return "zero";
    label!(one);   return "one";
    label!(other); "other"
}

// ── Generic function ──────────────────────────────────────────────────────────

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

// ── Multiple backward gotos ───────────────────────────────────────────────────

#[goto]
fn collatz_steps(mut n: u64) -> u64 {
    let mut steps = 0;
    label!(check);
    if n == 1 { goto!(done); }
    if n % 2 == 0 { goto!(even); }
    n = 3 * n + 1;
    steps += 1;
    goto!(check);

    label!(even);
    n /= 2;
    steps += 1;
    goto!(check);

    label!(done);
    steps
}

// ── Explicit function with return in every path ───────────────────────────────

#[goto]
fn sign(x: i32) -> &'static str {
    if x > 0 { goto!(pos); }
    if x < 0 { goto!(neg); }
    return "zero";

    label!(pos); return "positive";
    label!(neg); return "negative";
}

// ── Debug mode ───────────────────────────────────────────────────────────────

#[goto(debug)]
fn count_up_debug(limit: i32) -> i32 {
    let mut n = 0;
    label!(top);
    n += 1;
    if n < limit {
        goto!(top);
    }
    n
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[test]
fn test_backward_goto() {
    assert_eq!(count_up(5), 5);
    assert_eq!(count_up(1), 1);
    assert_eq!(count_up(0), 1); // increments before the condition check
}

#[test]
fn test_forward_goto() {
    assert_eq!(skip_middle(), vec!["first", "last"]);
}

#[test]
fn test_multiple_labels() {
    assert_eq!(fizzbuzz_once(15), "FizzBuzz");
    assert_eq!(fizzbuzz_once(9),  "Fizz");
    assert_eq!(fizzbuzz_once(10), "Buzz");
    assert_eq!(fizzbuzz_once(7),  "neither");
}

#[test]
fn test_void_function() {
    let mut v = vec![];
    void_goto(&mut v);
    assert_eq!(v, vec!["a", "c"]);
}

#[test]
fn test_nested_if_goto() {
    assert_eq!(nested_if_goto(1,  1),  "both positive");
    assert_eq!(nested_if_goto(1,  -1), "x positive only");
    assert_eq!(nested_if_goto(-1, 1),  "neither positive");
    assert_eq!(nested_if_goto(-1, -1), "neither positive");
}

#[test]
fn test_match_goto() {
    assert_eq!(match_goto(0), "zero");
    assert_eq!(match_goto(1), "one");
    assert_eq!(match_goto(2), "other");
    assert_eq!(match_goto(-1), "other");
}

#[test]
fn test_generic_goto() {
    assert!(linear_search(&[1, 2, 3], &2));
    assert!(!linear_search(&[1, 2, 3], &4));
    assert!(!linear_search::<i32>(&[], &1));
    assert!(linear_search(&["hello", "world"], &"world"));
}

#[test]
fn test_multiple_backward_gotos() {
    // Collatz: 6 → 3 → 10 → 5 → 16 → 8 → 4 → 2 → 1  (8 steps)
    assert_eq!(collatz_steps(6), 8);
    assert_eq!(collatz_steps(1), 0);
    assert_eq!(collatz_steps(2), 1);
}

#[test]
fn test_sign() {
    assert_eq!(sign(42),  "positive");
    assert_eq!(sign(-1),  "negative");
    assert_eq!(sign(0),   "zero");
}

#[test]
fn test_debug_mode() {
    // Verifies #[goto(debug)] still produces correct results (output goes to stdout).
    assert_eq!(count_up_debug(3), 3);
}

// ── Strict mode: valid code still compiles ────────────────────────────────────

#[goto(strict)]
fn strict_skip_safe() -> i32 {
    goto!(end);
    label!(end);
    42
}

#[goto(strict)]
fn strict_backward_ok(limit: i32) -> i32 {
    let mut n = 0;
    label!(top);
    n += 1;
    if n < limit { goto!(top); }
    n
}

#[test]
fn test_strict_valid_code() {
    assert_eq!(strict_skip_safe(), 42);
    assert_eq!(strict_backward_ok(4), 4);
}

// ── Strict mode: invalid patterns rejected at compile time ───────────────────

#[test]
fn test_strict_compile_errors() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/strict_after_forward_goto.rs");
    t.compile_fail("tests/ui/strict_skipped_segment.rs");
    t.pass("tests/ui/strict_ok.rs");
}
