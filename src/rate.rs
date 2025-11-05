#[cfg(target_arch = "wasm32")]
extern crate instant;
#[cfg(target_arch = "wasm32")]
use instant::{Duration, Instant};
#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};

#[cfg_attr(target_arch = "wasm32", allow(unused))]
pub struct UpdateRate {
    target_rate: Option<Duration>,
    prev_time: Instant,
    delta: Option<Duration>,
}

#[cfg_attr(target_arch = "wasm32", allow(unused))]
impl UpdateRate {
    pub fn new() -> Self {
        Self {
            // Default target rate: 4 ms per frame (~250 FPS)
            target_rate: Some(Duration::from_millis(4)),
            prev_time: Instant::now(),
            delta: None,
        }
    }

    #[inline]
    pub fn set_rate(&mut self, rate: Option<Duration>) {
        self.target_rate = rate;
    }

    pub fn update(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.prev_time);

        // If a target rate is set, sleep to match it
        if let Some(target_rate) = self.target_rate {
            if elapsed < target_rate {
                let sleep_time = target_rate - elapsed;
                std::thread::sleep(sleep_time);
            }
        }

        // Now mark the new frame time and compute total delta (including sleep)
        let now = Instant::now();
        self.delta = Some(now.duration_since(self.prev_time));
        self.prev_time = now;
    }

    #[inline]
    pub fn get_rate(&self) -> Option<Duration> {
        self.delta
    }
}
