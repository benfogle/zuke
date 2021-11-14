//! Implements before/after hook functions, and tag expressions.

use crate::{ComponentKind, Context, Fixture, Scope};
use async_trait::async_trait;
use futures::future::BoxFuture;

/// Simple, stack based operations for tag expressions
#[derive(Debug)]
pub enum Operation {
    /// Push a tag value (true, false) on the stack. Inherited tags
    Push(String),
    /// Push a tag value (true, false) on the stack. Non-inherited tags
    PushUninherited(String),
    /// Invert the top of the stack
    Not,
    /// AND the top two items on the stack
    And,
    /// OR the top two items on the stack
    Or,
}

/// Evaulate a tag expression. `stack` should be an empty vec. Re-used for efficiency.
fn eval_expr(ops: &[Operation], context: &Context, stack: &mut Vec<bool>) -> bool {
    // Most common case is 0 tags, probably few enough that it's not worth a hash table
    let uninherited = context.tags_uninherited();
    let tags = context.tags().collect::<Vec<_>>();

    stack.reserve(ops.len());
    for op in ops {
        match op {
            Operation::Push(s) => {
                stack.push(tags.iter().any(|t| *t == s));
            }
            Operation::PushUninherited(s) => {
                stack.push(uninherited.contains(s));
            }
            Operation::Not => {
                let last = stack.last_mut().expect("Mis-parsed tag expression");
                *last = !*last;
            }
            Operation::And => {
                let rhs = stack.pop().expect("Mis-parsed tag expression");
                let lhs = stack.pop().expect("Mis-parsed tag expression");
                stack.push(lhs && rhs);
            }
            Operation::Or => {
                let rhs = stack.pop().expect("Mis-parsed tag expression");
                let lhs = stack.pop().expect("Mis-parsed tag expression");
                stack.push(lhs || rhs);
            }
        }
    }

    assert!(stack.len() <= 1, "Mis-parsed tag expression");
    stack.pop().unwrap_or(true)
}

/// Should a `BeforeAfterHook` run before or after? Usually macro generated
#[allow(missing_docs)]
pub enum BeforeAfter {
    Before,
    After,
}

/// Used to register a hook. Usually macro generated
pub struct BeforeAfterHook {
    /// Is this a before or after hook?
    pub when: BeforeAfter,
    /// This triggers before/after this type of component
    pub kind: ComponentKind,
    /// The function to call
    pub func: for<'a> fn(&'a mut Context) -> BoxFuture<'a, anyhow::Result<()>>,
    /// The tag expression. May be empty.
    pub expr: Vec<Operation>,
}
inventory::collect!(BeforeAfterHook);

#[derive(Default)]
struct HookSet {
    before: Vec<&'static BeforeAfterHook>,
    after: Vec<&'static BeforeAfterHook>,
}

/// A fixture that runs before and after hooks defined as functions
#[derive(Default)]
pub(crate) struct HookRunner {
    global: HookSet,
    feature: HookSet,
    rule: HookSet,
    scenario: HookSet,
    step: HookSet,
}

#[async_trait]
impl Fixture for HookRunner {
    const SCOPE: Scope = Scope::Global;

    async fn setup(_context: &mut Context) -> anyhow::Result<Self> {
        let mut hooks = Self::default();
        for hook in inventory::iter::<BeforeAfterHook> {
            let set = match hook.kind {
                ComponentKind::Global => &mut hooks.global,
                ComponentKind::Feature => &mut hooks.feature,
                ComponentKind::Rule => &mut hooks.rule,
                ComponentKind::Scenario => &mut hooks.scenario,
                ComponentKind::Step => &mut hooks.step,
            };

            let set = match hook.when {
                BeforeAfter::Before => &mut set.before,
                BeforeAfter::After => &mut set.after,
            };

            set.push(hook);
        }

        Ok(hooks)
    }

    async fn before(&self, context: &mut Context) -> anyhow::Result<()> {
        let set = match context.kind() {
            ComponentKind::Global => &self.global,
            ComponentKind::Feature => &self.feature,
            ComponentKind::Rule => &self.rule,
            ComponentKind::Scenario => &self.scenario,
            ComponentKind::Step => &self.step,
        };

        let mut stack = vec![];
        for hook in set.before.iter() {
            if eval_expr(&hook.expr, context, &mut stack) {
                (hook.func)(context).await?;
            }
        }

        Ok(())
    }

    async fn after(&self, context: &mut Context) -> anyhow::Result<()> {
        let set = match context.kind() {
            ComponentKind::Global => &self.global,
            ComponentKind::Feature => &self.feature,
            ComponentKind::Rule => &self.rule,
            ComponentKind::Scenario => &self.scenario,
            ComponentKind::Step => &self.step,
        };

        let mut stack = vec![];
        for hook in set.after.iter() {
            if eval_expr(&hook.expr, context, &mut stack) {
                (hook.func)(context).await?;
            }
        }

        Ok(())
    }
}
