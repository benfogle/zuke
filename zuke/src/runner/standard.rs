use super::Runner;
use crate::component::{Component, ComponentKind};
use crate::context::OpenContext;
use crate::event::Event;
use crate::outcome::Outcome;
use crate::panic::PanicToError;
use anyhow;
use async_broadcast as broadcast;
use async_std::task;
use async_trait::async_trait;
use futures::channel::mpsc;
use futures::future::join_all;
use futures::stream::{FuturesUnordered, StreamExt};
use std::sync::Arc;

/// The standard test runner
pub struct StandardRunner {}

#[async_trait]
impl Runner for StandardRunner {
    async fn run(
        self: Box<Self>,
        global: Arc<Component>,
        features: mpsc::Receiver<Outcome>,
        events: broadcast::Sender<Event>,
    ) {
        assert_eq!(global.kind(), ComponentKind::Global);
        let _ = self.execute(global, features, events).await;
    }
}

impl Default for StandardRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl StandardRunner {
    /// Create a new `StandardRunner`
    pub fn new() -> Self {
        Self {}
    }

    async fn execute(
        self,
        global: Arc<Component>,
        features: mpsc::Receiver<Outcome>,
        events: broadcast::Sender<Event>,
    ) -> anyhow::Result<()> {
        let mut open = OpenContext::new_global(global);
        let component = open.context.component().clone();
        let mut outcomes = vec![];

        events.broadcast(Event::Started(component)).await?;

        // Pre-test hooks.
        let hooks = open.context.options().pre_test_hooks.clone();
        for hook in hooks.iter() {
            if let Err(e) = PanicToError::from(hook(&mut open.context)).await {
                open.context
                    .outcome_mut()
                    .set_err(anyhow::anyhow!("Pre-test hook failed: {}", e));
                break;
            }
        }

        open.before_hooks().await;

        {
            let mut features = features.fuse();
            let mut pending_features = FuturesUnordered::new();
            loop {
                futures::select! {
                    feat = features.select_next_some() => {
                        let feature_open = open.with_feature(feat);
                        let fut = self.run_feature(feature_open, &events);
                        pending_features.push(fut);
                    },
                    outcome = pending_features.select_next_some() => {
                        match outcome {
                            Err(e) => return Err(e.into()),
                            Ok(o) => outcomes.push(o),
                        };
                    },
                    complete => break,
                }
            }
        }

        open.after_hooks().await;
        let mut outcome = open.finalize().await;
        for o in outcomes {
            outcome.add_child(o);
        }

        let outcome = Arc::new(outcome);
        events.broadcast(Event::Finished(outcome)).await?;

        Ok(())
    }

    async fn run_feature(
        &self,
        mut open: OpenContext,
        events: &broadcast::Sender<Event>,
    ) -> Result<Arc<Outcome>, broadcast::SendError<Event>> {
        assert_eq!(open.context.kind(), ComponentKind::Feature);
        let component = open.context.component().clone();
        let mut outcomes = vec![];

        events.broadcast(Event::Started(component.clone())).await?;

        open.before_hooks().await;

        {
            let mut pending_rules = open
                .with_rules()
                .unwrap()
                .into_iter()
                .map(|r| self.run_rule(r, events))
                .collect::<FuturesUnordered<_>>();
            let mut pending_scenarios = open
                .with_scenarios()
                .unwrap()
                .into_iter()
                .map(|s| self.run_scenario(s, events))
                .collect::<FuturesUnordered<_>>();

            loop {
                let outcome = futures::select! {
                    r = pending_rules.select_next_some() => r,
                    r = pending_scenarios.select_next_some() => r,
                    complete => break,
                }?;

                outcomes.push(outcome);
            }
        }

        open.after_hooks().await;
        for o in outcomes {
            open.context.outcome_mut().add_child(o);
        }

        let outcome = Arc::new(open.finalize().await);
        events.broadcast(Event::Finished(outcome.clone())).await?;
        Ok(outcome)
    }

    async fn run_rule(
        &self,
        mut open: OpenContext,
        events: &broadcast::Sender<Event>,
    ) -> Result<Arc<Outcome>, broadcast::SendError<Event>> {
        assert_eq!(open.context.kind(), ComponentKind::Rule);

        events
            .broadcast(Event::Started(open.context.component().clone()))
            .await?;
        open.before_hooks().await;

        let outcomes;
        {
            let pending = open
                .with_scenarios()
                .unwrap()
                .into_iter()
                .map(|s| self.run_scenario(s, events));

            outcomes = join_all(pending)
                .await
                .into_iter()
                .filter_map(Result::ok)
                .collect::<Vec<_>>();
        }

        open.after_hooks().await;
        for o in outcomes {
            open.context.outcome_mut().add_child(o);
        }

        let outcome = Arc::new(open.finalize().await);
        events.broadcast(Event::Finished(outcome.clone())).await?;
        Ok(outcome)
    }

    async fn run_scenario(
        &self,
        mut open: OpenContext,
        events: &broadcast::Sender<Event>,
    ) -> Result<Arc<Outcome>, broadcast::SendError<Event>> {
        assert_eq!(open.context.kind(), ComponentKind::Scenario);

        // inclusion isn't late evaluated for scenarios
        if !open.context.component().is_included() {
            open.context.outcome_mut().set_excluded();
        }

        let component = open.context.component();
        events.broadcast(Event::Started(component.clone())).await?;

        // spawn a task. This is the part that we want to be truly parallel, and we have less
        // control over what the user ultimately runs. If they block a bit by accident, we don't
        // want to grind to a halt everywhere.
        let outcome = task::spawn(Self::scenario_worker(open, events.clone())).await?;

        let outcome = Arc::new(outcome);
        events.broadcast(Event::Finished(outcome.clone())).await?;
        Ok(outcome)
    }

    async fn scenario_worker(
        mut open: OpenContext,
        events: broadcast::Sender<Event>,
    ) -> Result<Outcome, broadcast::SendError<Event>> {
        let component = open.context.component().clone();
        assert_eq!(component.kind(), ComponentKind::Scenario);
        open.before_hooks().await;

        for step in component.with_background().unwrap() {
            open.set_component(step);
            let outcome = Self::run_step(&mut open, &events).await?;
            open.context.outcome_mut().add_child(outcome);
        }

        for step in component.with_steps().unwrap() {
            open.set_component(step);
            let outcome = Self::run_step(&mut open, &events).await?;
            open.context.outcome_mut().add_child(outcome);
        }

        // Reset to scenario level component before teardown
        open.set_component(component);
        open.after_hooks().await;
        Ok(open.finalize().await)
    }

    async fn run_step(
        open: &mut OpenContext,
        events: &broadcast::Sender<Event>,
    ) -> Result<Arc<Outcome>, broadcast::SendError<Event>> {
        // TODO: This is the most important place to handle cancellation

        let vocab = open.context.options().vocab.clone();
        let component = open.context.component().clone();
        let mut outcome = Outcome::with_parent(component.clone(), open.context.outcome());
        events.broadcast(Event::Started(component)).await?;

        if open.context.outcome().skipped() {
            // Skip with the same type (Excluded/Skipped)
            outcome.verdict = open.context.outcome().verdict;
        } else if open.context.outcome().failed() {
            outcome.set_skip();
        } else {
            let result = vocab.execute(&mut open.context).await;
            outcome.set_result(result);
        }

        let outcome = Arc::new(outcome);
        events.broadcast(Event::Finished(outcome.clone())).await?;
        Ok(outcome)
    }
}
