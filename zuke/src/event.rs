//! An event sent to reporters

use crate::component::Component;
use crate::outcome::Outcome;
use std::sync::Arc;

/// An event sent to reporters
#[derive(Debug, Clone)]
pub enum Event {
    /// A component has started
    Started(Arc<Component>),
    /// A component has finished.
    Finished(Arc<Outcome>),
}
