use crate::utils::make_call;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, quote_spanned};
use regex::Regex;
use std::collections::HashSet;
use syn::parse::Error as ParseError;
use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PatternType {
    Expression,
    Regex,
}

pub enum StepKeyword {
    Any, // Given|When|Then
    Raw, // expression matches given/when/then on its own.
    Given,
    When,
    Then,
}

pub struct StepArgs {
    pub pattern_span: Span,
    pub pattern: String,
    pub pattern_type: PatternType,
}

impl StepArgs {
    fn expand_pattern(&mut self) -> Result<()> {
        if self.pattern_type == PatternType::Regex {
            return Ok(());
        }

        enum State {
            Scanning,
            Escaped,
            Ident,
            MatchPattern,
        }

        let mut new_regex = String::new();
        let mut state = State::Scanning;
        let mut start = 0;

        for (i, c) in self.pattern.chars().enumerate() {
            state = match state {
                State::Scanning => match c {
                    '\\' => State::Escaped,
                    '{' => {
                        new_regex.push_str(&regex::escape(&self.pattern[start..i]));
                        start = i + 1;
                        State::Ident
                    }
                    _ => State::Scanning,
                },
                State::Escaped => State::Scanning,
                State::Ident => {
                    if c == '}' || c == ':' {
                        let ident = &self.pattern[start..i];
                        start = i + 1;
                        if ident.is_empty() {
                            return Err(syn::Error::new(
                                self.pattern_span,
                                "Capture must have a name",
                            ));
                        }

                        new_regex.push_str("(?P<");
                        new_regex.push_str(ident);
                        new_regex.push('>');
                        if c == '}' {
                            new_regex.push_str(".*)");
                            State::Scanning
                        } else {
                            State::MatchPattern
                        }
                    } else if c != '_' && !c.is_ascii_alphanumeric() {
                        // capture groups are restricted to ascii alphanumeric
                        return Err(syn::Error::new(
                            self.pattern_span,
                            format!("Illegal character in capture name '{:?}'", c),
                        ));
                    } else {
                        State::Ident
                    }
                }
                State::MatchPattern => {
                    if c == '}' {
                        let pattern = &self.pattern[start..i];
                        start = i + 1;
                        if pattern.is_empty() {
                            return Err(syn::Error::new(
                                self.pattern_span,
                                "Empty capture pattern",
                            ));
                        }

                        new_regex.push_str(pattern);
                        new_regex.push(')');
                        State::Scanning
                    } else {
                        State::MatchPattern
                    }
                }
            };
        }

        match state {
            State::Scanning | State::Escaped => {
                new_regex.push_str(&regex::escape(&self.pattern[start..]));
                self.pattern = new_regex;
                self.pattern_type = PatternType::Regex;
                Ok(())
            }
            _ => Err(syn::Error::new(self.pattern_span, "Unterminated '{'")),
        }
    }
}

impl Parse for StepArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut pattern_span = None;
        let mut pattern = None;
        let mut pattern_type = PatternType::Expression;
        let args = Punctuated::<syn::NestedMeta, syn::Token![,]>::parse_terminated(input)?;

        for arg in args {
            match arg {
                syn::NestedMeta::Lit(syn::Lit::Str(s)) => {
                    // A literal string: the pattern
                    if pattern.is_some() {
                        return Err(ParseError::new(s.span(), "Redefinition of pattern"));
                    } else {
                        pattern = Some(s.value());
                        pattern_span = Some(s.span());
                    }
                }
                syn::NestedMeta::Meta(syn::Meta::Path(p)) => {
                    // A flag
                    if p.is_ident("regex") {
                        pattern_type = PatternType::Regex;
                    } else {
                        return Err(ParseError::new(p.span(), "Unknown flag"));
                    }
                }
                _ => return Err(ParseError::new(arg.span(), "Unexpected")),
            }
        }

        let pattern = match pattern {
            Some(s) => s,
            _ => return Err(ParseError::new(input.span(), "Expected a pattern")),
        };

        let pattern_span = pattern_span.unwrap();

        Ok(Self {
            pattern,
            pattern_type,
            pattern_span,
        })
    }
}

pub fn generate_call(re: &Regex, func: &syn::ItemFn) -> proc_macro2::TokenStream {
    let mut capture_names: HashSet<&str> = re.capture_names().flatten().collect();
    let func_name = &func.sig.ident;
    // Find the arguments
    let mut func_args = vec![];
    for arg in func.sig.inputs.iter() {
        match arg {
            syn::FnArg::Receiver(_) => (), // just let it work itself out
            syn::FnArg::Typed(ty) => {
                if let syn::Type::Reference(r) = &*ty.ty {
                    // Disallow 'static lifetimes. Normally the compiler would figure this out
                    // just fine, but in the case of a non-async implementation, we need to
                    // coerce the lifetimes to 'static. As long as the function we call doesn't
                    // treat it as such we're fine, but if they do, we can get a use after
                    // free.
                    if let Some(lifetime) = &r.lifetime {
                        if lifetime.ident == "static" {
                            return quote_spanned! {lifetime.span()=>
                                compile_error!("'static lifetime not allowed in step implementations");
                            };
                        }
                    }
                }

                match &*ty.pat {
                    syn::Pat::Ident(p) => {
                        let is_ref = matches!(&*ty.ty, syn::Type::Reference(_));
                        func_args.push((p.ident.clone(), is_ref));
                    }
                    _ => {
                        return quote_spanned! {arg.span()=>
                            compile_error!("Expected an identifier");
                        };
                    }
                }
            }
        }
    }

    // place the function call parameters
    let mut func_inputs = quote! {};
    for (ident, is_ref) in func_args {
        let name = ident.to_string();
        if capture_names.take(name.as_str()).is_some() {
            let parse = if is_ref {
                quote! {}
            } else {
                quote! { .parse()? }
            };

            func_inputs.extend(quote! { captures.name(#name).unwrap().as_str()#parse, });
        } else if name == "context" || name == "_context" {
            func_inputs.extend(quote! { &mut context, });
        } else {
            func_inputs.extend(quote_spanned! {ident.span()=>
                compile_error!("Parameter not captured by pattern"),
            });
        }
    }

    let mut func_call = quote! {
        #func_name(#func_inputs)
    };

    // Anything left is a missing parameter. Append to func call so that the user gets them all
    for name in capture_names.iter() {
        let err = format!("Function does not capture `{:?}`", name);
        func_call.extend(quote_spanned! {func_name.span()=>
            ;compile_error!(#err)
        });
    }

    make_call(func_call, func, true, true)
}

pub fn implement_step(keyword: StepKeyword, mut args: StepArgs, func: syn::ItemFn) -> TokenStream {
    // always normalized to English, capitalized
    let prefix = match keyword {
        StepKeyword::Given => "Given ",
        StepKeyword::When => "When ",
        StepKeyword::Then => "Then ",
        StepKeyword::Raw => "",
        StepKeyword::Any => "(?:Given|When|Then) ",
    };

    if let Err(e) = args.expand_pattern() {
        return e.to_compile_error().into();
    }

    let final_pattern = format!("^(?i){}{}$", prefix, args.pattern);
    let re = match Regex::new(&final_pattern) {
        Ok(r) => r,
        Err(_) => {
            return quote_spanned! {args.pattern_span=>
                compile_error!("Pattern resulted in invalid regular expression");
            }
            .into();
        }
    };

    let pattern = re.as_str();
    // Line and file name are available in nightly, so leave as an unimplemented feature for now.
    let line: i32 = -1;
    let filename = "<unavailable>";
    let run_step = generate_call(&re, &func);

    (quote! {
        #func

        const _: () = {
            use ::zuke::reexport::inventory;
            inventory::submit! {

                struct StepImpl {
                    regex: ::zuke::reexport::regex::Regex,
                    location: ::zuke::Location,
                }

                #[::async_trait::async_trait]
                impl ::zuke::StepImplementation for StepImpl {
                    fn regex(&self) -> &::zuke::reexport::regex::Regex {
                        &self.regex
                    }

                    fn location(&self) -> &::zuke::Location {
                        &self.location
                    }

                    async fn execute(
                        &self,
                        mut context: &mut ::zuke::Context,
                        captures: &::zuke::reexport::regex::Captures,
                        ) -> ::anyhow::Result<()> {
                        #run_step
                    }
                }

                let step = ::std::boxed::Box::new(StepImpl {
                    regex: ::zuke::reexport::regex::Regex::new(#pattern).unwrap(),
                    location: ::zuke::Location {
                        path: ::std::path::PathBuf::from(#filename),
                        line: #line,
                    },
                });

                ::std::boxed::Box::leak(step) as &'static dyn ::zuke::StepImplementation
            }
        };

    })
    .into()
}
