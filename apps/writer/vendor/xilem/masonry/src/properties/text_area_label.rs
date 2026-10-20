// SPDX-License-Identifier: Apache-2.0

use std::sync::LazyLock;

use crate::core::{ArcStr, Property};

/// An explicit accessible name for a text area, independent of its placeholder.
///
/// `TextInput` forwards this property to its inner text area.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextAreaLabel {
    /// The name announced by assistive technology.
    pub label: ArcStr,
}

impl TextAreaLabel {
    /// Creates an accessible name.
    pub fn new(label: impl Into<ArcStr>) -> Self {
        Self { label: label.into() }
    }
}

impl Property for TextAreaLabel {
    fn static_default() -> &'static Self {
        static DEFAULT: LazyLock<TextAreaLabel> = LazyLock::new(TextAreaLabel::default);
        &DEFAULT
    }
}
