use std::collections::{HashMap, HashSet};

use crate::KeyCode;

#[derive(Debug, Default, Clone)]
pub struct InputState {
    pressed_keys: HashSet<KeyCode>,
    just_pressed_keys: HashSet<KeyCode>,
    just_released_keys: HashSet<KeyCode>,
    active_touches: HashMap<u64, TouchPoint>,
    just_pressed_touches: HashSet<u64>,
    just_released_touches: HashSet<u64>,
}

impl InputState {
    pub fn press(&mut self, key: KeyCode) {
        if self.pressed_keys.insert(key) {
            self.just_pressed_keys.insert(key);
            self.just_released_keys.remove(&key);
        }
    }

    pub fn release(&mut self, key: KeyCode) {
        if self.pressed_keys.remove(&key) {
            self.just_released_keys.insert(key);
            self.just_pressed_keys.remove(&key);
        }
    }

    pub fn pressed(&self, key: KeyCode) -> bool {
        self.pressed_keys.contains(&key)
    }

    pub fn just_pressed(&self, key: KeyCode) -> bool {
        self.just_pressed_keys.contains(&key)
    }

    pub fn just_released(&self, key: KeyCode) -> bool {
        self.just_released_keys.contains(&key)
    }

    pub fn end_frame(&mut self) {
        self.just_pressed_keys.clear();
        self.just_released_keys.clear();
        self.just_pressed_touches.clear();
        self.just_released_touches.clear();
    }

    pub fn is_key_down(&self, key: KeyCode) -> bool {
        self.pressed(key)
    }

    pub fn touch_start(&mut self, id: u64, x: f32, y: f32) {
        self.active_touches.insert(id, TouchPoint::new(id, x, y));
        self.just_pressed_touches.insert(id);
        self.just_released_touches.remove(&id);
    }

    pub fn touch_move(&mut self, id: u64, x: f32, y: f32) {
        if let Some(touch) = self.active_touches.get_mut(&id) {
            touch.position = (x, y);
        }
    }

    pub fn touch_end(&mut self, id: u64) {
        if self.active_touches.remove(&id).is_some() {
            self.just_released_touches.insert(id);
            self.just_pressed_touches.remove(&id);
        }
    }

    pub fn touch_cancel(&mut self, id: u64) {
        self.touch_end(id);
    }

    pub fn touch_pressed(&self, id: u64) -> bool {
        self.active_touches.contains_key(&id)
    }

    pub fn touch_just_pressed(&self, id: u64) -> bool {
        self.just_pressed_touches.contains(&id)
    }

    pub fn touch_just_released(&self, id: u64) -> bool {
        self.just_released_touches.contains(&id)
    }

    pub fn touch(&self, id: u64) -> Option<&TouchPoint> {
        self.active_touches.get(&id)
    }

    pub fn touches(&self) -> impl Iterator<Item = &TouchPoint> {
        self.active_touches.values()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TouchPoint {
    pub id: u64,
    pub position: (f32, f32),
}

impl TouchPoint {
    pub const fn new(id: u64, x: f32, y: f32) -> Self {
        Self {
            id,
            position: (x, y),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_press_sets_pressed_and_just_pressed() {
        let mut input = InputState::default();

        input.press(KeyCode::ArrowRight);

        assert!(input.pressed(KeyCode::ArrowRight));
        assert!(input.just_pressed(KeyCode::ArrowRight));
        assert!(!input.just_released(KeyCode::ArrowRight));
    }

    #[test]
    fn repeated_press_does_not_repeat_just_pressed_while_held() {
        let mut input = InputState::default();

        input.press(KeyCode::ArrowRight);
        input.end_frame();

        input.press(KeyCode::ArrowRight);

        assert!(input.pressed(KeyCode::ArrowRight));
        assert!(!input.just_pressed(KeyCode::ArrowRight));
        assert!(!input.just_released(KeyCode::ArrowRight));
    }

    #[test]
    fn release_sets_just_released_once_and_clears_pressed() {
        let mut input = InputState::default();

        input.press(KeyCode::ArrowRight);
        input.end_frame();

        input.release(KeyCode::ArrowRight);

        assert!(!input.pressed(KeyCode::ArrowRight));
        assert!(!input.just_pressed(KeyCode::ArrowRight));
        assert!(input.just_released(KeyCode::ArrowRight));

        input.end_frame();

        assert!(!input.pressed(KeyCode::ArrowRight));
        assert!(!input.just_pressed(KeyCode::ArrowRight));
        assert!(!input.just_released(KeyCode::ArrowRight));
    }

    #[test]
    fn end_of_frame_clears_transition_flags_but_keeps_held_keys() {
        let mut input = InputState::default();

        input.press(KeyCode::ArrowLeft);
        input.end_frame();

        assert!(input.pressed(KeyCode::ArrowLeft));
        assert!(!input.just_pressed(KeyCode::ArrowLeft));
        assert!(!input.just_released(KeyCode::ArrowLeft));
    }

    #[test]
    fn multiple_keys_remain_independent() {
        let mut input = InputState::default();

        input.press(KeyCode::ArrowLeft);
        input.press(KeyCode::ArrowRight);
        input.end_frame();
        input.release(KeyCode::ArrowLeft);

        assert!(!input.pressed(KeyCode::ArrowLeft));
        assert!(input.just_released(KeyCode::ArrowLeft));
        assert!(input.pressed(KeyCode::ArrowRight));
        assert!(!input.just_pressed(KeyCode::ArrowRight));
        assert!(!input.just_released(KeyCode::ArrowRight));
    }

    #[test]
    fn touch_lifecycle_tracks_active_and_transition_state() {
        let mut input = InputState::default();

        input.touch_start(7, 12.0, 34.0);

        assert!(input.touch_pressed(7));
        assert!(input.touch_just_pressed(7));
        assert!(!input.touch_just_released(7));
        assert_eq!(
            input.touch(7).map(|touch| touch.position),
            Some((12.0, 34.0))
        );

        input.end_frame();
        input.touch_move(7, 56.0, 78.0);

        assert!(input.touch_pressed(7));
        assert!(!input.touch_just_pressed(7));
        assert_eq!(
            input.touch(7).map(|touch| touch.position),
            Some((56.0, 78.0))
        );

        input.touch_end(7);

        assert!(!input.touch_pressed(7));
        assert!(input.touch_just_released(7));
    }
}
