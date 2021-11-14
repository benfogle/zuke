//! Test Runner

use crate::component::Component;
use crate::event::Event;
use crate::outcome::Outcome;
use async_broadcast as broadcast;
use async_trait::async_trait;
use futures::channel::mpsc;
use std::sync::Arc;

mod standard;
pub use standard::*;

/// A runner consumes features from a [`crate::parser::Parser`], runs tests, and sends the outcomes
/// to a [`crate::reporter::Reporter`].
#[async_trait]
pub trait Runner: Send + Sync {
    /// Run the tests
    async fn run(
        self: Box<Self>,
        global: Arc<Component>,
        features: mpsc::Receiver<Outcome>,
        events: broadcast::Sender<Event>,
    );
}
