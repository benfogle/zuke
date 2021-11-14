//! Failure related tags

use crate::*;
use async_trait::async_trait;

/// A fixture that implements `@fail` tags
///
/// Unlike most tags, these tags aren't inherited: an @expect-fail tag at the feature level does
/// *not* imply that every single scenario in the feature is expected to fail: only that at least
/// one will.
pub struct Fail;

#[async_trait]
impl Fixture for Fail {
    const SCOPE: Scope = Scope::Global;

    async fn setup(_context: &mut Context) -> anyhow::Result<Self> {
        Ok(Self)
    }

    async fn before(&self, context: &mut Context) -> anyhow::Result<()> {
        if context.component().step().is_some() {
            return Ok(());
        }

        if context.tags_uninherited().iter().any(|t| *t == "fail") {
            fail!()
        }

        Ok(())
    }

    async fn after(&self, context: &mut Context) -> anyhow::Result<()> {
        if context.component().step().is_some() {
            return Ok(());
        }

        // for borrowing rules, plus keeping double tags from negating each other.
        let mut found_expect_fail = false;
        let mut found_fail_as_warning = false;

        for tag in context.tags_uninherited().iter() {
            match tag.as_str() {
                "expect-fail" => found_expect_fail = true,
                "fail-as-warning" => found_fail_as_warning = true,
                _ => (),
            }
        }

        if found_expect_fail {
            expect_fail(context)?;
        }

        if found_fail_as_warning {
            fail_as_warning(context)?;
        }

        Ok(())
    }
}

fn expect_fail(context: &mut Context) -> anyhow::Result<()> {
    let outcome = context.outcome_mut();
    outcome.verdict = match outcome.verdict {
        Verdict::Passed | Verdict::PassedWithWarnings => Verdict::UnexpectedPass,
        Verdict::Failed => Verdict::ExpectedFailure,
        _ => outcome.verdict,
    };

    Ok(())
}

fn fail_as_warning(context: &mut Context) -> anyhow::Result<()> {
    let outcome = context.outcome_mut();
    outcome.verdict = match outcome.verdict {
        Verdict::Failed => Verdict::PassedWithWarnings,
        _ => outcome.verdict,
    };

    Ok(())
}
