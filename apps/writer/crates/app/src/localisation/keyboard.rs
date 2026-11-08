//! A field save names its exact catalogue entry rather than borrowing source-save policy.
use super::*;
use std::fmt;

#[derive(Clone, PartialEq, Eq)]
pub struct TranslationTarget(pub EntryKey);
impl fmt::Debug for TranslationTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TranslationTarget").finish_non_exhaustive()
    }
}
impl Writer {
    pub fn save_focused_translation(&mut self, target: &TranslationTarget) {
        self.save_translation_key(&target.0);
    }
}

#[cfg(test)]
mod tests;
