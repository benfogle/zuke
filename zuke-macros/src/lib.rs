#![warn(missing_docs)]

//! Macros for step implementations. Do not import directly. Use the `zuke` crate instead.
use proc_macro::TokenStream;
mod hooks;
mod options;
mod reporter;
mod step_args;
mod utils;
use hooks::*;
use options::*;
use reporter::*;
use step_args::*;

/// Implement a "given" step
///
/// # Examples
///
/// ```ignore
/// #[given("I have a widget")]
/// fn i_have_a_widget(context: &mut Context) -> anyhow::Result<()> {
///     Ok(())
/// }
/// ```
#[proc_macro_attribute]
pub fn given(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = syn::parse_macro_input!(args as StepArgs);
    let func = syn::parse_macro_input!(input as syn::ItemFn);
    implement_step(StepKeyword::Given, args, func)
}

/// Implement a "when" step
#[proc_macro_attribute]
pub fn when(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = syn::parse_macro_input!(args as StepArgs);
    let func = syn::parse_macro_input!(input as syn::ItemFn);
    implement_step(StepKeyword::When, args, func)
}

/// Implement a "then" step
#[proc_macro_attribute]
pub fn then(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = syn::parse_macro_input!(args as StepArgs);
    let func = syn::parse_macro_input!(input as syn::ItemFn);
    implement_step(StepKeyword::Then, args, func)
}

/// Implement a generic step. This matches Given, When, and Then.
#[proc_macro_attribute]
pub fn step(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = syn::parse_macro_input!(args as StepArgs);
    let func = syn::parse_macro_input!(input as syn::ItemFn);
    implement_step(StepKeyword::Any, args, func)
}

/// Implement a raw step. Matching against Given/When/Then must be done manually.
#[proc_macro_attribute]
pub fn raw(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = syn::parse_macro_input!(args as StepArgs);
    let func = syn::parse_macro_input!(input as syn::ItemFn);
    implement_step(StepKeyword::Raw, args, func)
}

/// Register a reporter struct for command line use
#[proc_macro_attribute]
pub fn reporter(args: TokenStream, input: TokenStream) -> TokenStream {
    let name = syn::parse_macro_input!(args as syn::LitStr);
    let func = syn::parse_macro_input!(input as syn::ItemFn);
    register_reporter(&name.value(), func)
}

/// Register a function to add extra command line options
#[proc_macro_attribute]
pub fn extra_options(_args: TokenStream, input: TokenStream) -> TokenStream {
    let func = syn::parse_macro_input!(input as syn::ItemFn);
    register_options(func)
}

/// Run a hook before the entire test run
#[proc_macro_attribute]
pub fn before_all(args: TokenStream, input: TokenStream) -> TokenStream {
    register_before_after(args, input, true, Kind::Global)
}

/// Run a hook after the entire test run
#[proc_macro_attribute]
pub fn after_all(args: TokenStream, input: TokenStream) -> TokenStream {
    register_before_after(args, input, false, Kind::Global)
}

/// Run a hook before each feature
#[proc_macro_attribute]
pub fn before_feature(args: TokenStream, input: TokenStream) -> TokenStream {
    register_before_after(args, input, true, Kind::Feature)
}

/// Run a hook after each feature
#[proc_macro_attribute]
pub fn after_feature(args: TokenStream, input: TokenStream) -> TokenStream {
    register_before_after(args, input, false, Kind::Feature)
}

/// Run a hook before each rule
#[proc_macro_attribute]
pub fn before_rule(args: TokenStream, input: TokenStream) -> TokenStream {
    register_before_after(args, input, true, Kind::Rule)
}

/// Run a hook after each rule
#[proc_macro_attribute]
pub fn after_rule(args: TokenStream, input: TokenStream) -> TokenStream {
    register_before_after(args, input, false, Kind::Rule)
}

/// Run a hook before each scenario
#[proc_macro_attribute]
pub fn before_scenario(args: TokenStream, input: TokenStream) -> TokenStream {
    register_before_after(args, input, true, Kind::Scenario)
}

/// Run a hook after each scenario
#[proc_macro_attribute]
pub fn after_scenario(args: TokenStream, input: TokenStream) -> TokenStream {
    register_before_after(args, input, false, Kind::Scenario)
}

/// Run a hook before each step
#[proc_macro_attribute]
pub fn before_step(args: TokenStream, input: TokenStream) -> TokenStream {
    register_before_after(args, input, true, Kind::Step)
}

/// Run a hook after each step
#[proc_macro_attribute]
pub fn after_step(args: TokenStream, input: TokenStream) -> TokenStream {
    register_before_after(args, input, false, Kind::Step)
}

/// Run a hook before each component (except individual steps).
///
/// Note that if you want to include steps, you can add `#[before_step] to the hook as well.
#[proc_macro_attribute]
pub fn before(args: TokenStream, input: TokenStream) -> TokenStream {
    register_before_after(args, input, true, Kind::Any)
}

/// Run a hook after each component (except individual steps).
///
/// Note that if you want to include steps, you can add `#[after_step] to the hook as well.
#[proc_macro_attribute]
pub fn after(args: TokenStream, input: TokenStream) -> TokenStream {
    register_before_after(args, input, false, Kind::Any)
}
