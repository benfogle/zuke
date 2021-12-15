# Zuke: A pragmatic, Cucumber-like BDD framework for Rust

Zuke is a [Cucumber][1] inspired test framework for Rust. It's primary goals are:

1. **Concurrency**: Zuke uses Rust's [async programming features][2] to run all features and
   scenarios in parallel.
2. **Fixtures**: Zuke includes first-class support for [test fixtures][3]. Test fixtures allow
   you to manage common or expensive setup and teardown between tests.
3. **Reusablility**: Steps and fixtures can be shared between code bases by importing a crate.
   Zuke comes "batteries included" with a set of default steps and fixtures.

(The name "Zuke" is short for "zucchini", because it looks a lot like a cucumber.)

[1]: https://cucumber.io
[2]: https://rust-lang.github.io/async-book/
[3]: https://en.wikipedia.org/wiki/Test_fixture

