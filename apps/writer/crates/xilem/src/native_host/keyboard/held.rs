//! A consumed press owns repeats and release even after its UI context retires.
use super::{ElementState, keyboard};

#[derive(Default)]
pub(super) struct HeldKeys(Vec<keyboard::PhysicalKey>);
impl HeldKeys {
    pub fn press(&mut self, key: keyboard::PhysicalKey) {
        if !self.0.contains(&key) {
            self.0.push(key);
        }
    }
    pub fn consume(&mut self, key: keyboard::PhysicalKey, state: ElementState) -> bool {
        let Some(index) = self.0.iter().position(|held| *held == key) else {
            return false;
        };
        if state == ElementState::Released {
            self.0.remove(index);
        }
        true
    }
}

#[cfg(test)]
mod tests;
