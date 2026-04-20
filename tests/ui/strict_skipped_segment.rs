use gotobykkrwhofrags::goto;

// Non-trivial `let` in a skipped labeled segment is now valid:
// only entry-segment locals are hoisted.
#[goto(strict)]
fn bad() -> i32 {
    goto!(end);

    label!(middle);
    let _conn = expensive();
    goto!(end);

    label!(end);
    0
}

fn expensive() -> i32 { 42 }

fn main() {}
