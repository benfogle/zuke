//! Registry for step implementations

use crate::context::Context;
use crate::panic::PanicToError;
use async_trait::async_trait;
use gherkin_rust::StepType;
use inventory;
use regex::{Captures, Regex, RegexSet, RegexSetBuilder};
use std::path::PathBuf;
use thiserror::Error;

/// An error that can occur when finding a step implementation
#[derive(Error, Debug)]
pub enum Error {
    /// No implementation found for the step
    #[error("No implementation found for {what:?}")]
    NoMatch {
        /// The expanded step that failed to match
        what: String,
    },
    /// Multiple implementations found for the step
    #[error("Multiple implementations found for {what:?}")]
    MultipleMatches {
        /// The expanded step that matched
        what: String,
        /// Where it matched. (Not meaningful currently.)
        locations: Vec<Location>,
    },
    /// Something went wrong dispatching the step implementation
    #[error("Wiring error: Bad parameters")]
    BadParameters,
}

/// A location where a step was implemented. Currently unused as this information is not exposed to
/// our macros except on nightly.
#[derive(Debug, Clone)]
pub struct Location {
    /// The source file of the step implementation
    pub path: PathBuf,
    /// The line number of the step implementation
    pub line: i32,
}

/// A step implementation
///
/// Users are not expected to implement this manually. Instead, the [`crate::given`],
/// [`crate::when`], and [`crate::then`] macros will create a `StepImplementation` for a given
/// function.
#[async_trait]
pub trait StepImplementation: Send + Sync {
    /// The regular expression for this step
    fn regex(&self) -> &Regex;
    /// The location this step was defined at. Not currently meaningful.
    fn location(&self) -> &Location;
    /// Execute this step implementation.
    async fn execute(&self, context: &mut Context, args: &Captures) -> anyhow::Result<()>;
}

/// Central registry of all step implementations
///
/// User's won't interact with this directly.
pub struct Vocab {
    regexes: RegexSet,
    steps: Vec<&'static dyn StepImplementation>,
}

impl Vocab {
    /// Create a new `Vocab` objecct.
    pub fn new() -> Result<Self, regex::Error> {
        let steps: Vec<_> = inventory::iter::<&'static dyn StepImplementation>
            .into_iter()
            .copied()
            .collect();
        let regexes = RegexSetBuilder::new(steps.iter().map(|s| s.regex().as_str()))
            .case_insensitive(true)
            .build()?;

        Ok(Self { steps, regexes })
    }

    /// Execute a step
    pub async fn execute(&self, context: &mut Context) -> anyhow::Result<()> {
        let step = match context.step() {
            Some(s) => s,
            None => anyhow::bail!("Step dispatch outside of step context"),
        };

        // Normalize step to English
        let mut line = String::from(match step.ty {
            StepType::Given => "Given ",
            StepType::When => "When ",
            StepType::Then => "Then ",
        });
        line.push_str(step.value.as_str());

        let matches: Vec<_> = self.regexes.matches(&line).into_iter().collect();

        if matches.is_empty() {
            let what = format!("{} {}", &step.keyword, &step.value);
            Err(Error::NoMatch { what }.into())
        } else if matches.len() > 1 {
            let what = format!("{} {}", &step.keyword, &step.value);
            let locations = matches
                .into_iter()
                .map(|i| self.steps[i].location().clone())
                .collect();
            Err(Error::MultipleMatches { what, locations }.into())
        } else {
            let i = matches[0];
            let captures = match self.steps[i].regex().captures(&line) {
                Some(c) => c,
                None => return Err(Error::BadParameters.into()),
            };

            self.execute_step(self.steps[i], context, &captures).await
        }
    }

    fn execute_step<'a>(
        &self,
        step: &'static dyn StepImplementation,
        context: &'a mut Context,
        captures: &'a Captures<'a>,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + 'a {
        // implemented this way because we had some trouble with the exact same code below
        // producing "impl Trait captures lifetime that does not appear in bounds." Specify the
        // future lifetime manually this way.

        async move {
            // Telling users not to use assert! in test code is a no-go. As long as it's the step
            // implementation that panics, and not Zuke or longer lived fixtures, then it should be
            // unwind safe.
            PanicToError::from(step.execute(context, captures)).await
        }
    }
}

inventory::collect!(&'static dyn StepImplementation);
