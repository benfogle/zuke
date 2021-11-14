//! Helpers for dealing with panics in user code. Used internally, but is public because it's used
//! by macros.

use futures::future::{CatchUnwind, FutureExt};
use std::any::Any;
use std::ffi::{OsStr, OsString};
use std::future::Future;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll};

pub struct PanicToError<F>(F);

impl<T, E, F> From<F> for PanicToError<F>
where
    E: Into<anyhow::Error>,
    F: FnOnce() -> Result<T, E>,
{
    fn from(func: F) -> Self {
        Self(func)
    }
}

impl<T, E, F> PanicToError<F>
where
    E: Into<anyhow::Error> + Send + Sync,
    F: FnOnce() -> Result<T, E>,
{
    pub fn call_once(self) -> anyhow::Result<T> {
        let Self(func) = self;
        flatten(catch_unwind(AssertUnwindSafe(func)))
    }
}

impl<T, E, F> From<F> for PanicToError<CatchUnwind<AssertUnwindSafe<F>>>
where
    E: Into<anyhow::Error>,
    F: Future<Output = Result<T, E>>,
{
    fn from(fut: F) -> Self {
        Self(AssertUnwindSafe(fut).catch_unwind())
    }
}

impl<T, E, F> Future for PanicToError<F>
where
    E: Into<anyhow::Error> + Send + Sync,
    F: Future<Output = std::thread::Result<Result<T, E>>>,
{
    type Output = anyhow::Result<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<Self::Output> {
        // structurally pinned
        let f = unsafe { self.map_unchecked_mut(|s| &mut s.0) };
        match f.poll(cx) {
            Poll::Ready(result) => Poll::Ready(flatten(result)),
            Poll::Pending => Poll::Pending,
        }
    }
}

fn flatten<T, E>(result: std::thread::Result<Result<T, E>>) -> anyhow::Result<T>
where
    E: Into<anyhow::Error> + Send + Sync,
{
    match result {
        Ok(r) => r.map_err(|e| e.into()),
        Err(panic) => Err(to_error(panic)),
    }
}

fn to_error(panic: Box<dyn Any + Send + 'static>) -> anyhow::Error {
    if let Some(msg) = panic.downcast_ref::<&str>() {
        anyhow::anyhow!(msg.to_string())
    } else if let Some(msg) = panic.downcast_ref::<String>() {
        anyhow::anyhow!(msg.clone())
    } else if let Some(msg) = panic.downcast_ref::<&OsStr>() {
        anyhow::anyhow!(msg.to_string_lossy().to_owned())
    } else if let Some(msg) = panic.downcast_ref::<&OsString>() {
        anyhow::anyhow!(msg.to_string_lossy().to_owned())
    } else {
        anyhow::anyhow!("Panicked! (No message available)")
    }
}
