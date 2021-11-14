//! A simple text based output
use super::Reporter;
use crate::component::{Component, ComponentKind};
use crate::event::Event;
use crate::options::TestOptions;
use crate::{extra_options, reporter};
use crate::{Outcome, Verdict};
use anyhow;
use async_broadcast as broadcast;
use async_std::io::{stdout, Stdout};
use async_trait::async_trait;
use clap::{App, Arg};
use futures::io::{AllowStdIo, AsyncWrite, AsyncWriteExt};
use futures::stream::StreamExt;
use std::fs;
use std::io;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

/// Reporter that prints simple text output to a stream
pub struct PlainReporter<T: AsyncWrite> {
    out: T,
}

#[reporter("plain")]
fn make_plain(_name: &str, options: &TestOptions) -> anyhow::Result<Box<dyn Reporter>> {
    // TODO: Make sure only one reporter can use "--output" at a time.
    match options.opts.value_of_os("output") {
        Some(path) => Ok(Box::new(PlainReporter::from(fs::File::create(path)?))),
        None => Ok(Box::new(PlainReporter::default())),
    }
}

#[extra_options]
fn plain_options<'a>(app: App<'static, 'a>) -> App<'static, 'a> {
    app.arg(
        Arg::with_name("output")
            .short("o")
            .long("output")
            .value_name("FILE")
            .takes_value(true)
            .help("Output file for text output. Default is stdout."),
    )
}

impl<T: AsyncWrite + Send + Sync + 'static> From<T> for PlainReporter<T> {
    fn from(out: T) -> Self {
        Self { out }
    }
}

impl<T: Write + Send + Sync + 'static> From<T> for PlainReporter<AllowStdIo<T>> {
    fn from(out: T) -> Self {
        Self {
            out: AllowStdIo::new(out),
        }
    }
}

impl Default for PlainReporter<Stdout> {
    fn default() -> Self {
        Self::from(stdout())
    }
}

#[async_trait]
impl<T: AsyncWrite + Send + Sync + 'static> Reporter for PlainReporter<T> {
    async fn report(
        self: Box<Self>,
        _global: Arc<Component>,
        events: broadcast::Receiver<Event>,
    ) -> anyhow::Result<()> {
        self.execute(events).await
    }
}

impl<T: AsyncWrite + Send + Sync + 'static> PlainReporter<T> {
    async fn execute(self, mut events: broadcast::Receiver<Event>) -> anyhow::Result<()> {
        let mut final_result = None;

        let out = self.out;
        futures::pin_mut!(out);

        // for now just print features as they complete
        while let Some(event) = events.next().await {
            if let Event::Finished(outcome) = event {
                match outcome.kind() {
                    ComponentKind::Global => {
                        final_result = Some(outcome);
                    }
                    ComponentKind::Feature => {
                        print_feature(&mut out, outcome).await?;
                    }
                    _ => (),
                }
            }
        }

        let outcome = match final_result {
            Some(o) => o,
            None => anyhow::bail!("Did not receive final test result"),
        };

        let stats = outcome.stats();
        let rows = [
            (ComponentKind::Feature, "features"),
            (ComponentKind::Rule, "rules"),
            (ComponentKind::Scenario, "scenarios"),
            (ComponentKind::Step, "steps"),
        ];

        for (kind, noun) in rows {
            let stat = stats
                .get(&kind)
                .map(Clone::clone)
                .unwrap_or_else(Default::default);
            out.write_all(
                format!(
                    "{} {} passed, {} failed, {} skipped\n",
                    stat.passed, noun, stat.failed, stat.skipped,
                )
                .as_ref(),
            )
            .await?;
        }

        out.write_all(format!("Took {}\n\n", format_duration(&outcome)).as_ref())
            .await?;

        // overall return code
        if outcome.failed() {
            anyhow::bail!("Test run failed");
        } else {
            Ok(())
        }
    }
}

fn is_scenario(outcome: &&Arc<Outcome>) -> bool {
    outcome.kind() == ComponentKind::Scenario
}

async fn print_feature<T: AsyncWrite + std::marker::Unpin>(
    out: &mut T,
    outcome: Arc<Outcome>,
) -> io::Result<()> {
    if outcome.verdict == Verdict::Excluded {
        return Ok(());
    }

    let feature = outcome.component().feature().unwrap();
    out.write_all(
        format!(
            "{}: {}\t# {}:{}\n",
            feature.keyword,
            feature.name,
            feature
                .path
                .as_ref()
                .unwrap_or(&PathBuf::from("<???>"))
                .display(),
            feature.position.line
        )
        .as_ref(),
    )
    .await?;

    out.write_all("\n".as_ref()).await?;

    // If there is a feature-level reason, print it out.
    if let Some(err) = outcome.reason.as_ref() {
        out.write_all(textwrap::indent(&format!("{:?}", &err), "  ").as_bytes())
            .await?;
        out.write_all("\n\n".as_ref()).await?;
    }

    // Scenarios first, then rules
    for child in outcome.children.iter().filter(is_scenario) {
        print_scenario(out, child, "  ").await?;
    }

    for child in outcome
        .children
        .iter()
        .filter(|o| o.kind() == ComponentKind::Rule)
    {
        print_rule(out, child).await?;
    }

    out.write_all("\n".as_ref()).await?;
    Ok(())
}

async fn print_rule<T: AsyncWrite + std::marker::Unpin>(
    out: &mut T,
    outcome: &Arc<Outcome>,
) -> io::Result<()> {
    if outcome.verdict == Verdict::Excluded {
        return Ok(());
    }

    let feature = outcome.component().feature().unwrap();
    let rule = outcome.component().rule().unwrap();
    out.write_all(
        format!(
            "  {}: {}\t# {}:{}\n",
            rule.keyword,
            rule.name,
            feature
                .path
                .as_ref()
                .unwrap_or(&PathBuf::from("<???>"))
                .display(),
            rule.position.line
        )
        .as_ref(),
    )
    .await?;

    for child in outcome.children.iter().filter(is_scenario) {
        print_scenario(out, child, "    ").await?;
    }

    out.write_all("\n".as_ref()).await?;
    Ok(())
}

async fn print_scenario<T: AsyncWrite + std::marker::Unpin>(
    out: &mut T,
    outcome: &Arc<Outcome>,
    indent: &str,
) -> io::Result<()> {
    if outcome.verdict == Verdict::Excluded {
        return Ok(());
    }

    let feature = outcome.component().feature().unwrap();
    let scenario = outcome.component().scenario().unwrap();
    out.write_all(
        format!(
            "{}{}: {}\t# {}:{} {}\n",
            indent,
            scenario.keyword,
            scenario.name,
            feature
                .path
                .as_ref()
                .unwrap_or(&PathBuf::from("<???>"))
                .display(),
            scenario.position.line,
            format_duration(outcome),
        )
        .as_ref(),
    )
    .await?;

    // If there is a scenario-level reason, print it out.
    if let Some(err) = outcome.reason.as_ref() {
        out.write_all(textwrap::indent(&format!("{:?}", &err), "  ").as_bytes())
            .await?;
        out.write_all("\n\n".as_ref()).await?;
    }

    let indent = format!("  {}", indent);
    for child in outcome
        .children
        .iter()
        .filter(|o| o.kind() == ComponentKind::Step)
    {
        print_step(out, child, &indent).await?;
    }

    out.write_all("\n".as_ref()).await?;
    Ok(())
}

async fn print_step<T: AsyncWrite + std::marker::Unpin>(
    out: &mut T,
    outcome: &Arc<Outcome>,
    indent: &str,
) -> io::Result<()> {
    // currently we don't have info on where the steps were implemented, except in nightly
    let step = outcome.component().step().unwrap();
    let duration = format_duration(outcome);
    out.write_all(
        format!(
            "{}{} {}\t# {} {}\n",
            indent, step.keyword, step.value, outcome.verdict, duration
        )
        .as_ref(),
    )
    .await?;

    if let Some(e) = &outcome.reason {
        let indent = format!("{}  ", indent);
        let errmsg = format!("{:?}\n", e);
        let errmsg = textwrap::indent(&errmsg, &indent);
        out.write_all(errmsg.as_ref()).await?;
    }

    Ok(())
}

fn format_duration(outcome: &Arc<Outcome>) -> String {
    let duration = outcome.ended - outcome.started;
    if let Some(ns) = duration.num_nanoseconds() {
        if ns < 500_000 {
            // 0 -> 500us, display as us
            format!("{:.3} μs", (ns as f64) / 1_000.0)
        } else if ns <= 500_000_000 {
            // 500us => 500ms, display as ms
            format!("{:.3} ms", (ns as f64) / 1_000_000.0)
        } else {
            // > 500ms, display as seconds
            format!("{:.3} s", (ns as f64) / 1_000_000_000.0)
        }
    } else {
        String::from("--- s")
    }
}
