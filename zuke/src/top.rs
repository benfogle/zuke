#![warn(missing_docs)]

//! Zuke: A pragmatic BDD framework for Rust

pub use super::*;

use crate::flag::Flag;
use crate::hooks::HookRunner;
use async_broadcast as broadcast;
use clap::App;
use futures::channel::mpsc;
use futures::future::{join_all, BoxFuture, FutureExt};
use futures::join;
use std::path::Path;
use std::sync::Arc;

/// TODO: Put this somewhere sensible
struct PanicSilencer {
    hook: Option<Box<dyn Fn(&std::panic::PanicInfo<'_>) + Sync + Send + 'static>>,
}

impl Drop for PanicSilencer {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            if let Some(hook) = self.hook.take() {
                std::panic::set_hook(hook);
            }
        }
    }
}

impl PanicSilencer {
    pub fn new() -> Self {
        let hook = Some(std::panic::take_hook());
        std::panic::set_hook(Box::new(|_| {}));
        Self { hook }
    }
}

/// Top level tester
pub struct Zuke {
    silence_panics: bool,
    parsers: Vec<Box<dyn Parser>>,
    runner: Box<dyn Runner>,
    reporters: Vec<Box<dyn Reporter>>,
    options: Arc<TestOptions>,
}

impl Zuke {
    /// Create a [`ZukeBuilder`] to customize this instance.
    ///
    /// At a miniumum, you will need to call [`ZukeBuilder::feature_path`] or
    /// [`ZukeBuilder::feature_source`].
    pub fn builder() -> ZukeBuilder {
        ZukeBuilder::new()
    }

    /// Run the test suite. Returns the final outcome, regardless of success or failure. Its return
    /// value is based on the reporters, if any.
    pub async fn run(mut self) -> anyhow::Result<()> {
        // disable "thread ... panicked" message at every assertion failure
        let _silence = if self.silence_panics {
            Some(PanicSilencer::new())
        } else {
            None
        };

        let global = Component::global(self.options.clone());
        let (features_tx, features_rx) = mpsc::channel(256);
        let (events_tx, events_rx) = broadcast::broadcast(256);

        // launch parsers and runners
        let mut runners = vec![self.runner.run(global.clone(), features_rx, events_tx)];
        runners.extend(
            self.parsers
                .drain(..)
                .map(|p| p.parse(global.clone(), features_tx.clone())),
        );
        let runners = join_all(runners);

        // launch reporters
        let reporters: Vec<_> = self
            .reporters
            .drain(..)
            .map(|r| r.report(global.clone(), events_rx.clone()))
            .collect::<Vec<_>>();
        let reporters = join_all(reporters);

        // Let them all run to completion
        drop(features_tx);
        drop(events_rx);
        let (_, results) = join!(runners, reporters);

        // Return the result, from reporters
        results.into_iter().find(Result::is_err).unwrap_or(Ok(()))
    }
}

/// How to cancel a test run
pub enum CancelMethod {
    /// Installs a Ctrl+C handler. May also be canceled manually.
    CtrlC,
    /// Share a cancellation flag with something else
    Shared(Flag),
    /// Manually cancel via `TestOptions::canceled.set()`
    Manual,
}

/// A builder for [`Zuke`]
pub struct ZukeBuilder {
    silence_panics: bool,
    cancel_method: CancelMethod,
    options_builder: TestOptionsBuilder,
    default_parser: Option<StandardParser>,
    parsers: Vec<Box<dyn Parser>>,
    runner: Box<dyn Runner>,
    reporters: Vec<Box<dyn Reporter>>,
}

impl Default for ZukeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ZukeBuilder {
    /// Create a new [`ZukeBuilder`]
    ///
    /// At a miniumum, you will need to call [`ZukeBuilder::feature_path`] or
    /// [`ZukeBuilder::feature_source`].
    pub fn new() -> Self {
        let mut zuke = Self {
            silence_panics: true,
            cancel_method: CancelMethod::CtrlC,
            options_builder: TestOptionsBuilder::new(),
            parsers: vec![],
            reporters: vec![],
            runner: Box::new(StandardRunner::new()),
            default_parser: None,
        };

        zuke.use_fixture::<HookRunner>();
        zuke
    }

    /// Create a [`Zuke`] test runner using a default set of command line arguments.  This will
    /// reset the builder to its default state.
    pub fn build(&mut self) -> anyhow::Result<Zuke> {
        self.build_with_app(App::new("Zuke"))
    }

    /// Create a [`Zuke`] test runner using a specified set of command line arguments. Extra
    /// command line arguments via [`TestOptions`] will be added to the set of command line
    /// arguments. This resets the builder to its default state.
    pub fn build_with_app(&mut self, app: App<'static, '_>) -> anyhow::Result<Zuke> {
        self.build_with_app_from(app, &mut std::env::args_os())
    }

    /// As `build_with_app`, but allows you to specify your own command line arguments.
    pub fn build_with_app_from<I, T>(
        &mut self,
        app: App<'static, '_>,
        iter: I,
    ) -> anyhow::Result<Zuke>
    where
        I: IntoIterator<Item = T>,
        T: Into<std::ffi::OsString> + Clone,
    {
        if self.reporters.is_empty() {
            self.command_line_reporter();
        }

        if self.parsers.is_empty() {
            self.default_parser();
        }

        if let Some(p) = self.default_parser.take() {
            self.parsers.push(Box::new(p));
        }

        let mut obj = Self::new();
        std::mem::swap(&mut obj, self);
        let ZukeBuilder {
            silence_panics,
            cancel_method,
            parsers,
            runner,
            reporters,
            mut options_builder,
            ..
        } = obj;

        let mut handler = false;
        match cancel_method {
            CancelMethod::CtrlC => {
                handler = true;
            }
            CancelMethod::Shared(flag) => {
                options_builder.cancel(flag);
            }
            CancelMethod::Manual => (),
        };

        let options = Arc::new(options_builder.build_with_app_from(app, iter)?);
        if handler {
            let canceled = options.canceled.clone();
            ctrlc::set_handler(move || canceled.set()).expect("Could not set up Ctrl+C handling");
        }

        Ok(Zuke {
            silence_panics,
            parsers,
            runner,
            reporters,
            options,
        })
    }

    /// How to cancel a test run. Default is Ctrl+C.
    pub fn cancel_method(&mut self, method: CancelMethod) -> &mut Self {
        self.cancel_method = method;
        self
    }

    #[doc(hidden)]
    /// Leave the default hook that prints information about panics. Generally this isn't what you
    /// want, because it will spam the output every time an assert! fails. Used for debugging Zuke
    /// itself. Note that setting RUST_BACKTRACE=1 will show panic backtraces in the output (though
    /// they can be hard to follow).
    pub fn silence_panics(&mut self, silence: bool) -> &mut Self {
        self.silence_panics = silence;
        self
    }

    /// Set the overall title of the test. Used to customize reporter output.
    pub fn title<T: Into<String>>(&mut self, title: T) -> &mut Self {
        self.options_builder.title(title);
        self
    }

    /// Cause a function to execute at global scope, just before the first feature runs.
    pub fn pre_test_hook<F: HookFn>(&mut self, hook: F) -> &mut Self {
        self.options_builder.pre_test_hook(hook);
        self
    }

    /// Use a fixture at global scope. The fixture will be in place before the first feature runs.
    /// Only globally-scoped features may be activated in this manner.
    pub fn use_fixture<F: Fixture>(&mut self) -> &mut Self {
        fn hook<F: Fixture>(context: &mut Context) -> BoxFuture<'_, anyhow::Result<()>> {
            context.use_fixture::<F>().boxed()
        }
        self.pre_test_hook(hook::<F>)
    }

    /// Add a custom parser. Multiple parsers may be added. If no parser is added, a default parser
    /// will be used based on [`ZukeBuilder::feature_path`] and [`ZukeBuilder::feature_source`].
    pub fn parser<T: Parser + 'static>(&mut self, parser: T) -> &mut Self {
        self.parsers.push(Box::new(parser));
        self
    }

    /// Add a custom runner. If no custom runner is added, the default runner will be used.
    pub fn runner<T: Runner + 'static>(&mut self, runner: T) -> &mut Self {
        self.runner = Box::new(runner);
        self
    }

    /// Add a custom reporter. Multiple reporters may be added. If no reporters are added, the
    /// command line will be examined to find a reporter (choosing a default if needed).
    pub fn reporter<T: Reporter + 'static>(&mut self, reporter: T) -> &mut Self {
        self.reporters.push(Box::new(reporter));
        self
    }

    /// Explicitly add reporters from the command line. Additional reporters may still be added in
    /// addition to the default.
    pub fn command_line_reporter(&mut self) -> &mut Self {
        self.reporter(CommandLineReporter::default())
    }

    /// Explicitly add the default parser. Additional reporters may still be added in addition to
    /// the default. The default parser will be used to handle calls to
    /// [`ZukeBuilder::feature_path`] and [`ZukeBuilder::feature_source`].
    pub fn default_parser(&mut self) -> &mut Self {
        let p = self.default_parser.take();
        self.default_parser = p.or_else(|| Some(StandardParser::default()));
        self
    }

    /// Add a feature file or directory of features to the test run
    pub fn feature_path<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        self.default_parser();
        self.default_parser.as_mut().unwrap().add_path(path);
        self
    }

    /// Add a feature as a source string. The `filename` parameter is an arbitrary value used for
    /// output.
    pub fn feature_source<N: Into<String>, S: Into<String>>(
        &mut self,
        filename: N,
        source: S,
    ) -> &mut Self {
        self.default_parser();
        self.default_parser
            .as_mut()
            .unwrap()
            .add_source(filename.into(), source.into());
        self
    }
}
