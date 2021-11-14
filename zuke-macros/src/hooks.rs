//! Registers before/after hook functions, and parses tag expressions
use crate::utils::make_call;
use pest::iterators::Pair;
use pest::prec_climber::{Assoc, Operator, PrecClimber};
use pest::Parser;
use pest_derive::Parser;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, quote_spanned};

/// Parses a tag expression
#[derive(Parser)]
#[grammar = "tag_expr.pest"]
struct TagExprParser;

/// Function to turn tokens into a Vec<Operation>, in stack order
fn consume(pair: Pair<'_, Rule>, climber: &PrecClimber<Rule>) -> TokenStream2 {
    let primary = |pair| consume(pair, climber);

    let infix = |lhs: TokenStream2, op: Pair<Rule>, rhs: TokenStream2| {
        let op = match op.as_rule() {
            Rule::and => quote! { ::zuke::hooks::Operation::And },
            Rule::or => quote! { ::zuke::hooks::Operation::Or },
            _ => unreachable!(),
        };

        quote! {
            #lhs,
            #rhs,
            #op
        }
    };

    match pair.as_rule() {
        Rule::tag => {
            let tag = &pair.as_str()[1..];
            quote! { ::zuke::hooks::Operation::Push(#tag.into()) }
        }
        Rule::tag_u => {
            let tag = &pair.as_str()[2..];
            quote! { ::zuke::hooks::Operation::PushUninherited(#tag.into()) }
        }
        Rule::invert => {
            let ops = climber.climb(pair.into_inner(), primary, infix);
            quote! {
                #ops,
                ::zuke::hooks::Operation::Not,
            }
        }
        _ => climber.climb(pair.into_inner(), primary, infix),
    }
}

/// Turn a tag expression into a sequence of operations
fn build_expr(expr: syn::LitStr) -> TokenStream2 {
    let climber = PrecClimber::new(vec![
        Operator::new(Rule::or, Assoc::Left),
        Operator::new(Rule::and, Assoc::Right),
    ]);

    let s = expr.value();
    let mut tag_expr = match TagExprParser::parse(Rule::tag_expr, &s) {
        Ok(expr) => expr,
        Err(e) => {
            let msg = e.to_string();
            let span = expr.span();
            return quote_spanned! {span=> compile_error!(#msg)};
        }
    };

    let expr = tag_expr.next().unwrap();
    consume(expr, &climber)
}

pub enum Kind {
    Global,
    Feature,
    Rule,
    Scenario,
    Step,
    Any,
}

/// get tag expr from attribute
fn get_tag_expr(input: TokenStream) -> syn::Result<Option<syn::LitStr>> {
    if syn::parse::<syn::parse::Nothing>(input.clone()).is_ok() {
        Ok(None)
    } else {
        syn::parse::<syn::LitStr>(input).map(Some)
    }
}

/// Register a before or after hook
pub fn register_before_after(
    args: TokenStream,
    input: TokenStream,
    before: bool,
    kind: Kind,
) -> TokenStream {
    let expr = match get_tag_expr(args) {
        Ok(s) => s,
        Err(e) => return e.into_compile_error().into(),
    };

    let func = syn::parse_macro_input!(input as syn::ItemFn);
    let func_name = &func.sig.ident;
    let func_call = quote! { #func_name(context) };
    let func_call = make_call(func_call, &func, false, true);

    let expr = match expr {
        None => quote! {},
        Some(s) => build_expr(s),
    };

    let when = if before {
        quote! { ::zuke::hooks::BeforeAfter::Before }
    } else {
        quote! { ::zuke::hooks::BeforeAfter::After }
    };

    let kind = match kind {
        Kind::Global => vec![quote! { ::zuke::ComponentKind::Global }],
        Kind::Feature => vec![quote! { ::zuke::ComponentKind::Feature }],
        Kind::Rule => vec![quote! { ::zuke::ComponentKind::Rule }],
        Kind::Scenario => vec![quote! { ::zuke::ComponentKind::Scenario }],
        Kind::Step => vec![quote! { ::zuke::ComponentKind::Step }],
        Kind::Any => vec![
            // intentionally doesn't include step
            quote! { ::zuke::ComponentKind::Global },
            quote! { ::zuke::ComponentKind::Feature },
            quote! { ::zuke::ComponentKind::Rule },
            quote! { ::zuke::ComponentKind::Scenario },
        ],
    };

    (quote! {
        #func

        const _: () = {
            use ::zuke::reexport::inventory;
            use ::zuke::reexport::futures::future::{BoxFuture, FutureExt};

            #(
                inventory::submit! {
                    ::zuke::hooks::BeforeAfterHook {
                        when: #when,
                        kind: #kind,
                        func: |context| async move { #func_call }.boxed(),
                        expr: vec![#expr],
                    }
                }
            )*
        };
    })
    .into()
}
