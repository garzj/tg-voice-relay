use std::time::Duration;

use reqwest::StatusCode;
use tokio::time;

use crate::backoff::{Backoff, BackoffVariant};

pub struct Heartbeat {
    endpoint: String,
    interval: Duration,
}

impl Heartbeat {
    pub fn new(endpoint: String, interval: Duration) -> Heartbeat {
        Heartbeat { endpoint, interval }
    }

    pub async fn task(self) {
        let mut interval = time::interval(self.interval);
        interval.set_missed_tick_behavior(time::MissedTickBehavior::Delay);

        let mut backoff = Backoff::new(BackoffVariant::Exponential, Some(3600));

        loop {
            let res = reqwest::get(&self.endpoint).await;
            let res = match res {
                Ok(res) => res,
                Err(err) => {
                    log::error!("request to health endpoint failed: {}", err);
                    time::sleep(Duration::from_secs(backoff.next())).await;
                    continue;
                }
            };
            let status = res.status();
            match status {
                StatusCode::OK => {
                    backoff.reset();
                }
                _ => {
                    log::error!(
                        "request to health endpoint returned bad status code: {}",
                        status
                    );
                    time::sleep(Duration::from_secs(backoff.next())).await;
                    continue;
                }
            }

            interval.tick().await;
        }
    }
}
