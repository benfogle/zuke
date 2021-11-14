use async_trait::async_trait;
use zuke::*;

struct TaggedFixture;
struct InheritedFixture;
struct NonInheritedFixture;
struct AndFixture;
struct OrFixture;

#[async_trait]
impl Fixture for TaggedFixture {
    async fn setup(_context: &mut Context) -> anyhow::Result<Self> {
        Ok(Self)
    }
}

#[async_trait]
impl Fixture for InheritedFixture {
    async fn setup(_context: &mut Context) -> anyhow::Result<Self> {
        Ok(Self)
    }
}

#[async_trait]
impl Fixture for NonInheritedFixture {
    async fn setup(_context: &mut Context) -> anyhow::Result<Self> {
        Ok(Self)
    }
}

#[async_trait]
impl Fixture for AndFixture {
    async fn setup(_context: &mut Context) -> anyhow::Result<Self> {
        Ok(Self)
    }
}

#[async_trait]
impl Fixture for OrFixture {
    async fn setup(_context: &mut Context) -> anyhow::Result<Self> {
        Ok(Self)
    }
}

#[before_scenario("@use-a-fixture")]
async fn tagged_fixture(context: &mut Context) -> anyhow::Result<()> {
    context.use_fixture::<TaggedFixture>().await
}

#[before_scenario("@use-a-fixture-left and @use-a-fixture-right")]
async fn and_fixture(context: &mut Context) -> anyhow::Result<()> {
    context.use_fixture::<AndFixture>().await
}

#[before_scenario("@use-a-fixture-left or @use-a-fixture-right")]
async fn or_fixture(context: &mut Context) -> anyhow::Result<()> {
    context.use_fixture::<OrFixture>().await
}

#[before_scenario("@inherited-tag")]
async fn inherited_fixture(context: &mut Context) -> anyhow::Result<()> {
    context.use_fixture::<InheritedFixture>().await
}

#[before_scenario("@@non-inherited-tag")]
async fn non_inherited_fixture(context: &mut Context) -> anyhow::Result<()> {
    context.use_fixture::<NonInheritedFixture>().await
}

#[then("the TaggedFixture fixture is present")]
async fn check_tagged(context: &mut Context) {
    context.fixture::<TaggedFixture>().await;
}

#[then("the InheritedFixture fixture is present")]
async fn check_inherit(context: &mut Context) {
    context.fixture::<InheritedFixture>().await;
}

#[then("the NonInheritedFixture fixture is not present")]
async fn check_no_noninherit(context: &mut Context) {
    assert!(context.try_fixture::<NonInheritedFixture>().await.is_none());
}

#[then("the AndFixture fixture is present")]
async fn check_and(context: &mut Context) {
    context.fixture::<AndFixture>().await;
}

#[then("the AndFixture fixture is not present")]
async fn check_and_not(context: &mut Context) {
    assert!(context.try_fixture::<AndFixture>().await.is_none());
}

#[then("the OrFixture fixture is present")]
async fn check_or(context: &mut Context) {
    context.fixture::<OrFixture>().await;
}
