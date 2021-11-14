use async_std::future::timeout;
use async_std::sync::Barrier;
use async_trait::async_trait;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Mutex;
use std::time::Duration;
use zuke::{when, Context, Fixture, Scope};

struct Barriers {
    barriers: Mutex<HashMap<(usize, String), Pin<Box<Barrier>>>>,
}

#[async_trait]
impl Fixture for Barriers {
    const SCOPE: Scope = Scope::Feature;
    async fn setup(_context: &mut Context) -> anyhow::Result<Self> {
        Ok(Self {
            barriers: Mutex::default(),
        })
    }
}

impl Barriers {
    async fn wait_for<'a, S: Into<String>>(
        &'a self,
        keyword: S,
        count: usize,
        timeout_dur: Duration,
    ) -> anyhow::Result<()> {
        let key = (count, keyword.into());
        let barrier: &'a Barrier;

        {
            let mut map = self.barriers.lock().unwrap();
            let barrier_local = &**map
                .entry(key)
                .or_insert_with(|| Box::pin(Barrier::new(count)));
            // outlive the mutex lock. Since this is a write-only struct, and since we have a
            // pinned pointer on the heap, this is fine.
            barrier = unsafe { std::mem::transmute::<&'_ Barrier, &'a Barrier>(barrier_local) };
        }

        let fut = barrier.wait();
        timeout(timeout_dur, fut).await?;
        Ok(())
    }
}

#[when("I wait for {n} scenarios to say \"{word}\"")]
async fn wait_for_others(context: &mut Context, n: usize, word: &str) -> anyhow::Result<()> {
    context.use_fixture::<Barriers>().await?;

    let barrier = context.fixture::<Barriers>().await;
    barrier.wait_for(word, n, Duration::from_secs(30)).await?;
    Ok(())
}
