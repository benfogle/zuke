//! A set-once flag. Used for cancellation.
use async_std::channel::{bounded, Receiver, Sender};
use std::sync::{Arc, Mutex};

/// A global flag future. It will be pending until the flag is set. Used for cancellation.
#[derive(Clone)]
pub struct Flag {
    recv: Receiver<()>,
    send: Arc<Mutex<Option<Sender<()>>>>,
}

impl Default for Flag {
    fn default() -> Self {
        Self::new()
    }
}

impl Flag {
    /// Create a new flag, initially unset
    pub fn new() -> Self {
        let (send, recv) = bounded(1);
        Self {
            recv,
            send: Arc::new(Mutex::new(Some(send))),
        }
    }

    /// Wait until the flag is set
    pub async fn wait(&self) {
        let _ = self.recv.recv().await;
    }

    /// Set the flag
    pub fn set(&self) {
        // close the channel
        let mut send = self.send.lock().unwrap();
        let _ = send.take();
    }
}
