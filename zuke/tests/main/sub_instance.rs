use async_std::task;
use async_trait::async_trait;
use std::sync::Arc;
use zuke::flag::Flag;
use zuke::reporter::Collect;
use zuke::*;

enum State {
    Building,
    Pending(task::JoinHandle<Arc<Outcome>>),
    Done(Arc<Outcome>),
    Failed, // failed to run. we don't have an outcome
}

pub struct SubInstance {
    builder: Option<ZukeBuilder>,
    pub args: Vec<String>,
    result: State,
    cancel: Flag,
}

#[async_trait]
impl Fixture for SubInstance {
    const SCOPE: Scope = Scope::Scenario;

    async fn setup(_context: &mut Context) -> anyhow::Result<Self> {
        let cancel = Flag::new();
        let mut builder = ZukeBuilder::new();
        builder.cancel_method(CancelMethod::Shared(cancel.clone()));

        Ok(Self {
            builder: Some(builder),
            args: vec!["arg0".into()],
            result: State::Building,
            cancel,
        })
    }

    async fn teardown(&mut self, _context: &mut Context) -> anyhow::Result<()> {
        self.cancel.set();
        Ok(())
    }
}

impl SubInstance {
    pub fn builder(&mut self) -> &mut ZukeBuilder {
        self.builder.as_mut().expect("Tests already built")
    }

    /// Run the tests in the background
    pub fn run(&mut self) -> anyhow::Result<()> {
        if !matches!(self.result, State::Building) {
            anyhow::bail!("Tests already run");
        }

        let (collect, out) = Collect::new();
        self.builder.as_mut().unwrap().reporter(collect);
        let zuke = self.do_build()?;

        let handle = task::spawn(async move {
            let _ = zuke.run().await;
            out.await.unwrap()
        });

        self.result = State::Pending(handle);
        Ok(())
    }

    fn do_build(&mut self) -> anyhow::Result<Zuke> {
        let mut builder = self.builder.take().expect("Tests already run");
        let app = clap::App::new("zuke-sub-instance");
        builder.build_with_app_from(app, self.args.clone())
    }

    pub async fn outcome(&mut self) -> Arc<Outcome> {
        let outcome: Arc<Outcome>;

        let mut result = State::Failed;
        std::mem::swap(&mut result, &mut self.result);

        self.result = match result {
            State::Pending(handle) => {
                outcome = handle.await;
                State::Done(outcome.clone())
            }
            State::Done(o) => {
                outcome = o.clone();
                State::Done(o)
            }
            _ => panic!("Tests have not run yet"),
        };

        outcome
    }

    pub fn cancel(&self) {
        self.cancel.set();
    }
}

#[given("a zuke sub-instance")]
async fn given_a_zuke_subinstance(context: &mut Context) -> anyhow::Result<()> {
    context.use_fixture::<SubInstance>().await?;
    Ok(())
}

#[when(regex, r#"I add the path "(?P<path>.*)""#)]
async fn when_i_add_the_path(context: &mut Context, path: String) -> anyhow::Result<()> {
    let sub_instance = context.fixture_mut::<SubInstance>().await;
    sub_instance.builder().feature_path(path);
    Ok(())
}

#[when("I add the feature source")]
async fn when_i_add_feature_source(context: &mut Context) -> anyhow::Result<()> {
    let source = match &context.step().unwrap().docstring {
        Some(s) => s.clone(),
        None => anyhow::bail!("Expected a docstring"),
    };
    let sub_instance = context.fixture_mut::<SubInstance>().await;

    sub_instance.builder().feature_source("<source>", source);
    Ok(())
}

#[when(r#"I add "{args}" to the command line"#)]
async fn when_i_add_args(context: &mut Context, args: String) -> anyhow::Result<()> {
    let sub_instance = context.fixture_mut::<SubInstance>().await;
    sub_instance.args.extend(shell_words::split(&args)?);
    Ok(())
}

#[when("I run the tests")]
async fn when_i_run_the_tests(context: &mut Context) -> anyhow::Result<()> {
    let sub_instance = context.fixture_mut::<SubInstance>().await;
    sub_instance.run()
}

#[then("the tests complete successfully")]
async fn the_tests_complete_successfully(context: &mut Context) -> anyhow::Result<()> {
    let sub_instance = context.fixture_mut::<SubInstance>().await;
    let outcome = sub_instance.outcome().await;
    assert!(outcome.passed(), "Outcome failed:\n{:#?}", outcome);
    Ok(())
}

#[then(regex, r#"there are (?P<num>\d+)/(?P<total>\d+) (?P<stat>passing|failed|skipped) (?P<what>features|rules|scenarios|steps)"#)]
async fn check_stats(
    context: &mut Context,
    num: usize,
    total: usize,
    stat: String,
    what: String,
) -> anyhow::Result<()> {
    let what = what.to_lowercase();
    let stat = stat.to_lowercase();

    let sub_instance = context.fixture_mut::<SubInstance>().await;
    let outcome = sub_instance.outcome().await;
    let stats = outcome.stats();

    let kind = match what.as_str() {
        "features" => ComponentKind::Feature,
        "rules" => ComponentKind::Rule,
        "scenarios" => ComponentKind::Scenario,
        "steps" => ComponentKind::Step,
        _ => panic!("Unexpected kind"),
    };

    let stat_row = match stats.get(&kind) {
        Some(s) => s.clone(),
        None => Default::default(),
    };

    let actual_num = match stat.as_str() {
        "passing" => stat_row.passed,
        "failed" => stat_row.failed,
        "skipped" => stat_row.skipped,
        _ => panic!("Unexpected stat"),
    };

    assert_eq!(num, actual_num, "Wrong number of {} {}", stat, what);
    assert_eq!(total, stat_row.total, "Wrong number of total {}", what);
    Ok(())
}

#[when("I cancel the tests")]
async fn when_i_cancel_the_tests(context: &mut Context) -> anyhow::Result<()> {
    let sub_instance = context.fixture_mut::<SubInstance>().await;
    sub_instance.cancel();
    Ok(())
}

#[then("the tests were canceled")]
async fn the_tests_were_canceled(context: &mut Context) -> anyhow::Result<()> {
    let sub_instance = context.fixture_mut::<SubInstance>().await;
    let outcome = sub_instance.outcome().await;
    assert_eq!(outcome.verdict, Verdict::Canceled);
    Ok(())
}
