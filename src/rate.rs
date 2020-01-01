use std::time::Duration;

pub struct UpdateRate {
	target_rate: Option<Duration>,
	prev_time: f64,
}

impl UpdateRate {
	pub fn new() -> UpdateRate {
		UpdateRate {
			// Default limit to 4 ms
			target_rate: Some(Duration::from_millis(4)),
			prev_time: 0.0,
		}
	}

	#[inline]
	pub fn set_rate(&mut self, rate: Option<Duration>) {
		self.target_rate = rate
	}

	pub fn update(&mut self) {
		if let Some(rate) = self.target_rate {
			let target_rate = rate.as_secs_f64();
			let current_time = time::precise_time_s();
			let delta = current_time - self.prev_time;

			if delta < target_rate {
				let sleep_time = target_rate - delta;
				if sleep_time > 0.0 {
					//println!("sleeping {} ms", sleep_time * 1000.0);
					std::thread::sleep(Duration::from_secs_f64(sleep_time));
				}
			}

			self.prev_time = time::precise_time_s();
		}
	}
}
