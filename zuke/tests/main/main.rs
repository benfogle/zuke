use async_std::task::block_on;
use zuke::Zuke;

mod cancel;
mod capture;
mod concurrent;
mod fixture_scope;
mod hooks;
mod implementations;
mod matches;
mod sub_instance;

fn main() -> anyhow::Result<()> {
    let zuke = Zuke::builder().feature_path("tests/features").build()?;
    block_on(zuke.run())
}
