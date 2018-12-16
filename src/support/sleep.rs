pub async fn sleep_ms(ms: u64) {
    use futures::compat::Future01CompatExt;
    use std::time::{Duration, Instant};
    use tokio::timer::Delay;

    let when = Instant::now() + Duration::from_millis(ms);

    await!(Delay::new(when).compat()).expect("Error running sleep");
}
