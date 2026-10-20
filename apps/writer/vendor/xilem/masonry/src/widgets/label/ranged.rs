// Copyright 2026 the Recite contributors
// SPDX-License-Identifier: Apache-2.0

use super::*;

impl Label {
    /// Sets byte-range styles, applied after this label's global styles.
    ///
    /// # Panics
    ///
    /// Panics if a range is outside the text, splits a UTF-8 character, or uses
    /// a nonzero brush index. Text replacement clears these styles.
    pub fn with_ranged_styles(mut self, styles: Vec<(Range<usize>, StyleProperty)>) -> Self {
        self.replace_ranged_styles(styles);
        self
    }

    fn replace_ranged_styles(&mut self, styles: Vec<(Range<usize>, StyleProperty)>) {
        for (range, property) in &styles {
            assert!(
                self.text.get(range.clone()).is_some(),
                "invalid UTF-8 style range"
            );
            assert!(
                !matches!(
                    property,
                    StyleProperty::Brush(BrushIndex(1..))
                        | StyleProperty::UnderlineBrush(Some(BrushIndex(1..)))
                        | StyleProperty::StrikethroughBrush(Some(BrushIndex(1..)))
                ),
                "a Label only supports brush index zero"
            );
        }
        self.ranged_styles = styles;
    }

    /// Replaces the active label's range styles, invalidating cached text layout.
    ///
    /// The same range and brush requirements as [`Self::with_ranged_styles`] apply.
    pub fn set_ranged_styles(
        this: &mut WidgetMut<'_, Self>,
        styles: Vec<(Range<usize>, StyleProperty)>,
    ) {
        this.widget.replace_ranged_styles(styles);
        this.widget.clear_cache();
        this.ctx.request_layout();
    }
}
