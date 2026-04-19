use goto::goto;

// Case A: non-trivial `let` appearing after a forward `goto!()` in the same segment.
#[goto(strict)]
fn bad(x: i32) -> i32 {
    goto!(end);
    let _y = expensive(); //~ ERROR this initializer appears after a forward `goto!()`
    label!(end);
    x
}

fn expensive() -> i32 { 42 }

fn main() {}
