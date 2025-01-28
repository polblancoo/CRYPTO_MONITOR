use std::time::Duration;
use tokio::time;

pub struct Timer {
    interval: Duration,
}

impl Timer {
    pub fn new(seconds: u64) -> Self {
        Self {
            interval: Duration::from_secs(seconds),
        }
    }

    pub async fn start<F, Fut>(&self, mut task: F)
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        let mut interval = time::interval(self.interval);
        loop {
            interval.tick().await;
            task().await;
        }
    }
} 