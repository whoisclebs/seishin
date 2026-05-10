use std::collections::HashSet;

use crate::KeyCode;

#[derive(Debug, Default, Clone)]
pub struct InputState {
    pressed_keys: HashSet<KeyCode>,
    just_pressed_keys: HashSet<KeyCode>,
    just_released_keys: HashSet<KeyCode>,
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
    }

    pub fn is_key_down(&self, key: KeyCode) -> bool {
        self.pressed(key)
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
}
