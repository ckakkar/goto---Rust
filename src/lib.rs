use proc_macro::TokenStream;
use proc_macro2::{Literal, TokenStream as TokenStream2};
use quote::quote;
use std::collections::HashMap;
use syn::{
    parse_macro_input,
    visit_mut::{self, VisitMut},
    Expr, ExprMacro, ExprReturn, Ident, ItemFn, ReturnType, Stmt, StmtMacro,
};

/// Enables `label!(name)` and `goto!(name)` inside a function.
///
/// The function body is rewritten as a state machine:
/// - The body is split into numbered segments at each `label!()` call.
/// - All `let` bindings are hoisted before the machine so variables remain
///   in scope across segment boundaries.
/// - `goto!(name)` sets the current state to the target segment's index
///   and loops back to the top of the machine.
/// - Tail expressions (implicit returns) are converted to explicit `return`.
///
/// # Example — backward goto (loop)
///
/// ```rust,no_run
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
/// ```
///
/// # Example — forward goto (skip)
///
/// ```rust,no_run
/// use goto::goto;
///
/// #[goto]
/// fn skip_middle() -> Vec<&'static str> {
///     let mut out = vec!["first"];
///     goto!(end);
///     out.push("middle");
///     label!(end);
///     out.push("last");
///     out
/// }
/// ```
#[proc_macro_attribute]
pub fn goto(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut func = parse_macro_input!(item as ItemFn);
    let stmts = std::mem::take(&mut func.block.stmts);

    // ── Phase 1: split at label!() boundaries ──────────────────────────────
    //
    // In syn 2.x, macro calls in statement position are parsed as Stmt::Macro,
    // not Stmt::Expr(Expr::Macro(...)).  extract_label handles this correctly.
    let mut segments: Vec<(Option<Ident>, Vec<Stmt>)> = Vec::new();
    let mut current_label: Option<Ident> = None;
    let mut current_stmts: Vec<Stmt> = Vec::new();

    for stmt in stmts {
        if let Some(label) = extract_label(&stmt) {
            segments.push((current_label.take(), std::mem::take(&mut current_stmts)));
            current_label = Some(label);
        } else {
            current_stmts.push(stmt);
        }
    }
    segments.push((current_label, current_stmts));

    // ── Phase 2: hoist let-bindings before the state machine ──────────────
    //
    // Only hoist lets that appear *before* the first top-level goto!() in
    // each segment.  Lets after a goto would be unreachable anyway, and
    // hoisting them would cause their initializers to run at startup — wrong
    // for forward-goto patterns like `goto!(end); let x = side_effect();`.
    let mut hoisted: Vec<Stmt> = Vec::new();
    for (_, stmts) in &mut segments {
        let mut i = 0;
        while i < stmts.len() {
            match &stmts[i] {
                Stmt::Local(_) => {
                    hoisted.push(stmts.remove(i));
                    // don't increment; the next element shifted into position i
                }
                Stmt::Macro(m) if m.mac.path.is_ident("goto") => break,
                _ => {
                    i += 1;
                }
            }
        }
    }

    // ── Phase 3: map label names → segment indices ──────────────────────────
    let label_indices: HashMap<String, usize> = segments
        .iter()
        .enumerate()
        .filter_map(|(i, (label, _))| label.as_ref().map(|l| (l.to_string(), i)))
        .collect();

    // ── Phase 4: replace goto!() with state transitions ────────────────────
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
        let errors: TokenStream2 = replacer.errors.iter().map(|e| e.to_compile_error()).collect();
        return errors.into();
    }

    // ── Phase 5: convert tail expressions to explicit returns ──────────────
    let returns_value = !matches!(func.sig.output, ReturnType::Default);
    for stmts in &mut transformed {
        if let Some(Stmt::Expr(expr, None)) = stmts.last_mut() {
            if returns_value {
                let cloned = expr.clone();
                *stmts.last_mut().unwrap() = Stmt::Expr(
                    Expr::Return(ExprReturn {
                        attrs: Vec::new(),
                        return_token: Default::default(),
                        expr: Some(Box::new(cloned)),
                    }),
                    Some(Default::default()),
                );
            } else if let Some(Stmt::Expr(_, semi @ None)) = stmts.last_mut() {
                *semi = Some(Default::default());
            }
        }
    }

    // ── Phase 6: build match arms ──────────────────────────────────────────
    //
    // Every arm ends with `continue 'goto_loop` so the arm type is `!`.
    // The wildcard uses `unreachable!()` (also `!`) so the entire loop is `!`,
    // which coerces to the function's return type without a mismatch.
    let arms: Vec<TokenStream2> = transformed
        .iter()
        .enumerate()
        .map(|(i, stmts)| {
            let idx = Literal::usize_suffixed(i);
            let next = Literal::usize_suffixed(i + 1);
            quote! {
                #idx => {
                    #(#stmts)*
                    __goto_state = #next;
                    continue 'goto_loop;
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
                    _ => unreachable!("invalid goto state — this is a bug in the goto macro"),
                }
            }
        }
    });

    quote! { #func }.into()
}

// ── AST visitor: replace goto!(name) with a state transition ───────────────

struct GotoReplacer<'a> {
    label_indices: &'a HashMap<String, usize>,
    errors: Vec<syn::Error>,
}

impl VisitMut for GotoReplacer<'_> {
    // Handle goto!() in statement position (Stmt::Macro in syn 2.x).
    fn visit_stmt_mut(&mut self, stmt: &mut Stmt) {
        if let Stmt::Macro(StmtMacro { mac, .. }) = stmt {
            if mac.path.is_ident("goto") {
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
                return;
            }
        }
        visit_mut::visit_stmt_mut(self, stmt);
    }

    // Handle goto!() in expression position (e.g. inside `if` bodies).
    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        let goto_mac = if let Expr::Macro(ExprMacro { mac, .. }) = &*expr {
            if mac.path.is_ident("goto") { Some(mac.clone()) } else { None }
        } else {
            None
        };

        if let Some(mac) = goto_mac {
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
            return;
        }

        visit_mut::visit_expr_mut(self, expr);
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn extract_label(stmt: &Stmt) -> Option<Ident> {
    // In syn 2.x, macro calls in statement position are Stmt::Macro.
    if let Stmt::Macro(StmtMacro { mac, .. }) = stmt {
        if mac.path.is_ident("label") {
            return mac.parse_body().ok();
        }
    }
    None
}
