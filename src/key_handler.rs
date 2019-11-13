extern crate instant;
extern crate time;

use std::mem;
use {InputCallback, Key, KeyRepeat};

pub struct KeyHandler {
    pub key_callback: Option<Box<dyn InputCallback>>,
    prev_time: f64,
    delta_time: f32,
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
            prev_time: self::instant::now(),
            delta_time: 0.0,
            key_repeat_delay: 0.250,
            key_repeat_rate: 0.050,
        }
    }

    #[inline]
    pub fn set_key_state(&mut self, key: Key, state: bool) {
        self.keys[key as usize] = state;
    }

    pub fn get_keys(&self) -> Option<Vec<Key>> {
        let mut index: u16 = 0;
        let mut keys: Vec<Key> = Vec::new();

        for i in self.keys.iter() {
            if *i {
                unsafe {
                    keys.push(mem::transmute(index as u8));
                }
            }

            index += 1;
        }

        Some(keys)
    }

    pub fn update(&mut self) {
        let current_time = time::precise_time_s();
        let delta_time = (current_time - self.prev_time) as f32;
        self.prev_time = current_time;
        self.delta_time = delta_time;

        for i in 0..self.keys.len() {
            if self.keys[i] {
                if self.keys_down_duration[i] < 0.0 {
                    self.keys_down_duration[i] = 0.0;
                } else {
                    self.keys_down_duration[i] += delta_time;
                }
            } else {
                self.keys_down_duration[i] = -1.0;
            }
            self.keys_prev[i] = self.keys[i];
        }
    }

    pub fn set_input_callback(&mut self, callback: Box<dyn InputCallback>) {
        self.key_callback = Some(callback);
    }

    pub fn get_keys_pressed(&self, repeat: KeyRepeat) -> Option<Vec<Key>> {
        let mut index: u16 = 0;
        let mut keys: Vec<Key> = Vec::new();

        for i in self.keys.iter() {
            if *i {
                unsafe {
                    if Self::key_pressed(self, index as usize, repeat) {
                        keys.push(mem::transmute(index as u8));
                    }
                }
            }

            index += 1;
        }

        Some(keys)
    }

    #[inline]
    pub fn is_key_down(&self, key: Key) -> bool {
        return self.keys[key as usize];
    }

    #[inline]
    pub fn set_key_repeat_delay(&mut self, delay: f32) {
        self.key_repeat_delay = delay;
    }

    #[inline]
    pub fn set_key_repeat_rate(&mut self, rate: f32) {
        self.key_repeat_rate = rate;
    }

    pub fn key_pressed(&self, index: usize, repeat: KeyRepeat) -> bool {
        let t = self.keys_down_duration[index];

        if t == 0.0 {
            return true;
        }

        if repeat == KeyRepeat::Yes && t > self.key_repeat_delay {
            let delay = self.key_repeat_delay;
            let rate = self.key_repeat_rate;
            if (((t - delay) % rate) > rate * 0.5)
                != (((t - delay - self.delta_time) % rate) > rate * 0.5)
            {
                return true;
            }
        }

        return false;
    }

    #[inline]
    pub fn is_key_pressed(&self, key: Key, repeat: KeyRepeat) -> bool {
        return Self::key_pressed(self, key as usize, repeat);
    }

    #[inline]
    pub fn is_key_released(&self, key: Key) -> bool {
        let idx = key as usize;
        return self.keys_prev[idx] && !self.keys[idx];
    }
}
