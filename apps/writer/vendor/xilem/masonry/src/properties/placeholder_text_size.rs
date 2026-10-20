// SPDX-License-Identifier: Apache-2.0

use crate::core::Property;

/// Font size for a text input's placeholder label, independently of its colour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlaceholderTextSize {
    /// Size in logical pixels.
    pub size: f32,
}
static DEFAULT: PlaceholderTextSize = PlaceholderTextSize { size: 14.0 };
impl Default for PlaceholderTextSize {
    fn default() -> Self {
        DEFAULT
    }
}
impl Property for PlaceholderTextSize {
    fn static_default() -> &'static Self {
        &DEFAULT
    }
}
