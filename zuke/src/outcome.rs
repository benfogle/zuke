//! Test outcomes

use crate::component::{Component, ComponentKind};
use crate::step::StepError;
use anyhow;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

/// A test result, but holds much more information about what happened
#[derive(Debug)]
pub struct Outcome {
    /// The component (i.e., scenario, feature, etc.) this outcome is for
    component: Arc<Component>,
    /// The final verdict (pass, fail, etc.)
    pub verdict: Verdict,
    /// Additional information about why the test failed, was skipped, etc. This is used to
    /// describe why this component decided it needed to fail. It is generally left empty if the
    /// reason for failure was "one of my sub-components" failed.
    pub reason: Option<anyhow::Error>,
    /// When the component was started
    pub started: DateTime<Utc>,
    /// When the component finished
    pub ended: DateTime<Utc>,
    /// Child outcomes. For example, a feature's outcome will use this field to point to outcomes
    /// for scenarios and rules. The top-level outcome can be traversed to get hierarchical
    /// information about the entire test run.
    pub children: Vec<Arc<Outcome>>,
}

/// A summary of how many things passed/failed/skipped.
#[derive(Debug, Clone)]
pub struct Stat {
    /// number of passing components
    pub passed: usize,
    /// number of failed components
    pub failed: usize,
    /// number of skipped components
    pub skipped: usize,
    /// total number of components
    pub total: usize,
}

impl Default for Stat {
    fn default() -> Self {
        Stat {
            passed: 0,
            failed: 0,
            skipped: 0,
            total: 0,
        }
    }
}

/// The ultimate verdict for a test component. These are ordered from lowest priority (Skipped) to
/// highest priority (Canceled).
#[derive(Copy, Clone, Debug, PartialOrd, PartialEq, Ord, Eq)]
pub enum Verdict {
    /// The outcome is still running. This should never be seen after the test is done.
    Undecided,
    /// The component was excluded from the test run (a type of "skipped")
    Excluded,
    /// The component was skipped
    Skipped,
    /// The component passed
    Passed,
    /// Something went wrong, but the component is still considered passing
    PassedWithWarnings,
    /// The component failed, but it was supposed to fail
    ExpectedFailure,
    /// The component was supposed to fail, but it passed
    UnexpectedPass,
    /// The component failed
    Failed,
    /// The component was canceled before it could complete
    Canceled,
}

impl Default for Verdict {
    fn default() -> Self {
        Self::Undecided
    }
}

impl Verdict {
    /// The verdict passed
    pub fn passed(&self) -> bool {
        matches!(
            self,
            Verdict::Passed | Verdict::PassedWithWarnings | Verdict::ExpectedFailure
        )
    }

    /// The verdict is skipped
    pub fn skipped(&self) -> bool {
        matches!(self, Verdict::Excluded | Verdict::Skipped)
    }

    /// The verdict is undecided
    pub fn is_undecided(&self) -> bool {
        *self == Self::Undecided
    }

    /// The verdict is failed
    pub fn failed(&self) -> bool {
        matches!(
            self,
            Verdict::UnexpectedPass | Verdict::Failed | Verdict::Canceled
        )
    }
}

impl fmt::Display for Verdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            Verdict::Undecided => "undecided",
            Verdict::Excluded => "excluded",
            Verdict::Skipped => "skipped",
            Verdict::Passed => "passed",
            Verdict::PassedWithWarnings => "passed (with warnings)",
            Verdict::ExpectedFailure => "passed (expected failure)",
            Verdict::Failed => "failed",
            Verdict::UnexpectedPass => "failed (unexpected success)",
            Verdict::Canceled => "failed (canceled)",
        };

        f.write_str(msg)
    }
}

/*
impl<C: Into<Arc<Component>>> From<C> for Outcome {
    fn from(component: C) -> Self {
        Self::undecided(component.into())
    }
}
*/

impl Outcome {
    /// Create a new outcome for the given component, with verdict specified
    pub fn new(component: Arc<Component>, verdict: Verdict) -> Self {
        Outcome {
            component,
            verdict,
            reason: None,
            started: Utc::now(),
            ended: Utc::now(), // will be updated
            children: vec![],
        }
    }

    /// Create a new undecided outcome for the given component
    pub fn undecided(component: Arc<Component>) -> Self {
        Self::new(component, Verdict::Undecided)
    }

    /// Create an outcome, taking the parent into account when determining the initial verdict
    pub fn with_parent(component: Arc<Component>, parent: &Self) -> Self {
        // early eval of excluded status
        let verdict = if component.is_excluded() {
            Verdict::Excluded
        } else {
            match parent.verdict {
                Verdict::Undecided => Verdict::Undecided,
                Verdict::Excluded => Verdict::Excluded,
                _ => Verdict::Skipped,
            }
        };

        Self::new(component, verdict)
    }

    /// Set the component as [`Verdict::Skipped`], with no error message.
    pub fn set_skip(&mut self) -> &mut Self {
        self.verdict = Verdict::Skipped;
        self
    }

    /// Set the component as [`Verdict::Skipped`], with an error message
    pub fn set_skip_with_reason(&mut self, e: anyhow::Error) -> &mut Self {
        self.verdict = Verdict::Skipped;
        self.reason = Some(e);
        self
    }

    /// Set the component to passed
    pub fn set_passed(&mut self) -> &mut Self {
        self.verdict = Verdict::Passed;
        self
    }

    /// Set the component to excluded (skipped)
    pub fn set_excluded(&mut self) -> &mut Self {
        self.verdict = Verdict::Excluded;
        self
    }

    /// Set the component's verdict from a `Result`. If the result is `Err(StepError)`, the verdict
    /// will honor [`StepError::Skip`], [`StepError::Warning`], etc. Otherwise, an `Err` result
    /// will set the verdict to [`Verdict::Failed`].
    pub fn set_result<T>(&mut self, result: anyhow::Result<T>) -> &mut Self {
        match result {
            Ok(_) => self.verdict = Verdict::Passed,
            Err(e) => {
                self.set_err(e);
            }
        }

        self.ended = Utc::now();
        self
    }

    /// Set the component's verdict from an error. If the error is a [`StepError`], the verdict
    /// will honor [`StepError::Skip`], [`StepError::Warning`], etc. Otherwise this function will
    /// set the verdict to [`Verdict::Failed`].
    pub fn set_err(&mut self, err: anyhow::Error) -> &mut Self {
        match err.downcast::<StepError>() {
            Ok(e) => {
                self.verdict = e.verdict;
                self.reason = e.reason;
            }
            Err(e) => {
                self.verdict = Verdict::Failed;
                self.reason = Some(e);
            }
        };

        self.ended = Utc::now();
        self
    }

    /// Add a child to the outcome. This does not set the reason, which generally isn't for
    /// describing sub-components.
    pub fn add_child(&mut self, child: Arc<Outcome>) -> &mut Self {
        if child.verdict > self.verdict {
            self.verdict = child.verdict;
        }
        self.children.push(child);
        self.ended = Utc::now();
        self
    }

    /// Return true if the component is still undecided
    pub fn is_undecided(&self) -> bool {
        self.verdict == Verdict::Undecided
    }

    /// Return true if the component passed
    pub fn passed(&self) -> bool {
        self.verdict.passed()
    }

    /// Return true if the component passed
    pub fn passed_or_undecided(&self) -> bool {
        self.verdict.passed() || self.verdict == Verdict::Undecided
    }

    /// Return true if the component was skipped
    pub fn skipped(&self) -> bool {
        self.verdict.skipped()
    }

    /// Return true if the component failed (or has not been decided)
    pub fn failed(&self) -> bool {
        self.verdict.failed()
    }

    /// Return basic stats about this outcome and all child outcomes.
    pub fn stats(&self) -> HashMap<ComponentKind, Stat> {
        let mut stats = HashMap::new();
        let mut outcomes = vec![self];

        while let Some(outcome) = outcomes.pop() {
            let entry = stats
                .entry(outcome.component.kind())
                .or_insert_with(Stat::default);
            entry.total += 1;
            if outcome.passed() {
                entry.passed += 1;
            } else if outcome.skipped() {
                entry.skipped += 1;
            } else {
                entry.failed += 1;
            }

            outcomes.extend(outcome.children.iter().map(Arc::as_ref));
        }

        stats
    }

    /// Return the component associated with this outcome
    pub fn component(&self) -> &Arc<Component> {
        &self.component
    }

    /// Shortcut for self.component().kind()
    pub fn kind(&self) -> ComponentKind {
        self.component.kind()
    }

    /// Shortcut for self.component().tags_uninherited()
    pub fn tags_uninherited(&self) -> &[String] {
        self.component.tags_uninherited()
    }

    /// Shortcut for self.component().tags()
    pub fn tags(&self) -> impl Iterator<Item = &String> {
        self.component.tags()
    }

    /// Find a component by name.
    pub fn find_by_name<S: AsRef<str>>(
        self: Arc<Self>,
        kind: ComponentKind,
        name: S,
    ) -> Vec<Arc<Outcome>> {
        let name = name.as_ref().to_lowercase();
        let mut outcomes = vec![self];
        let mut found = vec![];

        while let Some(outcome) = outcomes.pop() {
            let outcome_name = outcome.component.name().to_lowercase();
            if kind == outcome.kind() && outcome_name == name {
                found.push(outcome);
                continue;
            }
            outcomes.extend(outcome.children.iter().map(Arc::clone));
        }

        found
    }

    /// Recursively iterate through this outcome and its children for outcomes of type `kind`.
    pub fn iter_components(self: Arc<Self>, kind: ComponentKind) -> IterComponents {
        IterComponents {
            stack: vec![self],
            kind,
        }
    }
}

impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.verdict)?;

        match &self.reason {
            Some(r) => write!(f, " ({})", r)?,
            None => (),
        }

        Ok(())
    }
}

/// Iterator returned by [`Outcome::iter_components`]
pub struct IterComponents {
    stack: Vec<Arc<Outcome>>,
    kind: ComponentKind,
}

impl Iterator for IterComponents {
    type Item = Arc<Outcome>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(outcome) = self.stack.pop() {
            let curr_kind = outcome.kind();
            #[allow(clippy::comparison_chain)] // it's _more_ verbose the way clippy wants it
            if curr_kind == self.kind {
                return Some(outcome);
            } else if curr_kind < self.kind {
                // it's higher level (i.e., it's a feature, and we're looking for scenarios.)
                self.stack.extend(outcome.children.iter().map(Arc::clone));
            }
        }

        None
    }
}
