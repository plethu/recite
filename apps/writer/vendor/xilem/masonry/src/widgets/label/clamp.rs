//! Optional shaped abbreviations leave the original text/layout authoritative for accessibility.
use super::*;

pub(super) struct Abbreviation {
    text: String,
    pub(super) layout: Layout<BrushIndex>,
    #[cfg(feature = "testing")]
    shapes: usize,
}
impl TextLayout {
    pub(super) fn display(&self) -> &Layout<BrushIndex> {
        self.clamped
            .as_ref()
            .map_or(&self.layout, |clamped| &clamped.layout)
    }
}
impl Label {
    /// Additional layouts shaped for abbreviations retained in the current cache.
    #[cfg(feature = "testing")]
    pub fn debug_abbreviation_shapes(&self) -> usize {
        self.layouts
            .iter()
            .filter_map(|layout| layout.clamped.as_ref())
            .map(|clamped| clamped.shapes)
            .sum()
    }
    pub(super) fn displayed_text(&self) -> &str {
        self.layouts
            .get(self.active_layout)
            .and_then(|layout| layout.clamped.as_ref())
            .map_or(self.text.as_ref(), |clamped| &clamped.text)
    }
    /// Cap visual text at a positive line count, with a shaped ellipsis on overflow.
    /// Accessibility retains the complete original text. If even the ellipsis
    /// cannot fit the available width, the visual abbreviation is empty.
    pub fn with_max_lines(mut self, limit: Option<NonZeroUsize>) -> Self {
        self.max_lines = limit;
        self
    }
    /// Update the visual line cap without replacing the native label.
    pub fn set_max_lines(this: &mut WidgetMut<'_, Self>, limit: Option<NonZeroUsize>) {
        if this.widget.max_lines != limit {
            this.widget.max_lines = limit;
            this.widget.clear_cache();
            this.ctx.request_layout();
        }
    }
}
pub(super) struct Shaper<'a> {
    pub(super) text: &'a str,
    pub(super) styles: &'a StyleSet,
    pub(super) ranged: &'a [(Range<usize>, StyleProperty)],
    pub(super) max_advance: Option<f32>,
    pub(super) fonts: &'a mut FontContext,
    pub(super) layouts: &'a mut LayoutContext<BrushIndex>,
    #[cfg(feature = "testing")]
    pub(super) shapes: usize,
}
impl Shaper<'_> {
    fn candidate(&mut self, end: usize, ellipsis: bool) -> Abbreviation {
        #[cfg(feature = "testing")]
        {
            self.shapes += 1;
        }
        let prefix = self.text[..end].trim_end();
        let mut text = prefix.to_owned();
        if ellipsis {
            text.push('…');
        }
        let mut layout = Layout::new();
        let mut builder = self.layouts.ranged_builder(self.fonts, &text, 1., true);
        for property in self.styles.inner().values() {
            builder.push_default(property.to_owned());
        }
        for (range, property) in self.ranged {
            let end = range.end.min(prefix.len());
            if range.start < end {
                builder.push(property.clone(), range.start..end);
            }
            if ellipsis && range.contains(&prefix.len().saturating_sub(1)) {
                builder.push(property.clone(), prefix.len()..text.len());
            }
        }
        builder.build_into(&mut layout, &text);
        layout.break_all_lines(self.max_advance);
        Abbreviation {
            text,
            layout,
            #[cfg(feature = "testing")]
            shapes: 0,
        }
    }
    fn fits(&self, layout: &Layout<BrushIndex>, lines: NonZeroUsize) -> bool {
        layout.len() <= lines.get()
            && self
                .max_advance
                .is_none_or(|width| layout.width() <= width + TextLayout::EPSILON)
    }
}

impl Shaper<'_> {
    fn finish(self, abbreviation: Abbreviation) -> Abbreviation {
        #[cfg(feature = "testing")]
        let abbreviation = Abbreviation {
            shapes: self.shapes,
            ..abbreviation
        };
        abbreviation
    }
    pub(super) fn abbreviate(
        mut self,
        original: &Layout<BrushIndex>,
        lines: NonZeroUsize,
    ) -> Option<Abbreviation> {
        let shaper = &mut self;
        if shaper.fits(original, lines) {
            return None;
        }
        let mut best = shaper.candidate(0, true);
        if !shaper.fits(&best.layout, lines) {
            let empty = shaper.candidate(0, false);
            return Some(self.finish(empty));
        }
        let Some(line) = original.get(lines.get().min(original.len()).saturating_sub(1)) else {
            return Some(self.finish(best));
        };
        // Collect logical shaped-cluster boundaries once. Visual run order is not
        // logical truncation order for mixed-direction text.
        let mut boundaries = vec![line.text_range().start];
        for run in line.runs() {
            for cluster in run.clusters() {
                boundaries.push(cluster.text_range().end);
            }
        }
        boundaries.sort_unstable();
        boundaries.dedup();
        // Shaping is not formally monotone. This is a bounded candidate search, and
        // only a verified fitting layout is retained; it may conservatively omit more.
        let (mut low, mut high) = (0, boundaries.len());
        while low < high {
            let middle = low + (high - low) / 2;
            let candidate = shaper.candidate(boundaries[middle], true);
            if shaper.fits(&candidate.layout, lines) {
                best = candidate;
                low = middle + 1;
            } else {
                high = middle;
            }
        }
        Some(self.finish(best))
    }
}
