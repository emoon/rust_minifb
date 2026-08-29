use web_time::{Duration, Instant};

use crate::{InputCallback, Key, KeyRepeat};

/// Number of slots in the key state arrays.
///
/// `Key` is `#[repr(u8)]` with contiguous discriminants `0..=Key::Count`, so
/// every index below this is a valid `Key` and every `key as usize` is in
/// range.
const KEY_COUNT: usize = Key::Count as usize + 1;

/// Converts a key state array index back into the `Key` it belongs to.
#[inline]
fn key_from_index(idx: usize) -> Option<Key> {
    if idx < Key::Count as usize {
        // SAFETY: `Key` is `#[repr(u8)]` and its discriminants are contiguous
        // from 0 to `Key::Count`, so every `idx < Key::Count` is a valid
        // discriminant. `Key::Count` itself is a sentinel and is excluded.
        Some(unsafe { std::mem::transmute::<u8, Key>(idx as u8) })
    } else {
        None
    }
}

pub struct KeyHandler {
    pub key_callback: Option<Box<dyn InputCallback>>,
    prev_time: Instant,
    delta_time: Duration,
    keys: [bool; KEY_COUNT],
    keys_prev: [bool; KEY_COUNT],
    keys_down_duration: [f32; KEY_COUNT],
    key_repeat_delay: f32,
    key_repeat_rate: f32,
}

impl KeyHandler {
    pub fn new() -> KeyHandler {
        KeyHandler {
            key_callback: None,
            keys: [false; KEY_COUNT],
            keys_prev: [false; KEY_COUNT],
            keys_down_duration: [-1.0; KEY_COUNT],
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
                keys.extend(key_from_index(idx));
            }
        }

        keys
    }

    /// Snapshots the current key levels as "previous", which is what
    /// `is_key_index_released` compares against.
    ///
    /// A backend that applies platform key events itself must call this
    /// *before* applying them: the release edge is `keys_prev && !keys`, so
    /// snapshotting after a release has already been written to `keys` makes
    /// both sides false and destroys the edge before the caller can read it.
    pub fn snapshot_prev(&mut self) {
        self.keys_prev.copy_from_slice(&self.keys);
    }

    /// Advances each key's held duration from the levels currently in `keys`.
    ///
    /// A backend that applies platform key events itself must call this
    /// *after* applying them: `is_key_index_pressed` reports a key only while
    /// its duration is exactly `0.0`, and a key pressed in this batch is still
    /// at the initial `-1.0` until this runs. Advancing before the batch is
    /// applied defers every press by a cycle, and loses it outright if the
    /// release lands in the next batch.
    pub fn advance_durations(&mut self) {
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
        }
    }

    /// Both phases together, for backends that apply their platform key
    /// events outside the window this call spans (X11, macOS, Windows), where
    /// the ordering the two phases exist to separate does not arise.
    #[allow(dead_code)] // the Wayland backend calls the two phases directly
    pub fn update(&mut self) {
        self.snapshot_prev();
        self.advance_durations();
    }

    #[inline]
    pub fn set_input_callback(&mut self, callback: Box<dyn InputCallback>) {
        self.key_callback = Some(callback);
    }

    pub fn get_keys_pressed(&self, repeat: KeyRepeat) -> Vec<Key> {
        let mut keys: Vec<Key> = Vec::new();

        for (idx, is_down) in self.keys.iter().enumerate() {
            if *is_down && self.is_key_index_pressed(idx, repeat) {
                keys.extend(key_from_index(idx));
            }
        }

        keys
    }

    pub fn get_keys_released(&self) -> Vec<Key> {
        let mut keys: Vec<Key> = Vec::new();

        for (idx, is_down) in self.keys.iter().enumerate() {
            if !(*is_down) && self.is_key_index_released(idx) {
                keys.extend(key_from_index(idx));
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Pins the contiguity that `key_from_index`'s transmute relies on.
    #[test]
    fn every_index_below_count_round_trips() {
        for idx in 0..Key::Count as usize {
            let key = key_from_index(idx).expect("index below Count must map to a Key");
            assert_eq!(key as usize, idx);
        }
        assert_eq!(key_from_index(Key::Count as usize), None);
        assert_eq!(key_from_index(usize::MAX), None);
    }

    #[test]
    fn state_array_covers_every_key() {
        let mut handler = KeyHandler::new();
        handler.set_key_state(Key::Unknown, true);
        assert!(handler.is_key_down(Key::Unknown));
        assert_eq!(handler.get_keys(), vec![Key::Unknown]);
    }
}
