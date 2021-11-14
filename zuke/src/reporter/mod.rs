//! Writes output given test outcomes

use crate::component::Component;
use crate::event::Event;
use anyhow;
use async_broadcast as broadcast;
use async_std::io::Stdout;
use async_trait::async_trait;
use std::sync::Arc;

pub mod collect;
pub mod command_line;
pub mod plain;
pub use collect::*;
pub use command_line::*;
pub use plain::*;

/// A Reporter takes [`crate::Event`]s from a [`crate::runner::Runner`] and creates an output
/// report from them.
#[async_trait]
pub trait Reporter: Send + Sync {
    /// Create an output report from input events. The return value is used to determine the final
    /// exit code.
    async fn report(
        self: Box<Self>,
        global: Arc<Component>,
        events: broadcast::Receiver<Event>,
    ) -> anyhow::Result<()>;
}

/// The default type of reporter to create if none are specified
pub type DefaultReporter = PlainReporter<Stdout>;
