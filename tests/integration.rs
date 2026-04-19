use goto::goto;

// Backward goto: acts like a loop.
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

// Forward goto: skips over code.
#[goto]
fn skip_middle() -> Vec<&'static str> {
    let mut out = vec!["first"];
    goto!(end);
    #[allow(unused)]
    let _never = out.push("middle"); // never executed
    label!(end);
    out.push("last");
    out
}

// Multiple labels and gotos.
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

#[test]
fn test_backward_goto() {
    assert_eq!(count_up(5), 5);
    assert_eq!(count_up(1), 1);
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
