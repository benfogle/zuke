use futures::future::pending;
use zuke::*;

#[when("I pause forever")]
async fn pause_forever() {
    let () = pending().await;
}
