use anyhow;
use zuke::{given, then};

#[given("a step that returns nothing")]
#[given("a lever long enough")]
#[given("a place to stand")]
#[then("I will move the world")]
fn given_a_lever() {}

#[given("an async step that returns nothing")]
#[given("an async lever long enough")]
#[given("an async place to stand")]
#[then("I will move the world without blocking")]
async fn given_a_lever_async() {}

#[given("a step that returns Ok from anyhow::Result")]
fn returns_anyhow_ok() -> anyhow::Result<()> {
    Ok(())
}

#[given("a step that returns Ok from std::io::Result")]
fn returns_io_ok() -> std::io::Result<()> {
    Ok(())
}

#[given("a step that returns Ok(42) from std::io::Result")]
fn returns_io_ok_42() -> std::io::Result<i32> {
    Ok(42)
}

#[given("an async step that returns Ok from anyhow::Result")]
async fn returns_anyhow_ok_async() -> anyhow::Result<()> {
    Ok(())
}

#[given("an async step that returns Ok from std::io::Result")]
async fn returns_io_ok_async() -> std::io::Result<()> {
    Ok(())
}

#[given("an async step that returns Ok(42) from std::io::Result")]
async fn returns_io_ok_42_async() -> std::io::Result<i32> {
    Ok(42)
}

#[given("a step that panics")]
#[given("I shouldn't get here")]
fn panics() {
    panic!("PANIC!");
}

#[given("a step that return Err from anyhow::Result")]
fn err_anyhow() -> anyhow::Result<()> {
    anyhow::bail!("error!");
}

#[given("a step that return Err from std::io::Result")]
fn err_io() -> std::io::Result<()> {
    Err(std::io::Error::new(std::io::ErrorKind::Other, "I/O error"))
}

#[given("a step that is implemented twice")]
fn multiple_1() {}

#[given("a step that is implemented twice")]
fn multiple_2() {}
