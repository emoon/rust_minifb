#[cfg(target_arch = "wasm32")]
extern crate instant;

#[cfg(target_arch = "wasm32")]
use instant::{Duration, Instant};
#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};

use crate::{InputCallback, Key, KeyRepeat};

pub struct KeyHandler {
    pub key_callback: Option<Box<dyn InputCallback>>,
    prev_time: Instant,
    delta_time: Duration,
    keys: [bool; 512],
    keys_prev: [bool; 512],
    keys_down_duration: [f32; 512],
    key_repeat_delay: f32,
    key_repeat_rate: f32,
}

impl KeyHandler {
    pub fn new() -> KeyHandler {
        KeyHandler {
            key_callback: None,
            keys: [false; 512],
            keys_prev: [false; 512],
            keys_down_duration: [-1.0; 512],
            prev_time: Instant::now(),
            delta_time: Duration::from_secs(0),
            key_repeat_delay: 0.250,
            key_repeat_rate: 0.050,
        }
    }

    #[inline]
    pub fn set_key_state(&mut self, key: Key, state: bool) {
        self.keys[key as usize] = state;
        if let Some(cb) = &mut self.key_callback {
            cb.set_key_state(key, state);
        }
    }

    pub fn get_keys(&self) -> Vec<Key> {
        let mut keys: Vec<Key> = Vec::new();

        for (idx, is_down) in self.keys.iter().enumerate() {
            if *is_down {
                unsafe {
                    keys.push(std::mem::transmute::<u8, Key>(idx as u8));
                }
            }
        }

        keys
    }

    pub fn update(&mut self) {
        self.delta_time = self.prev_time.elapsed();
        self.prev_time = Instant::now();
        let delta_time = self.delta_time.as_secs_f32();

        for idx in 0..self.keys.len() {
            if self.keys[idx] {
                if self.keys_down_duration[idx] < 0.0 {
                    self.keys_down_duration[idx] = 0.0;
                } else {
                    self.keys_down_duration[idx] += delta_time;
                }
            } else {
                self.keys_down_duration[idx] = -1.0;
            }
            self.keys_prev[idx] = self.keys[idx];
        }
    }

    #[inline]
    pub fn set_input_callback(&mut self, callback: Box<dyn InputCallback>) {
        self.key_callback = Some(callback);
    }

    pub fn get_keys_pressed(&self, repeat: KeyRepeat) -> Vec<Key> {
        let mut keys: Vec<Key> = Vec::new();

        for (idx, is_down) in self.keys.iter().enumerate() {
            if *is_down && self.is_key_index_pressed(idx, repeat) {
                unsafe {
                    keys.push(std::mem::transmute::<u8, Key>(idx as u8));
                }
            }
        }

        keys
    }

    pub fn get_keys_released(&self) -> Vec<Key> {
        let mut keys: Vec<Key> = Vec::new();

        for (idx, is_down) in self.keys.iter().enumerate() {
            if !(*is_down) && self.is_key_index_released(idx) {
                unsafe {
                    keys.push(std::mem::transmute::<u8, Key>(idx as u8));
                }
            }
        }

        keys
    }

    #[inline]
    pub fn is_key_down(&self, key: Key) -> bool {
        self.keys[key as usize]
    }

    #[inline]
    pub fn set_key_repeat_delay(&mut self, delay: f32) {
        self.key_repeat_delay = delay;
    }

    #[inline]
    pub fn set_key_repeat_rate(&mut self, rate: f32) {
        self.key_repeat_rate = rate;
    }

    fn is_key_index_pressed(&self, index: usize, repeat: KeyRepeat) -> bool {
        let t = self.keys_down_duration[index];

        if t == 0.0 {
            return true;
        }

        if repeat == KeyRepeat::Yes && t > self.key_repeat_delay {
            let delta_time = self.delta_time.as_secs_f32();
            let delay = self.key_repeat_delay;
            let rate = self.key_repeat_rate;
            if (((t - delay) % rate) > rate * 0.5)
                != (((t - delay - delta_time) % rate) > rate * 0.5)
            {
                return true;
            }
        }

        false
    }

    #[inline]
    pub fn is_key_pressed(&self, key: Key, repeat: KeyRepeat) -> bool {
        self.is_key_index_pressed(key as usize, repeat)
    }

    #[inline]
    pub fn is_key_released(&self, key: Key) -> bool {
        self.is_key_index_released(key as usize)
    }

    #[inline]
    fn is_key_index_released(&self, idx: usize) -> bool {
        self.keys_prev[idx] && !self.keys[idx]
    }
}
