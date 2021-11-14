//! Misc things for implementing steps

use crate::outcome::Verdict;
use std::error::Error;
use std::fmt;

/// A special error type that may be returned from a step implementation (or fixture
/// setup/teardown/etc.) to cause other effects aside from failing the test.
///
/// Internally, all errors returned from a step, hook, or fixture are transparently converted to
/// this type, so users only need to return it directly if they want to skip or cause some other
/// action to happen.
pub struct StepError {
    /// The type of "error" (failed, skipped, etc.)
    /// Even Passed is allowed, though it wouldn't make much sense.
    pub verdict: Verdict,
    /// Optional reason, which will be displayed if present
    pub reason: Option<anyhow::Error>,
}

impl fmt::Debug for StepError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.verdict, f)?;
        if let Some(reason) = &self.reason {
            write!(f, ": {:?}", reason)?;
        }
        Ok(())
    }
}

impl fmt::Display for StepError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.verdict, f)?;
        if let Some(reason) = &self.reason {
            write!(f, ": {}", reason)?;
        }
        Ok(())
    }
}

impl Error for StepError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.reason.as_ref().map(|e| e.as_ref())
    }
}

impl From<anyhow::Error> for StepError {
    fn from(e: anyhow::Error) -> Self {
        Self::fail_with_reason(e)
    }
}

impl StepError {
    /// Fail the component with no message
    pub fn fail() -> Self {
        Self {
            verdict: Verdict::Failed,
            reason: None,
        }
    }

    /// Fail the component with an error message
    pub fn fail_with_reason<E: Into<anyhow::Error>>(reason: E) -> Self {
        Self {
            verdict: Verdict::Failed,
            reason: Some(reason.into()),
        }
    }

    /// Fail the component with a string message
    pub fn fail_with_message<M: Into<String>>(message: M) -> Self {
        Self {
            verdict: Verdict::Failed,
            reason: Some(anyhow::anyhow!(message.into())),
        }
    }

    /// Skip the component with no message
    pub fn skip() -> Self {
        Self {
            verdict: Verdict::Skipped,
            reason: None,
        }
    }

    /// Skip the component with an error message
    pub fn skip_with_reason<E: Into<anyhow::Error>>(reason: E) -> Self {
        Self {
            verdict: Verdict::Skipped,
            reason: Some(reason.into()),
        }
    }

    /// Skip the component with a string message
    pub fn skip_with_message<M: Into<String>>(message: M) -> Self {
        Self {
            verdict: Verdict::Skipped,
            reason: Some(anyhow::anyhow!(message.into())),
        }
    }

    /// Pass with warnings. No message.
    ///
    /// Doesn't make a lot of sense, but here for consistency.
    pub fn warn() -> Self {
        Self {
            verdict: Verdict::PassedWithWarnings,
            reason: None,
        }
    }

    /// Pass with warnings, with an error message
    pub fn warn_with_reason<E: Into<anyhow::Error>>(reason: E) -> Self {
        Self {
            verdict: Verdict::PassedWithWarnings,
            reason: Some(reason.into()),
        }
    }

    /// Pass with warnings, with a string message
    pub fn warn_with_message<M: Into<String>>(message: M) -> Self {
        Self {
            verdict: Verdict::PassedWithWarnings,
            reason: Some(anyhow::anyhow!(message.into())),
        }
    }

    /// Cancel with no message
    pub fn cancel() -> Self {
        Self {
            verdict: Verdict::Canceled,
            reason: None,
        }
    }

    /// Cancel with an error message
    pub fn cancel_with_reason<E: Into<anyhow::Error>>(reason: E) -> Self {
        Self {
            verdict: Verdict::Canceled,
            reason: Some(reason.into()),
        }
    }

    /// Cancel with a string message
    pub fn cancel_with_message<M: Into<String>>(message: M) -> Self {
        Self {
            verdict: Verdict::Canceled,
            reason: Some(anyhow::anyhow!(message.into())),
        }
    }
}

/// Fail the component. Note that `anyhow::bail!` or simply returning an error will work equally
/// well for failing.
#[macro_export]
macro_rules! fail {
    () => {{
        return ::std::result::Result::Err($crate::step::StepError::fail().into());
    }};
    ($msg:tt) => {{
        return ::std::result::Result::Err(
            $crate::step::StepError::fail_with_reason(anyhow::anyhow!($msg)).into(),
        );
    }};
}

/// Skip the component.
#[macro_export]
macro_rules! skip {
    () => {{
        return ::std::result::Result::Err($crate::step::StepError::skip().into());
    }};
    ($msg:tt) => {{
        return ::std::result::Result::Err(
            $crate::step::StepError::skip_with_reason(anyhow::anyhow!($msg)).into(),
        );
    }};
}

/// Pass the component (with warnings)
#[macro_export]
macro_rules! warn {
    ($msg:tt) => {{
        return ::std::result::Result::Err(
            $crate::step::StepError::warn_with_reason(anyhow::anyhow!($msg)).into(),
        );
    }};
}

/// Cancel the component.
#[macro_export]
macro_rules! cancel {
    () => {{
        return Err($crate::step::StepError::cancel().into());
    }};
    ($msg:tt) => {{
        return ::std::result::Result::Err(
            $crate::step::StepError::cancel_with_reason(anyhow::anyhow!($msg)).into(),
        );
    }};
}
