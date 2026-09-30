use super::{Symbol, SymbolByteRange};
use crate::model::TextRange;
use quote::ToTokens;
use std::collections::BTreeMap;
use syn::{spanned::Spanned, visit::Visit};

pub(super) struct Function {
    pub symbol: Symbol,
    signature: String,
    body: String,
    statements: Vec<String>,
}

#[derive(Default)]
struct Functions {
    functions: Vec<Function>,
    containers: Vec<String>,
    exceeded: bool,
}

impl Functions {
    fn push(
        &mut self,
        signature: &syn::Signature,
        block: &syn::Block,
        span: proc_macro2::Span,
        kind: &str,
    ) {
        if self.functions.len() >= 512 || block.stmts.len() > 2048 {
            self.exceeded = true;
            return;
        }
        let mut anonymous = signature.clone();
        anonymous.ident = syn::Ident::new("__mapped_function", signature.ident.span());
        self.functions.push(Function {
            symbol: Symbol {
                language: "rust".into(),
                kind: kind.into(),
                name: signature.ident.to_string(),
                container: self.containers.clone(),
                range: TextRange {
                    start_line: span.start().line,
                    end_line: span.end().line,
                },
                byte_range: Some(SymbolByteRange {
                    start: span.byte_range().start,
                    end: span.byte_range().end,
                }),
            },
            signature: anonymous.into_token_stream().to_string(),
            body: block.to_token_stream().to_string(),
            statements: block
                .stmts
                .iter()
                .map(|statement| statement.to_token_stream().to_string())
                .collect(),
        });
    }
}

impl<'ast> Visit<'ast> for Functions {
    fn visit_item_mod(&mut self, module: &'ast syn::ItemMod) {
        self.containers.push(format!("mod {}", module.ident));
        syn::visit::visit_item_mod(self, module);
        self.containers.pop();
    }

    fn visit_item_impl(&mut self, implementation: &'ast syn::ItemImpl) {
        let mut container = format!("impl {}", implementation.self_ty.to_token_stream());
        if let Some((_, implemented_trait, _)) = &implementation.trait_ {
            container = format!(
                "impl {} for {}",
                implemented_trait.to_token_stream(),
                implementation.self_ty.to_token_stream()
            );
        }
        self.containers.push(container);
        syn::visit::visit_item_impl(self, implementation);
        self.containers.pop();
    }

    fn visit_item_trait(&mut self, item: &'ast syn::ItemTrait) {
        self.containers.push(format!("trait {}", item.ident));
        syn::visit::visit_item_trait(self, item);
        self.containers.pop();
    }

    fn visit_item_fn(&mut self, function: &'ast syn::ItemFn) {
        self.push(&function.sig, &function.block, function.span(), "function");
        self.containers.push(format!("fn {}", function.sig.ident));
        syn::visit::visit_item_fn(self, function);
        self.containers.pop();
    }

    fn visit_impl_item_fn(&mut self, function: &'ast syn::ImplItemFn) {
        self.push(&function.sig, &function.block, function.span(), "method");
        self.containers.push(format!("fn {}", function.sig.ident));
        syn::visit::visit_impl_item_fn(self, function);
        self.containers.pop();
    }

    fn visit_trait_item_fn(&mut self, function: &'ast syn::TraitItemFn) {
        if let Some(block) = &function.default {
            self.push(&function.sig, block, function.span(), "default_method");
        }
        self.containers.push(format!("fn {}", function.sig.ident));
        syn::visit::visit_trait_item_fn(self, function);
        self.containers.pop();
    }
}

pub(super) enum ParseError {
    Syntax,
    Limit,
}

pub(super) fn functions(source: &str) -> Result<Vec<Function>, ParseError> {
    let parsed = syn::parse_file(source).map_err(|_| ParseError::Syntax)?;
    let mut visitor = Functions::default();
    visitor.visit_file(&parsed);
    if visitor.exceeded {
        Err(ParseError::Limit)
    } else {
        Ok(visitor.functions)
    }
}

fn contains(outer: &TextRange, inner: &TextRange) -> bool {
    outer.start_line <= inner.start_line && outer.end_line >= inner.end_line
}

fn shared_statements(source: &Function, destination: &Function) -> usize {
    let mut counts = BTreeMap::new();
    for statement in &source.statements {
        *counts.entry(statement).or_insert(0_usize) += 1;
    }
    let mut shared = 0_usize;
    for statement in &destination.statements {
        if let Some(count) = counts.get_mut(statement)
            && *count > 0
        {
            *count -= 1;
            shared = shared.saturating_add(1);
        }
    }
    shared
}

pub(super) fn correspondences<'a>(
    source: &'a [Function],
    destination: &'a [Function],
    original: &TextRange,
) -> (Vec<(&'a Function, &'a Function, bool, usize)>, bool) {
    let mut candidates = Vec::new();
    let mut comparisons = 0_usize;
    let exact = source
        .iter()
        .any(|function| function.symbol.range == *original);
    let enclosing_lines = source
        .iter()
        .filter(|function| contains(&function.symbol.range, original))
        .map(|function| {
            function
                .symbol
                .range
                .end_line
                .saturating_sub(function.symbol.range.start_line)
        })
        .min();
    for from in source.iter().filter(|function| {
        if exact {
            return function.symbol.range == *original;
        }
        if let Some(lines) = enclosing_lines {
            return contains(&function.symbol.range, original)
                && function
                    .symbol
                    .range
                    .end_line
                    .saturating_sub(function.symbol.range.start_line)
                    == lines;
        }
        contains(original, &function.symbol.range)
    }) {
        for to in destination {
            comparisons = comparisons.saturating_add(1);
            if comparisons > 4096 || candidates.len() > 100 {
                return (candidates, true);
            }
            let equivalent = from.signature == to.signature && from.body == to.body;
            let shared = shared_statements(from, to);
            let partial = shared > 0
                && (shared.saturating_mul(3) >= from.statements.len()
                    || shared.saturating_mul(3) >= to.statements.len());
            if equivalent || partial {
                candidates.push((from, to, equivalent, shared));
            }
        }
    }
    (candidates, false)
}
