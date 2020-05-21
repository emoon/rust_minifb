use std::time::{Duration, Instant};

pub struct UpdateRate {
    target_rate: Option<Duration>,
    prev_time: Instant,
}

impl UpdateRate {
    pub fn new() -> UpdateRate {
        UpdateRate {
            // Default limit to 4 ms
            target_rate: Some(Duration::from_millis(4)),
            prev_time: Instant::now(),
        }
    }

    #[inline]
    pub fn set_rate(&mut self, rate: Option<Duration>) {
        self.target_rate = rate
    }

    pub fn update(&mut self) {
        if let Some(target_rate) = self.target_rate {
            let delta = self.prev_time.elapsed();

            if delta < target_rate {
                let sleep_time = target_rate - delta;
                //eprintln!("sleeping {} ms", sleep_time.as_secs_f64() * 1000.);
                std::thread::sleep(sleep_time);
            }

            self.prev_time = Instant::now();
        }
    }
}
