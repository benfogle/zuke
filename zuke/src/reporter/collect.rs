//! A trivial reporter that grabs the top-level result
use super::Reporter;
use crate::component::{Component, ComponentKind};
use crate::event::Event;
use crate::outcome::Outcome;
use anyhow;
use async_broadcast as broadcast;
use async_trait::async_trait;
use futures::channel::oneshot;
use futures::StreamExt;
use std::sync::Arc;

/// A reporter that just send the final outcome somewhere. Often useful for tests or custom
/// follow-on processing.
pub struct Collect {
    dest: oneshot::Sender<Arc<Outcome>>,
}

impl Collect {
    /// Create a new `Collect` object and a corresponding receiver for the top-level outcome
    pub fn new() -> (Self, oneshot::Receiver<Arc<Outcome>>) {
        let (tx, rx) = oneshot::channel();
        (Self { dest: tx }, rx)
    }
}

#[async_trait]
impl Reporter for Collect {
    async fn report(
        self: Box<Self>,
        _global: Arc<Component>,
        mut events: broadcast::Receiver<Event>,
    ) -> anyhow::Result<()> {
        let mut final_outcome = None;

        while let Some(event) = events.next().await {
            if let Event::Finished(outcome) = event {
                if outcome.kind() == ComponentKind::Global {
                    assert!(final_outcome.is_none());
                    final_outcome = Some(outcome);
                }
            }
        }

        let outcome = final_outcome.expect("No final test result received");
        let _ = self.dest.send(outcome);
        Ok(())
    }
}
