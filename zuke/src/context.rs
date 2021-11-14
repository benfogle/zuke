//! Defines a test context object
//!
//! A context is an outcome plus active fixtures. When the test component is done running, the
//! fixtures will be jettisoned and the outcome will be passed along to reporters.

use crate::component::{Component, ComponentKind, NewComponentError};
use crate::fixture::{Fixture, FixtureError, FixtureSet, Scope};
use crate::options::TestOptions;
use crate::outcome::Outcome;
use async_std::task;
use gherkin_rust::{Feature, Rule, Scenario, Step};
use std::any::TypeId;
use std::sync::Arc;

/// The test context is a combination of the current test component (i.e., scenario, step, feature,
/// etc.), the currently active test fixtures, and any other information needed to execute a test.
pub struct Context {
    options: Arc<TestOptions>,
    component: Arc<Component>,
    outcome: Outcome,
    global_fixtures: Option<Arc<FixtureSet>>, // an option for teardown
    feature_fixtures: Option<Arc<FixtureSet>>,
    scenario_fixtures: Option<Arc<FixtureSet>>, // only an arc to keep the borrow checker happy
}

/// An "open" context is a context that can be used to derive other contexts. They are used by
/// [`crate::runner::Runner`] objects, and users generally won't ever touch them.
///
/// By contrast, a "closed" context is the regular type of [`crate::Context`] seen by users.
/// We don't pass open contexts to users because they could then cause fixtures to remiain valid
/// past their indented scopes.
pub struct OpenContext {
    /// The closed context
    pub context: Context,
}

impl OpenContext {
    /// A new global context
    pub fn new_global(component: Arc<Component>) -> Self {
        let outcome = Outcome::undecided(component.clone());
        let options = component.options().clone();

        Self {
            context: Context {
                options,
                component,
                outcome,
                global_fixtures: Some(Arc::new(FixtureSet::new())),
                feature_fixtures: None,
                scenario_fixtures: None,
            },
        }
    }

    /// Derive a feature context from a global context
    ///
    /// By default in a skipped state if the global test is failing
    pub fn with_feature(&self, feature: Outcome) -> Self {
        // should be called on a global context, but we can revert to global easily enough.
        let component = feature.component().clone();

        Self {
            context: Context {
                options: self.context.options.clone(),
                component,
                outcome: feature,
                global_fixtures: self.context.global_fixtures.clone(),
                feature_fixtures: Some(Arc::new(FixtureSet::new())),
                scenario_fixtures: None,
            },
        }
    }

    /// Derive a rule context from a feature context
    ///
    /// By default in a skipped state if the feature is failing
    pub fn with_rules(&self) -> Result<Vec<Self>, NewComponentError> {
        Ok(self
            .context
            .outcome
            .component()
            .with_rules()?
            .into_iter()
            .map(|component| Self {
                context: Context {
                    options: self.context.options.clone(),
                    outcome: Outcome::with_parent(component.clone(), &self.context.outcome),
                    component,
                    global_fixtures: self.context.global_fixtures.clone(),
                    feature_fixtures: self.context.feature_fixtures.clone(),
                    scenario_fixtures: None,
                },
            })
            .collect())
    }

    /// Derive a scenario context from a feature or rule context
    ///
    /// By default in a skipped state if the rule is failing
    pub fn with_scenarios(&self) -> Result<Vec<Self>, NewComponentError> {
        Ok(self
            .context
            .outcome
            .component()
            .with_scenarios()?
            .into_iter()
            .map(|component| Self {
                context: Context {
                    options: self.context.options.clone(),
                    outcome: Outcome::with_parent(component.clone(), &self.context.outcome),
                    component,
                    global_fixtures: self.context.global_fixtures.clone(),
                    feature_fixtures: self.context.feature_fixtures.clone(),
                    scenario_fixtures: Some(Arc::new(FixtureSet::new())),
                },
            })
            .collect())
    }

    /// Sets the component and nothing else. For step execution where we mutate the context serially
    /// rather than derive new contexts.
    pub fn set_component(&mut self, component: Arc<Component>) {
        self.context.component = component;
    }

    /// Run the before hooks (fixtures).
    pub async fn before_hooks(&mut self) {
        // TODO: better handling of multiple errors

        let fixture_sets = [
            self.context.global_fixtures.clone(),
            self.context.feature_fixtures.clone(),
            self.context.scenario_fixtures.clone(),
        ];

        for fixtures in fixture_sets.iter().flatten() {
            if let Err(e) = fixtures.before(&mut self.context).await {
                self.context
                    .outcome_mut()
                    .set_err(e.context("Error in before hook"));
            }
        }
    }

    /// Run the after hooks (fixtures)
    pub async fn after_hooks(&mut self) {
        // TODO: better handling of multiple errors

        let fixture_sets = [
            self.context.scenario_fixtures.clone(),
            self.context.feature_fixtures.clone(),
            self.context.global_fixtures.clone(),
        ];

        for fixtures in fixture_sets.iter().flatten() {
            if let Err(e) = fixtures.after(&mut self.context).await {
                self.context
                    .outcome_mut()
                    .set_err(e.context("Error in before hook"));
            }
        }
    }

    /// Tear down fixtures and return the final result.
    ///
    /// Fixtures that are going out of scope are sent to a blocking thread to drop, because there
    /// is no async-drop yet.
    pub async fn finalize(self) -> Outcome {
        // No Async-drop, so we will do our best to drop fixtures on a background thread.
        let Self { mut context, .. } = self;

        // On implementation trick is that we must ensure that all non-unique Arc's are decremented
        // by the end of the function. If we just push everything to a background task, the feature
        // fixtures pointers from a scenario might still be hanging around by the time the feature
        // itself is ready to be torn down.
        async fn do_teardown<'a>(
            context: &'a mut Context,
            fixtures: Option<Arc<FixtureSet>>,
            kind: ComponentKind,
            panicmsg: &'static str,
        ) {
            if context.kind() != kind {
                return;
            }

            if let Some(mut f) = fixtures {
                let result = Arc::get_mut(&mut f)
                    .expect(panicmsg)
                    .teardown(context)
                    .await;
                if let Err(e) = result {
                    context.outcome.set_err(e);
                }
                // No async drop, so we'll do this in the background
                let _ = task::spawn_blocking(move || drop(f));
            }
        }

        // Explicit clean up of this scope's fixtures, and check that it's going to be dropped.
        // Clean up from scenario on backwards, closing them off for further modification as we go.
        // TODO: We can try to track feature dependencies and remove them one at a time so that
        // fixtures can still access dependent fixtures
        let scenario_fixtures = context.scenario_fixtures.take();
        do_teardown(
            &mut context,
            scenario_fixtures,
            ComponentKind::Scenario,
            "Scenario fixtures are still in use at scenario end",
        )
        .await;

        let feature_fixtures = context.feature_fixtures.take();
        do_teardown(
            &mut context,
            feature_fixtures,
            ComponentKind::Feature,
            "Feature fixtures are still in use at feature end",
        )
        .await;

        let global_fixtures = context.global_fixtures.take();
        do_teardown(
            &mut context,
            global_fixtures,
            ComponentKind::Global,
            "Global fixtures are still in use at test run end",
        )
        .await;

        let Context { mut outcome, .. } = context;
        if outcome.is_undecided() {
            // Late evaluation of inclusion
            if outcome.component().is_included() {
                outcome.set_passed();
            } else {
                outcome.set_excluded();
            }
        }
        outcome
    }
}

impl Context {
    /// The global test options
    pub fn options(&self) -> &TestOptions {
        &self.options
    }

    /// Attempt to get a fixture. If the fixture is not *already* in use, this returns `None`.
    ///
    /// This function is async because it is possible for the fixture to be in the process of being
    /// set up in another scenario. In that case it will return `Some` once the fixture is ready.
    pub async fn try_fixture<T: Fixture>(&self) -> Option<&T> {
        match T::SCOPE {
            Scope::Global => self.global_fixtures.as_ref()?.get().await,
            Scope::Feature => self.feature_fixtures.as_ref()?.get().await,
            Scope::Scenario => self.scenario_fixtures.as_ref()?.get().await,
        }
    }

    /// Attempt to get a fixture. If the fixture is not *already* in use, this function *panics*.
    pub async fn fixture<T: Fixture>(&self) -> &T {
        self.try_fixture()
            .await
            .unwrap_or_else(|| panic!("No feature {:?} in current context", TypeId::of::<T>()))
    }

    /// As `try_fixture`, but attempts to get a *mutable* reference to the fixture. Returns `None`
    /// if the fixture is not *already* in use or if the fixture is in use by multiple tests.
    ///
    /// In general, in a step implementation you can only obtain a mutable reference for
    /// scenario-scoped fixture.
    ///
    /// You can get mutable references for feature-scoped features when executing at the feature
    /// scope. This happens:
    ///
    /// 1. In the `setup()` function of a feature-scoped fixture
    /// 2. When a `before()` or `after()` hook executes for a feature.
    ///
    /// Globally-scoped fixtures work similarly.
    pub async fn try_fixture_mut<T: Fixture>(&mut self) -> Option<&mut T> {
        // Merging these match arms seems to confuse the borrow checker
        match T::SCOPE {
            Scope::Global => match self.global_fixtures {
                Some(ref mut f) => Arc::get_mut(f)?.get_mut().await,
                None => None,
            },
            Scope::Feature => match self.feature_fixtures {
                Some(ref mut f) => Arc::get_mut(f)?.get_mut().await,
                None => None,
            },
            Scope::Scenario => match self.scenario_fixtures {
                Some(ref mut f) => Arc::get_mut(f)?.get_mut().await,
                None => None,
            },
        }
    }

    /// As `try_fixture_mut`, but panics if the reference cannot be obtained.
    pub async fn fixture_mut<T: Fixture>(&mut self) -> &mut T {
        // Merging these match arms seems to confuse the borrow checker
        let not_mut = &format!("Cannot use {:?} mutably in this context", TypeId::of::<T>());
        let not_found = &format!("Cannot use {:?} mutably in this context", TypeId::of::<T>());

        match T::SCOPE {
            Scope::Global => match self.global_fixtures {
                Some(ref mut f) => Arc::get_mut(f).expect(not_mut).get_mut().await,
                None => None,
            },
            Scope::Feature => match self.feature_fixtures {
                Some(ref mut f) => Arc::get_mut(f).expect(not_mut).get_mut().await,
                None => None,
            },
            Scope::Scenario => match self.scenario_fixtures {
                Some(ref mut f) => Arc::get_mut(f).expect(not_mut).get_mut().await,
                None => None,
            },
        }
        .expect(not_found)
    }

    /// Activate a fixture. This must be called before `get_fixture`, etc., will
    /// work.
    pub async fn use_fixture<T: Fixture>(&mut self) -> anyhow::Result<()> {
        // increment reference count to make the borrow checker happy
        let set = match T::SCOPE {
            Scope::Global => self.global_fixtures.clone(),
            Scope::Feature => self.feature_fixtures.clone(),
            Scope::Scenario => self.scenario_fixtures.clone(),
        };

        match set {
            Some(f) => f.activate::<T>(self).await,
            None => Err(anyhow::anyhow!(FixtureError::WrongScope)),
        }
    }

    /// Current scope, as it pertains to fixtures. [`Self::kind`] is finer-grained and usually what you
    /// want.
    pub fn fixture_scope(&self) -> Scope {
        if self.component.feature().is_none() {
            Scope::Global
        } else if self.component.scenario().is_none() {
            Scope::Feature
        } else {
            Scope::Scenario
        }
    }

    /// The current test component
    pub fn component(&self) -> &Arc<Component> {
        &self.component
    }

    /// Shortcut for `self.component().feature()`
    pub fn feature(&self) -> Option<&Feature> {
        self.component.feature()
    }

    /// Shortcut for `self.component().rule()`
    pub fn rule(&self) -> Option<&Rule> {
        self.component.rule()
    }

    /// Shortcut for `self.component().scenario()`
    pub fn scenario(&self) -> Option<&Scenario> {
        self.component.scenario()
    }

    /// Shortcut for `self.component().step()`
    pub fn step(&self) -> Option<&Step> {
        self.component.step()
    }

    /// Shortcut for `self.component().kind()`
    pub fn kind(&self) -> ComponentKind {
        self.component.kind()
    }

    /// Shortcut for `self.component().tags()`
    pub fn tags_uninherited(&self) -> &[String] {
        self.component.tags_uninherited()
    }

    /// Shortcut for `self.component().tags_uninherited()`
    pub fn tags(&self) -> impl Iterator<Item = &String> {
        self.component.tags()
    }

    /// Shortcut for `self.component().name()`
    pub fn name(&self) -> &str {
        self.component.name()
    }

    /// The in-progress outcome
    pub fn outcome(&self) -> &Outcome {
        &self.outcome
    }

    /// The in-progress outcome. Fixtures and step implementations are allowed to manipulate the
    /// test outcome directly, though returning an error is usually easier.
    pub fn outcome_mut(&mut self) -> &mut Outcome {
        &mut self.outcome
    }
}
