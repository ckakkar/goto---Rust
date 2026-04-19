use gotobykkrwhofrags::goto;

// Trivial inits (literals, paths) are always fine in strict mode.
#[goto(strict)]
fn ok(limit: i32) -> i32 {
    let mut n = 0;       // literal — trivial
    label!(top);
    n += 1;
    if n < limit { goto!(top); }
    n
}

// Forward goto whose skipped region has no let with a non-trivial init.
#[goto(strict)]
fn skip_safe() -> i32 {
    goto!(end);
    label!(end);
    42
}

fn main() {
    assert_eq!(ok(3), 3);
    assert_eq!(skip_safe(), 42);
}
