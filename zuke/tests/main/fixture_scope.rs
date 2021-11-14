use async_trait::async_trait;
use std::sync::atomic::{AtomicU32, Ordering};
use zuke::*;

struct ScenarioCounter {
    count: AtomicU32,
    expected: u32,
}

#[async_trait]
impl Fixture for ScenarioCounter {
    const SCOPE: Scope = Scope::Scenario;

    async fn setup(_context: &mut Context) -> anyhow::Result<Self> {
        Ok(Self {
            count: AtomicU32::new(0),
            expected: 1,
        })
    }

    async fn teardown(&mut self, _context: &mut Context) -> anyhow::Result<()> {
        assert_eq!(
            *self.count.get_mut(),
            self.expected,
            "scenario counter is wrong"
        );
        Ok(())
    }
}

#[given("a counter fixture with scenario scope, that should be 1 on teardown")]
async fn get_scenario_counter(context: &mut Context) -> anyhow::Result<()> {
    context.use_fixture::<ScenarioCounter>().await?;
    Ok(())
}

#[when("I increment the scenario counter")]
async fn inc_scenario_counter(context: &mut Context) -> anyhow::Result<()> {
    let counter = context.fixture::<ScenarioCounter>().await;
    counter.count.fetch_add(1, Ordering::Relaxed);
    Ok(())
}

struct FeatureCounter {
    count: AtomicU32,
    expected: u32,
}

#[async_trait]
impl Fixture for FeatureCounter {
    const SCOPE: Scope = Scope::Feature;

    async fn setup(_context: &mut Context) -> anyhow::Result<Self> {
        Ok(Self {
            count: AtomicU32::new(0),
            expected: 2,
        })
    }

    async fn teardown(&mut self, _context: &mut Context) -> anyhow::Result<()> {
        assert_eq!(
            *self.count.get_mut(),
            self.expected,
            "feature counter is wrong"
        );
        Ok(())
    }
}

#[given("a counter fixture with feature scope, that should be 2 on teardown")]
async fn get_feature_counter(context: &mut Context) -> anyhow::Result<()> {
    context.use_fixture::<FeatureCounter>().await?;
    Ok(())
}

#[when("I increment the feature counter")]
async fn inc_feature_counter(context: &mut Context) -> anyhow::Result<()> {
    let counter = context.fixture::<FeatureCounter>().await;
    counter.count.fetch_add(1, Ordering::Relaxed);
    Ok(())
}

struct GlobalCounter {
    count: AtomicU32,
    expected: u32,
}

#[async_trait]
impl Fixture for GlobalCounter {
    const SCOPE: Scope = Scope::Global;

    async fn setup(_context: &mut Context) -> anyhow::Result<Self> {
        Ok(Self {
            count: AtomicU32::new(0),
            expected: 4,
        })
    }

    async fn teardown(&mut self, _context: &mut Context) -> anyhow::Result<()> {
        assert_eq!(
            *self.count.get_mut(),
            self.expected,
            "global counter is wrong"
        );
        Ok(())
    }
}

#[given("a counter fixture with global scope, that should be 4 on teardown")]
async fn get_global_counter(context: &mut Context) -> anyhow::Result<()> {
    context.use_fixture::<GlobalCounter>().await?;
    Ok(())
}

#[when("I increment the global counter")]
async fn inc_global_counter(context: &mut Context) -> anyhow::Result<()> {
    let counter = context.fixture::<GlobalCounter>().await;
    counter.count.fetch_add(1, Ordering::Relaxed);
    Ok(())
}
