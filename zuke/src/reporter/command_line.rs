//! A reporter that creates other reporters based on the command line. Reporters that wish to
//! participate need to register via `inventory::submit!`

use super::{DefaultReporter, Reporter};
use crate::component::Component;
use crate::event::Event;
use crate::extra_options;
use crate::options::TestOptions;
use async_broadcast as broadcast;
use async_trait::async_trait;
use clap::{App, Arg};
use futures::future::join_all;
use std::sync::Arc;

/// A reporter that creates other reporters based on the command line. Reporters that wish to
/// participate need to register via `#[reporter("name")]`
#[derive(Default)]
pub struct CommandLineReporter;

#[extra_options]
fn choose_reporter<'a>(app: App<'static, 'a>) -> App<'static, 'a> {
    app.arg(
        Arg::with_name("reporters")
            .multiple(true)
            .short("r")
            .long("reporter")
            .takes_value(true)
            .max_values(1)
            .value_name("NAME")
            .help("Add a reporter. If no reporter is given, a default reporter will be used"),
    )
}

#[doc(hidden)]
/// A reporter entry. You may prefer using the `#[reporter]` macro.
pub struct ReporterEntry {
    pub name: String,
    pub func: fn(name: &str, options: &TestOptions) -> anyhow::Result<Box<dyn Reporter>>,
}

#[async_trait]
impl Reporter for CommandLineReporter {
    async fn report(
        self: Box<Self>,
        global: Arc<Component>,
        events: broadcast::Receiver<Event>,
    ) -> anyhow::Result<()> {
        // launch all sub-reporters
        let futs: Vec<_> = make_reporters(&global)?
            .into_iter()
            .map(|r| {
                let e = events.clone();
                let g = global.clone();
                async move { r.report(g, e).await }
            })
            .collect();
        drop(events);
        drop(global);

        // await and return the first error
        let results = join_all(futs).await;
        for r in results {
            if r.is_err() {
                return r;
            }
        }

        return Ok(());
    }
}

fn make_reporters(global: &Component) -> anyhow::Result<Vec<Box<dyn Reporter>>> {
    let requested = match global.options().opts.values_of("reporters") {
        Some(r) => r,
        None => return Ok(vec![Box::new(DefaultReporter::default())]),
    };

    let entries: Vec<_> = inventory::iter::<ReporterEntry>().collect();
    let mut reporters = vec![];
    for req in requested {
        let reporter = match entries.iter().find(|e| e.name == req) {
            Some(e) => (e.func)(req, global.options())?,
            None => anyhow::bail!("No such reporter {}", req),
        };
        reporters.push(reporter);
    }

    Ok(reporters)
}

inventory::collect!(ReporterEntry);
