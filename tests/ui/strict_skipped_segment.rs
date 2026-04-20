use gotobykkrwhofrags::goto;

// Case B: non-trivial `let` in a segment entirely skipped by a forward goto.
// The hoisting would make expensive() run at function entry unconditionally.
#[goto(strict)]
fn bad() -> i32 {
    goto!(end);

    label!(middle);
    let _conn = expensive(); //~ ERROR would be hoisted to function entry
    goto!(end);

    label!(end);
    0
}

fn expensive() -> i32 { 42 }

fn main() {}
