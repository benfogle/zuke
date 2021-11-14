//! Test fixtures

use crate::context::Context;
use crate::panic::PanicToError;
use async_std::channel;
use async_std::sync::{RwLock, RwLockUpgradableReadGuard};
use async_trait::async_trait;
use futures::future::{BoxFuture, FutureExt};
use std::any::{Any, TypeId};
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::fmt;
use std::pin::Pin;
use thiserror::Error;

/// An error that can occur when creating a fixture
#[derive(Error, Debug)]
pub enum FixtureError {
    /// The fixture setup function failed
    #[error("Fixture setup failed in another step")]
    Failed,
    /// Attempted to, e.g., create a scenario-scoped fixture at a global scope. (The other way
    /// around is fine.)
    #[error("Fixture is not valid in this scope")]
    WrongScope,
}

/// The fixture scope. More coarse than `ComponentKind`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Scope {
    /// Global fixtures
    Global,
    /// per-Feature fixtures
    Feature,
    /// per-Scenario fixtures
    Scenario,
}

/// A fixture sets up a known state for a test, and tears it down once done. Fixtures objects have
/// a scope:
///
/// * Scenario scoped fixtures are unique to the scenario. They are destroyed at the end of the
///   scenario.
/// * Feature scoped fixtures are shared by all scenarios in a single feature. They are destroyed
///   at the end of the feature. These are useful for things that require expensive but long lived
///   setup, such as a sample database or a virtual machine.
/// * Globally scoped fixtures are shared by all scenarios in the test. They are not destroyed
///   until the end of the test run.
///
/// Fixtures can also observe (and modify) test execution during their lifetime using
/// [`Self::before`] and [`Self::after`] hooks.
///
/// Fixtures are dropped on a background thread, after teardown, so it is acceptable to block on
/// drop.
#[async_trait]
pub trait Fixture: Any + Send + Sync + Sized + 'static {
    /// The scope of this fixture. Default is [`Scope::Scenario`]
    const SCOPE: Scope = Scope::Scenario;

    /// Called to create the fixture in reponse to `Context::use_fixture`. This is generally
    /// mid-scenario, regardless of scope, but may also occur during before hooks if one fixture
    /// depends on another.
    ///
    /// Errors here will be returned to the scenario, typically resulting in test failure.
    async fn setup(context: &mut Context) -> anyhow::Result<Self>;

    /// Called just before fixture teardown. This happens after the fixture's scope ends. So, e.g.,
    /// a feature-level fixture will have this called just after the last scenario in the feature
    /// ends.
    ///
    /// Only fixtures at a higher scope are available for you to use here. For example, a
    /// scenario-level fixture can use feature- and global-level fixtures, but not other
    /// scenario-level fixtures.
    ///
    /// Errors here will cause the scenario, feature, or test run to fail, depending on the scope.
    async fn teardown(&mut self, _context: &mut Context) -> anyhow::Result<()> {
        Ok(())
    }

    /// Called when a feature or scenario begins. This function will not be called prior to fixture
    /// setup, so a global-level fixture will start to receive these callbacks only after it has
    /// first been set up: scenarios that finished prior will be missed. Similarly, a feature-level
    /// fixture will never receive a "before-feature" hook, because it had not yet been created.
    ///
    /// To receive this hook for _all_ scenarios, create a global fixture using
    /// `ZukeBuilder::use_fixture`.
    ///
    /// Returning an error from this function will cause the component to fail, and any scenarios
    /// inside to be skipped.
    async fn before(&self, _context: &mut Context) -> anyhow::Result<()> {
        Ok(())
    }

    /// Called when a test component ends. This function will not be called prior to fixture setup,
    /// so the same caveats apply as for `before`.
    ///
    /// Returning an error from this function will cause the component to fail.
    async fn after(&self, _context: &mut Context) -> anyhow::Result<()> {
        Ok(())
    }
}

type FixtureFuncMut = for<'a> fn(
    &'a mut (dyn Any + Send + Sync + 'static),
    &'a mut Context,
) -> BoxFuture<'a, anyhow::Result<()>>;

type FixtureFunc = for<'a> fn(
    &'a (dyn Any + Send + Sync + 'static),
    &'a mut Context,
) -> BoxFuture<'a, anyhow::Result<()>>;

type EntryCallbackFn =
    for<'a> fn(&'a FixtureEntry, &'a mut Context) -> BoxFuture<'a, anyhow::Result<()>>;

trait EntryCallback:
    for<'a> Fn(&'a FixtureEntry, &'a mut Context) -> BoxFuture<'a, anyhow::Result<()>>
{
}
impl EntryCallback for EntryCallbackFn {}

/// This is mostly a workaround for the fact that Fixture is not object safe. Instead we make our
/// own vtable. This helps us hide some of the grossness from the end users.
struct FixtureEntry {
    fixture: Box<dyn Any + Send + Sync + 'static>,
    teardown: FixtureFuncMut,
    before: FixtureFunc,
    after: FixtureFunc,
}

impl FixtureEntry {
    fn new<F: Fixture>(fixture: F) -> Self {
        fn teardown<'a, F: Fixture>(
            f: &'a mut (dyn Any + Send + Sync + 'static),
            c: &'a mut Context,
        ) -> BoxFuture<'a, anyhow::Result<()>> {
            let f: &mut F = f.downcast_mut().expect("Internal type error");
            f.teardown(c)
        }

        fn before<'a, F: Fixture>(
            f: &'a (dyn Any + Send + Sync + 'static),
            c: &'a mut Context,
        ) -> BoxFuture<'a, anyhow::Result<()>> {
            let f: &F = f.downcast_ref().expect("Internal type error");
            f.before(c)
        }

        fn after<'a, F: Fixture>(
            f: &'a (dyn Any + Send + Sync + 'static),
            c: &'a mut Context,
        ) -> BoxFuture<'a, anyhow::Result<()>> {
            let f: &F = f.downcast_ref().expect("Internal type error");
            f.after(c)
        }

        Self {
            fixture: Box::new(fixture),
            teardown: teardown::<F>,
            before: before::<F>,
            after: after::<F>,
        }
    }

    fn downcast_ref<F: Fixture>(&self) -> Option<&F> {
        self.fixture.downcast_ref()
    }

    fn downcast_mut<F: Fixture>(&mut self) -> Option<&mut F> {
        self.fixture.downcast_mut()
    }

    async fn teardown(&mut self, context: &mut Context) -> anyhow::Result<()> {
        PanicToError::from((self.teardown)(&mut *self.fixture, context)).await
    }

    async fn before(&self, context: &mut Context) -> anyhow::Result<()> {
        PanicToError::from((self.before)(&*self.fixture, context)).await
    }

    async fn after(&self, context: &mut Context) -> anyhow::Result<()> {
        PanicToError::from((self.after)(&*self.fixture, context)).await
    }
}

enum FixtureState {
    Pending(channel::Receiver<()>),
    // If we need to release the lock for some computation, we want to hold a valid &FixtureEntry
    // across the boundary, even if new items are inserted in the meantime.
    Ready(Pin<Box<FixtureEntry>>),
    Failed,
}

type FixtureHash = HashMap<TypeId, FixtureState>;

/// Holds fixtures at a single scope
pub(crate) struct FixtureSet {
    // Because this is a write-only structure, we can relax some of the restrictions around
    // locking.  In particular, we can return an immutable reference that outlives the lock itself.
    // Even if the hashtable udpates or moves while the reference is active, it's a reference to a
    // location on the heap that will not be affected. To make rust happy with all of this, our
    // lock is a separate object.
    lock: RwLock<()>,
    fixtures: UnsafeCell<FixtureHash>,
}

unsafe impl Sync for FixtureSet {}

impl Default for FixtureSet {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for FixtureSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<FixtureSet>")
    }
}

impl FixtureSet {
    /// create a new fixture set
    pub fn new() -> Self {
        Self {
            fixtures: UnsafeCell::new(HashMap::new()),
            lock: RwLock::new(()),
        }
    }

    fn get_unlocked<T: Fixture>(&self) -> Option<&T> {
        let fixtures: &FixtureHash = unsafe { &*self.fixtures.get() };
        let key = TypeId::of::<T>();
        let state = fixtures.get(&key);
        match state {
            Some(FixtureState::Ready(entry)) => Some(
                entry
                    .downcast_ref::<T>()
                    .expect("Internal error: bad fixture type"),
            ),
            _ => None,
        }
    }

    /// Get a reference to a fixture, if possible
    pub async fn get<T: Fixture>(&self) -> Option<&T> {
        let _lock = self.lock.read().await;
        self.get_unlocked()
    }

    fn get_mut_unlocked<T: Fixture>(&mut self) -> Option<&mut T> {
        let fixtures = self.fixtures.get_mut();
        let key = TypeId::of::<T>();
        let state = fixtures.get_mut(&key);
        match state {
            Some(FixtureState::Ready(entry)) => Some(
                entry
                    .downcast_mut::<T>()
                    .expect("Internal error: bad fixture type"),
            ),
            _ => None,
        }
    }

    /// Get a mutable reference, if possible
    pub async fn get_mut<T: Fixture>(&mut self) -> Option<&mut T> {
        // Compile-time checks mean we don't have to lock. There can only be one at a time.
        self.get_mut_unlocked()
    }

    /// Call only with the lock held. Insulates raw pointer such that Rust doesn't try to hold on
    /// to it across an await boundary, which is not Send.
    unsafe fn get_hash(&self) -> &FixtureHash {
        &*self.fixtures.get()
    }

    /// Call only with the write lock held. Insulates raw pointer such that Rust doesn't try to
    /// hold on to it across an await boundary, which is not Send.
    #[allow(clippy::mut_from_ref)]
    unsafe fn get_hash_mut(&self) -> &mut FixtureHash {
        &mut *self.fixtures.get()
    }

    /// Activate a fixture.
    pub async fn activate<T: Fixture>(&self, context: &mut Context) -> anyhow::Result<()> {
        let lock = self.lock.upgradable_read().await;
        let key = TypeId::of::<T>();
        let fixtures = unsafe { self.get_hash() };
        let state = fixtures.get(&key);

        match state {
            Some(FixtureState::Ready(_)) => Ok(()),
            Some(FixtureState::Pending(r)) => {
                let wait = r.clone();
                drop(lock);
                let _ = wait.recv().await;
                Ok(())
            }
            Some(FixtureState::Failed) => Err(anyhow::anyhow!(FixtureError::Failed)),
            None => {
                let lock = RwLockUpgradableReadGuard::upgrade(lock).await;
                let fixtures = unsafe { self.get_hash_mut() };
                let (_tx, rx) = channel::bounded(1);
                fixtures.insert(key, FixtureState::Pending(rx));

                // unlock so that the fixture can use other fixtures
                drop(lock);
                let result = self.create_fixture::<T>(context).await;
                let _lock = self.lock.write().await;

                match result {
                    Ok(e) => {
                        fixtures.insert(key, FixtureState::Ready(Box::pin(e)));
                        Ok(())
                    }
                    Err(e) => {
                        fixtures.insert(key, FixtureState::Failed);
                        Err(e)
                    }
                }

                // _tx drop will release anyone else waiting
            }
        }
    }

    /// Tear down all fixtures in this scope.
    pub async fn teardown(&mut self, context: &mut Context) -> anyhow::Result<()> {
        // no locking required due to &mut self
        let mut errors = vec![];
        let fixtures = self.fixtures.get_mut();

        for fixture in fixtures.values_mut() {
            match fixture {
                FixtureState::Ready(entry) => {
                    if let Err(e) = entry.teardown(context).await {
                        errors.push(e);
                    }
                }
                FixtureState::Pending(_) => {
                    panic!("Teardown while a fixture is being set up");
                }
                _ => (),
            }
        }

        if errors.is_empty() {
            Ok(())
        } else if errors.len() == 1 {
            Err(errors.drain(..).next().unwrap())
        } else {
            // todo pass them all along
            Err(anyhow::anyhow!("Multiple errors in teardown"))
        }
    }

    /// Call all before hooks in this scope
    pub async fn before(&self, context: &mut Context) -> anyhow::Result<()> {
        self.for_each_fixture(|e, c| e.before(c).boxed(), context)
            .await
    }

    /// Call all after hooks in this scope
    pub async fn after(&self, context: &mut Context) -> anyhow::Result<()> {
        self.for_each_fixture(|e, c| e.after(c).boxed(), context)
            .await
    }

    async fn create_fixture<T: Fixture>(
        &self,
        context: &mut Context,
    ) -> anyhow::Result<FixtureEntry> {
        let fixture = T::setup(context).await?;
        Ok(FixtureEntry::new(fixture))
    }

    async fn for_each_fixture<F>(&self, callback: F, context: &mut Context) -> anyhow::Result<()>
    where
        F: for<'a> Fn(&'a FixtureEntry, &'a mut Context) -> BoxFuture<'a, anyhow::Result<()>>,
    {
        let mut result = Ok(());
        let fixtures = unsafe { self.get_hash() }; // only use with lock held

        // we only promise that fixtures will see components after they have been set up. That
        // doesn't include whatever is happening right now. We will only go through this list once,
        // and anyone who isn't in place loses out.
        let keys: Vec<TypeId> = {
            let _lock = self.lock.read().await;
            fixtures.keys().map(Clone::clone).collect()
        };

        // From here on out we hold the lock as little as possible so that our fixtures can create
        // other fixtures as they need to.
        for id in keys {
            let fut = {
                let _lock = self.lock.read().await;
                match fixtures.get(&id).unwrap() {
                    FixtureState::Ready(entry) => callback(entry, context),
                    _ => continue,
                }
            };

            // TODO: handle multiple errors better
            if let Err(e) = fut.await {
                result = Err(e)
            }
        }

        result
    }
}
