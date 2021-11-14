//! Top level test configuration
use crate::context::Context;
use crate::flag::Flag;
use crate::vocab::Vocab;
use anyhow::Context as _;
use clap::{App, Arg, ArgMatches};
use futures::future::BoxFuture;
use regex::{RegexSet, RegexSetBuilder};
use std::sync::Arc;

/// A callback that executes just prior to test execution.
pub trait HookFn:
    (for<'a> Fn(&'a mut Context) -> BoxFuture<'a, anyhow::Result<()>>) + Sync + Send + 'static
{
}
impl<F> HookFn for F where
    F: (for<'a> Fn(&'a mut Context) -> BoxFuture<'a, anyhow::Result<()>>) + Sync + Send + 'static
{
}

/// Global test information
///
/// This has also gained some global state...
pub struct TestOptions {
    /// Command line arguments passed to this test run
    pub opts: ArgMatches<'static>,
    /// Registry of step implementations
    pub vocab: Arc<Vocab>,
    /// Title of the test run. An arbitrary value that may be used by reporters.
    pub title: String,
    /// Hooks that run prior to test execution.
    pub pre_test_hooks: Arc<Vec<Box<dyn HookFn>>>,
    /// Names of components to include. Not that an empty set means include everything
    pub included: RegexSet,
    /// Names of components to exclude. Not that an empty set means exclude nothing
    pub excluded: RegexSet,
    /// Notification that the user would like to cancel the test run
    pub canceled: Flag,
}

impl TestOptions {
    /// Creats a [`TestOptionsBuilder`]
    pub fn builder() -> TestOptionsBuilder {
        TestOptionsBuilder::new()
    }

    /// Creats a default set of test options
    pub fn new() -> anyhow::Result<Self> {
        Self::builder().build()
    }

    /// Explicitly includes something by name
    pub fn includes(&self, name: &str) -> bool {
        self.included.is_empty() || self.included.is_match(name)
    }

    /// Explicitly excludes something by name
    pub fn excludes(&self, name: &str) -> bool {
        self.excluded.is_match(name)
    }
}

/// A hook that can add command line arguments. Useful for adding arguments for test fixtures.
///
/// Examples:
///
/// ```
/// use clap::{App, Arg};
/// use zuke::ExtraOptionsFunc;
///
/// fn my_hook<'a>(app: App<'static, 'a>) -> App<'static, 'a> {
///     app.arg(Arg::with_name("my_option")
///             .long("my_option")
///             .takes_value(true))
/// }
/// inventory::submit! { ExtraOptionsFunc::from(my_hook) }
/// ```
pub struct ExtraOptionsFunc {
    make_options: Box<dyn for<'a> Fn(App<'static, 'a>) -> App<'static, 'a>>,
}

impl<F> From<F> for ExtraOptionsFunc
where
    F: for<'a> Fn(App<'static, 'a>) -> App<'static, 'a> + 'static,
{
    fn from(func: F) -> Self {
        let make_options = Box::new(func);
        Self { make_options }
    }
}

inventory::collect!(ExtraOptionsFunc);

/// Builder for [`TestOptions`]
pub struct TestOptionsBuilder {
    // Can't contain clap::App, because that's not Send. Make it harder to test this struct using Zuke
    // itself
    title: String,
    pre_test_hooks: Vec<Box<dyn HookFn>>,
    canceled: Flag,
}

impl Default for TestOptionsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TestOptionsBuilder {
    /// Create a new [`TestOptionsBuilder`]
    pub fn new() -> Self {
        Self {
            title: String::from("Zuke"),
            pre_test_hooks: vec![],
            canceled: Flag::new(),
        }
    }

    /// Set the test title. This is an abitrary value used when generating output.
    pub fn title<T: Into<String>>(&mut self, title: T) -> &mut Self {
        self.title = title.into();
        self
    }

    /// Add a pre-test hook that will execute just before the test run begins.
    pub fn pre_test_hook<F: HookFn>(&mut self, hook: F) -> &mut Self {
        self.pre_test_hooks.push(Box::new(hook));
        self
    }

    /// Set the canceled flag. You probably won't need this.
    ///
    /// Used to share cancelation between multiple Zuke instances
    pub fn cancel(&mut self, flag: Flag) -> &mut Self {
        self.canceled = flag;
        self
    }

    /// Create the test options with default command line arguments
    pub fn build(self) -> anyhow::Result<TestOptions> {
        self.build_with_app(App::new("Zuke"))
    }

    /// Add the base options
    fn add_base_options<'a>(app: App<'static, 'a>) -> App<'static, 'a> {
        app.arg(
            Arg::with_name("name")
                .short("n")
                .long("name")
                .takes_value(true)
                .multiple(true)
                .max_values(1)
                .value_name("REGEX")
                .help("Only run components (features, scenarios) that match REGEX"),
        )
        .arg(
            Arg::with_name("exclude")
                .short("e")
                .long("exclude")
                .takes_value(true)
                .multiple(true)
                .max_values(1)
                .value_name("REGEX")
                .help("Don't run components (features, scenarios) that match REGEX"),
        )
    }

    /// Parse the base options
    fn parse_base_options(opts: &ArgMatches<'static>) -> anyhow::Result<(RegexSet, RegexSet)> {
        let included: Vec<_> = match opts.values_of("name") {
            None => vec![],
            Some(values) => values.collect(),
        };
        let included = RegexSetBuilder::new(included)
            .case_insensitive(true)
            .build()
            .with_context(|| "Bad --name pattern")?;

        let excluded: Vec<_> = match opts.values_of("exclude") {
            None => vec![],
            Some(values) => values.collect(),
        };
        let excluded = RegexSetBuilder::new(excluded)
            .case_insensitive(true)
            .build()
            .with_context(|| "Bad --exclude pattern")?;

        Ok((included, excluded))
    }

    /// Create the test options with custom command line arguments. Any registered
    /// [`ExtraOptionsFunc`]s will still be added to `app`.
    pub fn build_with_app(self, app: App<'static, '_>) -> anyhow::Result<TestOptions> {
        self.build_with_app_from(app, &mut std::env::args_os())
    }

    /// As `build_with_app` but allows you to specify the command line yourself.
    pub fn build_with_app_from<I, T>(
        self,
        mut app: App<'static, '_>,
        iter: I,
    ) -> anyhow::Result<TestOptions>
    where
        I: IntoIterator<Item = T>,
        T: Into<std::ffi::OsString> + Clone,
    {
        let Self {
            title,
            pre_test_hooks,
            canceled,
        } = self;

        let vocab = Arc::new(Vocab::new()?);

        app = Self::add_base_options(app);
        for extra in inventory::iter::<ExtraOptionsFunc>() {
            app = (extra.make_options)(app);
        }

        let opts = app.get_matches_from_safe(iter)?;
        let (included, excluded) = Self::parse_base_options(&opts)?;

        Ok(TestOptions {
            opts,
            vocab,
            title,
            pre_test_hooks: Arc::new(pre_test_hooks),
            included,
            excluded,
            canceled,
        })
    }
}
