#![warn(missing_docs)]

//! Zuke: A pragmatic BDD framework for Rust
//! ========================================
//!
//! Zuke is a [Cucumber][1] inspired test framework for Rust. It's primary goals are:
//!
//! 1. **Concurrency**: Zuke uses Rust's [async programming features][2] to run all features and
//!    scenarios in parallel.
//! 2. **Fixtures**: Zuke includes first-class support for [test fixtures][3]. Test fixtures allow
//!    you to manage common or expensive setup and teardown between tests.
//! 3. **Reusablility**: Steps and fixtures can be shared between code bases by importing a crate.
//!    Zuke comes "batteries included" with a set of default steps and fixtures.
//!
//! (The name "Zuke" is short for "zucchini", because it looks a lot like a cucumber.)
//!
//! [1]: https://cucumber.io
//! [2]: https://rust-lang.github.io/async-book/
//! [3]: https://en.wikipedia.org/wiki/Test_fixture

extern crate self as zuke;
pub mod component;
pub mod context;
pub mod event;
pub mod fixture;
pub mod flag;
pub mod hooks;
pub mod options;
pub mod outcome;
#[doc(hidden)]
pub mod panic;
pub mod parser;
#[doc(hidden)]
pub mod reexport;
pub mod reporter;
pub mod runner;
pub mod step;
pub mod top;
pub mod vocab;

#[cfg(feature = "tags")]
pub mod tags;

pub use component::*;
pub use context::*;
pub use event::*;
pub use fixture::*;
pub use options::*;
pub use outcome::*;
pub use panic::*;
pub use parser::*;
pub use reporter::*;
pub use runner::*;
pub use step::*;
pub use top::*;
pub use vocab::*;
pub use zuke_macros::*;
