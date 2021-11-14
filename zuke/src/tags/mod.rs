#![warn(missing_docs)]

//! Default tags for Zuke

use crate::{before_all, Context};
use futures::future::{BoxFuture, FutureExt};
pub mod fail;
pub mod skip;

#[before_all]
async fn add_default_tags(context: &mut Context) -> anyhow::Result<()> {
    context.use_fixture::<skip::Skip>().await?;
    context.use_fixture::<fail::Fail>().await?;
    Ok(())
}

/// Pre-test hook to add default tag handling to Zuke
pub fn default_tags(context: &mut Context) -> BoxFuture<'_, anyhow::Result<()>> {
    add_default_tags(context).boxed()
}
