//! Feature generation

use crate::component::Component;
use crate::outcome::Outcome;
use anyhow;
use async_trait::async_trait;
use futures::channel::mpsc;
use futures::stream::{FuturesUnordered, StreamExt};
use futures::{stream, SinkExt};
use gherkin_rust::{Feature, GherkinEnv, Rule, Scenario};
use lazy_static::lazy_static;
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// A `crate::parser::Parser` generates features and feeds them into a [`crate::runner::Runner`].
#[async_trait]
pub trait Parser: Send + Sync {
    /// Generate features and send them to `output`. If a feature fails to parse, this function
    /// should emit a placeholder component in a failed state.
    async fn parse(self: Box<Self>, global: Arc<Component>, output: mpsc::Sender<Outcome>);
}

enum FeatureSource {
    Dir(PathBuf),
    File(PathBuf),
    Source(String, String),
}

/// Parses features from files, directories, or source strings
pub struct StandardParser {
    sources: Vec<FeatureSource>,
    language: String,
}

impl Default for StandardParser {
    fn default() -> Self {
        Self::new()
    }
}

impl StandardParser {
    /// Create a new `StandardParser` with no inputs.
    pub fn new() -> Self {
        Self {
            sources: vec![],
            language: "en".to_string(),
        }
    }

    /// Create a new `StandardParser` with a source string as input. The `filename` parameter is
    /// arbitrary and used for displaying information to the user.
    ///
    /// See also [`Self::add_source`]
    pub fn from_source(filename: String, source: String) -> Self {
        let mut parser = Self::new();
        parser.add_source(filename, source);
        parser
    }

    /// Create a new `StandardParser` with a file or directory as input.
    ///
    /// See also [`Self::add_path`]
    pub fn from_path<P: AsRef<Path>>(path: P) -> Self {
        let mut parser = Self::new();
        parser.add_path(path);
        parser
    }

    /// Add a feature from a source string.  The `filename` parameter is arbitrary and used for
    /// displaying information to the user.
    pub fn add_source(&mut self, filename: String, source: String) -> &mut Self {
        self.sources.push(FeatureSource::Source(filename, source));
        self
    }

    /// Add a file or directory as input. If `path` is a directory, it will be searched recursively
    /// for `*.feature` files.
    pub fn add_path<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        let path = path.as_ref();

        // if it's not a dir, or if there was an error, pass it along as a file and we'll get a
        // sensible error at parse time.
        let source = match fs::metadata(path) {
            Ok(m) if m.is_dir() => FeatureSource::Dir(path.to_path_buf()),
            _ => FeatureSource::File(path.to_path_buf()),
        };

        self.sources.push(source);
        self
    }

    async fn execute(
        self,
        global: Arc<Component>,
        output: mpsc::Sender<Outcome>,
    ) -> Result<(), mpsc::SendError> {
        let StandardParser { sources, language } = self;
        let mut sources = stream::iter(sources).fuse();
        let mut pending = FuturesUnordered::new();

        loop {
            futures::select! {
                source = sources.select_next_some() => {
                    let mut out = output.clone();
                    let fut = async {
                        match source {
                            FeatureSource::File(path) => {
                                parse_feature_file(path, &language, &global, &mut out).await
                            },
                            FeatureSource::Dir(path) => {
                                parse_feature_dir(path, &language, &global, out).await
                            },
                            FeatureSource::Source(filename, source) => {
                                parse_feature_source(filename, source, &language, &global, out).await
                            },
                        }
                    };
                    pending.push(fut);
                },
                result = pending.select_next_some() => {
                    if let Err(e) = result {
                        return Err(e);
                    }
                },
                complete => break,
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Parser for StandardParser {
    async fn parse(self: Box<Self>, global: Arc<Component>, output: mpsc::Sender<Outcome>) {
        let _ = self.execute(global, output).await;
    }
}

// this one is written to be either top level or called from parse_feature_dir
async fn parse_feature_file(
    path: PathBuf,
    lang: &str,
    global: &Arc<Component>,
    output: &mut mpsc::Sender<Outcome>,
) -> Result<(), mpsc::SendError> {
    let outcome = match do_parse_feature_file(&path, lang) {
        Ok(mut feature) => {
            let result = cook_feature(&mut feature);
            let mut outcome = Outcome::undecided(global.with_feature(feature));
            if let Err(e) = result {
                outcome.set_err(e);
            }
            outcome
        }
        Err(e) => {
            let feature = Feature::builder()
                .keyword("Feature".into())
                .name(path.display().to_string())
                .path(Some(path))
                .build();
            let mut outcome = Outcome::undecided(global.with_feature(feature));
            outcome.set_err(e);
            outcome
        }
    };

    output.send(outcome).await
}

/// maybe should go on a blocking task, but it's probably not the bottleneck.
fn do_parse_feature_file(path: &Path, lang: &str) -> anyhow::Result<Feature> {
    let env = GherkinEnv::new(lang)?;
    let feature = Feature::parse_path(&path, env)?;
    Ok(feature)
}

/// maybe should go on a blocking task, but it's probably not the bottleneck.
async fn parse_feature_dir(
    path: PathBuf,
    lang: &str,
    global: &Arc<Component>,
    mut output: mpsc::Sender<Outcome>,
) -> Result<(), mpsc::SendError> {
    // skip errors. If the top level doesn't exist, we've already handled that when checking the
    // source type. Otherwise we don't want to crash because we recursed farther than the user
    // intended.
    let mut dirs = vec![path];

    let is_dir = |e: &fs::DirEntry| match e.file_type() {
        Ok(t) => t.is_dir(),
        Err(_) => false,
    };

    let is_feature = |p: &Path| match p.extension() {
        Some(s) => s == "feature",
        None => false,
    };

    while let Some(path) = dirs.pop() {
        if let Ok(items) = fs::read_dir(path) {
            for entry in items.flatten() {
                let path = entry.path();

                if is_dir(&entry) {
                    dirs.push(path);
                } else if is_feature(&path) {
                    parse_feature_file(path, lang, global, &mut output).await?;
                }
            }
        }
    }

    Ok(())
}

async fn parse_feature_source(
    filename: String,
    source: String,
    lang: &str,
    global: &Arc<Component>,
    mut output: mpsc::Sender<Outcome>,
) -> Result<(), mpsc::SendError> {
    let outcome = match do_parse_feature_source(&filename, &source, lang) {
        Ok(feature) => Outcome::undecided(global.with_feature(feature)),
        Err(e) => {
            let feature = Feature::builder()
                .keyword("Feature".into())
                .name(filename.clone())
                .path(Some(filename.into()))
                .build();
            let mut outcome = Outcome::undecided(global.with_feature(feature));
            outcome.set_err(e);
            outcome
        }
    };

    output.send(outcome).await
}

fn do_parse_feature_source(filename: &str, source: &str, lang: &str) -> anyhow::Result<Feature> {
    let env = GherkinEnv::new(lang)?;
    let mut feature = Feature::parse(source, env)?;
    feature.path = Some(PathBuf::from(filename));
    Ok(feature)
}

/// Function to expand scenario outlines into individual scenarios, etc.
fn cook_feature(feature: &mut Feature) -> anyhow::Result<()> {
    for rule in feature.rules.iter_mut() {
        cook_rule(rule)?;
    }

    cook_scenarios(&mut feature.scenarios)
}

fn cook_rule(rule: &mut Rule) -> anyhow::Result<()> {
    cook_scenarios(&mut rule.scenarios)
}

fn cook_scenarios(scenarios: &mut Vec<Scenario>) -> anyhow::Result<()> {
    // we will continue past errors in order to make the cooked scenarios as complete as possible.
    // This might be helpful to the user. Only return the first error.
    let mut i = 0;
    let mut result = Ok(());

    while i < scenarios.len() {
        if scenarios[i].examples.is_some() {
            match expand_scenario(&scenarios[i]) {
                Ok(expanded) => {
                    let n = expanded.len();
                    scenarios.splice(i..i + 1, expanded);
                    i += n;
                }
                Err(e) => {
                    result = result.and(Err(e));
                }
            }
        } else {
            i += 1;
        }
    }

    result
}

fn expand_scenario(scenario: &Scenario) -> anyhow::Result<Vec<Scenario>> {
    lazy_static! {
        static ref BRACKET: Regex = Regex::new("<[^>]+>").unwrap();
    }

    let examples = scenario.examples.as_ref().unwrap();
    if examples.table.rows.len() < 2 {
        return Ok(vec![]);
    }

    let key_row = &examples.table.rows[0];
    let data_rows = &examples.table.rows[1..];

    // figure out where we need to do the substitutions
    let mut params = vec![];
    for step in scenario.steps.iter() {
        params.push(
            BRACKET
                .find_iter(&step.value)
                .filter_map(|m| {
                    let subst = &m.as_str()[1..m.as_str().len() - 1];
                    let idx = key_row.iter().position(|k| k == subst)?;
                    Some((m.range(), idx))
                })
                .collect::<Vec<_>>(),
        );
    }

    let mut expanded = Vec::with_capacity(data_rows.len());
    for row in data_rows {
        let mut example = Scenario {
            keyword: scenario.keyword.clone(),
            name: scenario.name.clone(),
            steps: Vec::with_capacity(scenario.steps.len()),
            examples: None,
            tags: scenario.tags.clone(),
            span: scenario.span,
            position: scenario.position,
        };

        for (step, param_row) in scenario.steps.iter().zip(params.iter()) {
            let mut pos = 0;
            let mut expanded_step = step.clone();
            expanded_step.value.clear();
            for (range, index) in param_row.iter() {
                expanded_step.value.push_str(&step.value[pos..range.start]);
                expanded_step.value.push_str(&row[*index]);
                pos = range.end;
            }
            expanded_step.value.push_str(&step.value[pos..]);
            example.steps.push(expanded_step);
        }

        expanded.push(example);
    }

    Ok(expanded)
}
