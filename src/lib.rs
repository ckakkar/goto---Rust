#![forbid(unsafe_code)]

//! A procedural macro that brings C-style `goto` to Rust.
//!
//! Apply [`goto`] to any function and use `label!(name)` and `goto!(name)` inside its body.
//! The macro rewrites the function at compile time into a state-machine loop — no `unsafe`,
//! no runtime overhead beyond the loop itself.
//!
//! # Quick start
//!
//! ```rust
//! use goto::goto;
//!
//! #[goto]
//! fn count_up(limit: i32) -> i32 {
//!     let mut n = 0;
//!     label!(top);
//!     n += 1;
//!     if n < limit { goto!(top); }
//!     n
//! }
//!
//! assert_eq!(count_up(5), 5);
//! ```

use proc_macro::TokenStream;
use proc_macro2::{Literal, Span, TokenStream as TokenStream2};
use quote::quote;
use std::collections::HashMap;
use syn::{
    parse_macro_input,
    visit_mut::{self, VisitMut},
    Expr, ExprClosure, ExprMacro, ExprReturn, Ident, ItemFn, ReturnType, Stmt, StmtMacro,
};

/// Enables `label!(name)` and `goto!(name)` inside a function.
///
/// The function body is rewritten as a state machine at compile time:
///
/// 1. The body is split into numbered segments at each `label!()` call.
/// 2. `let` bindings before the first `goto!()` in each segment are hoisted above the
///    state machine so variables remain in scope across segment boundaries.
/// 3. Duplicate label names produce a compile error.
/// 4. Each label is assigned the index of its segment.
/// 5. `goto!(name)` is replaced with `{ __goto_state = N; continue 'goto_loop; }`.
/// 6. The result is wrapped in `'goto_loop: loop { match __goto_state { … } }`.
///
/// # Errors
///
/// The macro emits a **compile error** when:
/// - A `goto!()` references an undefined label.
/// - The same label name appears more than once in the function.
/// - A `goto!()` or `label!()` call has invalid syntax (e.g. a non-identifier argument).
/// - A `goto!()` appears inside a closure body.
///
/// # Example — backward goto (loop)
///
/// ```rust
/// use goto::goto;
///
/// #[goto]
/// fn count_up(limit: i32) -> i32 {
///     let mut n = 0;
///     label!(top);
///     n += 1;
///     if n < limit { goto!(top); }
///     n
/// }
///
/// assert_eq!(count_up(5), 5);
/// ```
///
/// # Example — forward goto (skip)
///
/// ```rust
/// use goto::goto;
///
/// #[goto]
/// fn skip_middle() -> Vec<&'static str> {
///     let mut out = vec!["first"];
///     goto!(end);
///     out.push("middle"); // never reached
///     label!(end);
///     out.push("last");
///     out
/// }
///
/// assert_eq!(skip_middle(), vec!["first", "last"]);
/// ```
///
/// # Example — multiple labels (dispatch table)
///
/// ```rust
/// use goto::goto;
///
/// #[goto]
/// fn fizzbuzz_once(n: i32) -> &'static str {
///     if n % 15 == 0 { goto!(fizzbuzz); }
///     if n % 3  == 0 { goto!(fizz); }
///     if n % 5  == 0 { goto!(buzz); }
///     goto!(neither);
///
///     label!(fizzbuzz); return "FizzBuzz";
///     label!(fizz);     return "Fizz";
///     label!(buzz);     return "Buzz";
///     label!(neither);  return "neither";
/// }
/// ```
#[proc_macro_attribute]
pub fn goto(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut func = parse_macro_input!(item as ItemFn);
    let stmts = std::mem::take(&mut func.block.stmts);

    // ── Phase 1: split at label!() boundaries ─────────────────────────────────
    //
    // In syn 2.x, macro calls in statement position are Stmt::Macro, not
    // Stmt::Expr(Expr::Macro(…)).  extract_label handles both forms correctly
    // and propagates parse errors for malformed label!() invocations.
    let mut segments: Vec<(Option<Ident>, Vec<Stmt>)> = Vec::new();
    let mut current_label: Option<Ident> = None;
    let mut current_stmts: Vec<Stmt> = Vec::new();
    let mut phase1_errors: Vec<syn::Error> = Vec::new();

    for stmt in stmts {
        match extract_label(&stmt) {
            ExtractResult::Label(label) => {
                segments.push((current_label.take(), std::mem::take(&mut current_stmts)));
                current_label = Some(label);
            }
            ExtractResult::Error(e) => phase1_errors.push(e),
            ExtractResult::NotALabel => current_stmts.push(stmt),
        }
    }
    segments.push((current_label, current_stmts));

    if !phase1_errors.is_empty() {
        return combine_errors(phase1_errors);
    }

    // ── Phase 2: hoist let-bindings before the state machine ──────────────────
    //
    // Only hoists lets that appear *before* the first top-level goto!() in each
    // segment.  Lets after a goto would be unreachable, and hoisting their
    // initializers would cause side effects to run at function entry — wrong for
    // forward-goto patterns like `goto!(end); let x = side_effect();`.
    let mut hoisted: Vec<Stmt> = Vec::new();
    for (_, stmts) in &mut segments {
        let mut i = 0;
        while i < stmts.len() {
            match &stmts[i] {
                Stmt::Local(_) => {
                    hoisted.push(stmts.remove(i));
                }
                Stmt::Macro(m) if m.mac.path.is_ident("goto") => break,
                _ => i += 1,
            }
        }
    }

    // ── Phase 3: detect duplicate labels ──────────────────────────────────────
    let mut seen: HashMap<String, Span> = HashMap::new();
    let mut dup_errors: Vec<syn::Error> = Vec::new();
    for (label_opt, _) in &segments {
        if let Some(label) = label_opt {
            let name = label.to_string();
            if seen.contains_key(&name) {
                dup_errors.push(syn::Error::new(
                    label.span(),
                    format!("duplicate label: `{name}` — each label must be unique within a #[goto] function"),
                ));
            } else {
                seen.insert(name, label.span());
            }
        }
    }
    if !dup_errors.is_empty() {
        return combine_errors(dup_errors);
    }

    // ── Phase 4: map label names → segment indices ────────────────────────────
    let label_indices: HashMap<String, usize> = segments
        .iter()
        .enumerate()
        .filter_map(|(i, (label, _))| label.as_ref().map(|l| (l.to_string(), i)))
        .collect();

    // ── Phase 5: replace goto!() with state transitions ───────────────────────
    let mut replacer = GotoReplacer { label_indices: &label_indices, errors: Vec::new() };
    let mut transformed: Vec<Vec<Stmt>> = segments
        .into_iter()
        .map(|(_, mut stmts)| {
            for stmt in &mut stmts {
                replacer.visit_stmt_mut(stmt);
            }
            stmts
        })
        .collect();

    if !replacer.errors.is_empty() {
        return combine_errors(replacer.errors);
    }

    // ── Phase 6: convert tail expressions to explicit returns ─────────────────
    let returns_value = !matches!(func.sig.output, ReturnType::Default);
    for stmts in &mut transformed {
        if let Some(last) = stmts.last_mut() {
            if let Stmt::Expr(expr, None) = last {
                if returns_value {
                    let cloned = expr.clone();
                    *last = Stmt::Expr(
                        Expr::Return(ExprReturn {
                            attrs: Vec::new(),
                            return_token: Default::default(),
                            expr: Some(Box::new(cloned)),
                        }),
                        Some(Default::default()),
                    );
                } else {
                    if let Stmt::Expr(_, semi @ None) = last {
                        *semi = Some(Default::default());
                    }
                }
            }
        }
    }

    // ── Phase 7: build match arms ─────────────────────────────────────────────
    //
    // Every arm must diverge (type `!`) so the match arms are type-compatible.
    // For value-returning functions, Phase 6 ensures the last segment ends with
    // an explicit `return`, so the trailing `continue` is unreachable but
    // satisfies the type checker.  For void functions the last segment has no
    // implicit terminator, so we emit `return;` there instead of advancing to a
    // nonexistent state N.
    let n_segments = transformed.len();
    let arms: Vec<TokenStream2> = transformed
        .iter()
        .enumerate()
        .map(|(i, stmts)| {
            let idx = Literal::usize_suffixed(i);
            let terminator = if i == n_segments - 1 && !returns_value {
                quote! { return; }
            } else {
                let next = Literal::usize_suffixed(i + 1);
                quote! { __goto_state = #next; continue 'goto_loop; }
            };
            quote! {
                #idx => {
                    #(#stmts)*
                    #terminator
                }
            }
        })
        .collect();

    func.attrs
        .push(syn::parse_quote!(#[allow(unreachable_code, unused_assignments)]));

    func.block = Box::new(syn::parse_quote! {
        {
            #(#hoisted)*
            let mut __goto_state: usize = 0usize;
            'goto_loop: loop {
                match __goto_state {
                    #(#arms,)*
                    _ => unreachable!(
                        "invalid goto state {} — this is a bug in the goto macro",
                        __goto_state
                    ),
                }
            }
        }
    });

    quote! { #func }.into()
}

// ── AST visitor: replace goto!(name) with a state transition ──────────────────

struct GotoReplacer<'a> {
    label_indices: &'a HashMap<String, usize>,
    errors: Vec<syn::Error>,
}

impl VisitMut for GotoReplacer<'_> {
    // Handle goto!() in statement position (Stmt::Macro in syn 2.x).
    fn visit_stmt_mut(&mut self, stmt: &mut Stmt) {
        if let Stmt::Macro(StmtMacro { mac, .. }) = stmt {
            if mac.path.is_ident("goto") {
                self.replace_goto_stmt(stmt);
                return;
            }
        }
        visit_mut::visit_stmt_mut(self, stmt);
    }

    // Handle goto!() in expression position (e.g. inside `if` bodies).
    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        if let Expr::Macro(ExprMacro { mac, .. }) = &*expr {
            if mac.path.is_ident("goto") {
                self.replace_goto_expr(expr);
                return;
            }
        }
        visit_mut::visit_expr_mut(self, expr);
    }

    // Stop descent into closures.  A goto!() inside a closure would reference
    // 'goto_loop from the outer function, producing a confusing compile error.
    // We detect this case explicitly and emit a clear diagnostic instead.
    fn visit_expr_closure_mut(&mut self, closure: &mut ExprClosure) {
        let mut finder = GotoInClosureFinder { span: None };
        finder.visit_expr(&closure.body);
        if let Some(span) = finder.span {
            self.errors.push(syn::Error::new(
                span,
                "`goto!()` inside a closure is not supported — \
                 apply `#[goto]` to a named inner function instead",
            ));
        }
        // Do NOT recurse — leave the closure body as-is.
    }
}

impl GotoReplacer<'_> {
    fn replace_goto_stmt(&mut self, stmt: &mut Stmt) {
        let mac = match stmt {
            Stmt::Macro(StmtMacro { mac, .. }) => mac.clone(),
            _ => return,
        };
        match mac.parse_body::<Ident>() {
            Ok(label) => match self.label_indices.get(&label.to_string()).copied() {
                Some(idx) => {
                    let idx_lit = Literal::usize_suffixed(idx);
                    *stmt = syn::parse_quote! {
                        { __goto_state = #idx_lit; continue 'goto_loop };
                    };
                }
                None => self.errors.push(syn::Error::new_spanned(
                    &label,
                    format!("undefined label: `{label}`"),
                )),
            },
            Err(e) => self.errors.push(e),
        }
    }

    fn replace_goto_expr(&mut self, expr: &mut Expr) {
        let mac = match expr {
            Expr::Macro(ExprMacro { mac, .. }) => mac.clone(),
            _ => return,
        };
        match mac.parse_body::<Ident>() {
            Ok(label) => match self.label_indices.get(&label.to_string()).copied() {
                Some(idx) => {
                    let idx_lit = Literal::usize_suffixed(idx);
                    *expr = syn::parse_quote! {
                        { __goto_state = #idx_lit; continue 'goto_loop }
                    };
                }
                None => self.errors.push(syn::Error::new_spanned(
                    &label,
                    format!("undefined label: `{label}`"),
                )),
            },
            Err(e) => self.errors.push(e),
        }
    }
}

// ── Immutable visitor: find goto!() inside a closure body ─────────────────────

struct GotoInClosureFinder {
    span: Option<Span>,
}

impl GotoInClosureFinder {
    fn visit_expr(&mut self, expr: &Expr) {
        if self.span.is_some() {
            return;
        }
        match expr {
            Expr::Macro(ExprMacro { mac, .. }) if mac.path.is_ident("goto") => {
                self.span = mac.path.get_ident().map(|i| i.span());
            }
            // Stop at nested closure boundaries — they have their own scope.
            Expr::Closure(_) => {}
            other => syn::visit::visit_expr(self, other),
        }
    }
}

impl<'ast> syn::visit::Visit<'ast> for GotoInClosureFinder {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        self.visit_expr(expr);
    }

    fn visit_stmt(&mut self, stmt: &'ast Stmt) {
        if self.span.is_some() {
            return;
        }
        if let Stmt::Macro(StmtMacro { mac, .. }) = stmt {
            if mac.path.is_ident("goto") {
                self.span = mac.path.get_ident().map(|i| i.span());
                return;
            }
        }
        syn::visit::visit_stmt(self, stmt);
    }

    fn visit_expr_closure(&mut self, _: &'ast ExprClosure) {
        // Stop at nested closures.
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

enum ExtractResult {
    Label(Ident),
    Error(syn::Error),
    NotALabel,
}

fn extract_label(stmt: &Stmt) -> ExtractResult {
    if let Stmt::Macro(StmtMacro { mac, .. }) = stmt {
        if mac.path.is_ident("label") {
            return match mac.parse_body::<Ident>() {
                Ok(ident) => ExtractResult::Label(ident),
                Err(e) => ExtractResult::Error(syn::Error::new(
                    e.span(),
                    format!("invalid label!() syntax: {e} — expected an identifier, e.g. `label!(my_label)`"),
                )),
            };
        }
    }
    ExtractResult::NotALabel
}

fn combine_errors(errors: Vec<syn::Error>) -> TokenStream {
    let ts: TokenStream2 = errors.iter().map(|e| e.to_compile_error()).collect();
    ts.into()
}
