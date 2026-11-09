//! Shaped abbreviations end at a logical cluster boundary with an ellipsis.
use super::*;

pub(super) struct Abbreviation {
    pub(super) text: String,
    pub(super) layout: Layout<BrushIndex>,
}

pub(super) struct Abbreviator<'a> {
    pub(super) text: &'a str,
    pub(super) styles: &'a StyleSet,
    pub(super) ranged: &'a [(Range<usize>, StyleProperty)],
    pub(super) max_advance: Option<f32>,
    pub(super) fonts: &'a mut FontContext,
    pub(super) layouts: &'a mut LayoutContext<BrushIndex>,
    /// Candidate layouts shaped by this search.
    pub(super) shapes: usize,
}

/// Width tolerance for floating-point line breaking.
const EPSILON: f32 = 0.001;

impl Abbreviator<'_> {
    /// The text up to `end`, trimmed, optionally followed by an ellipsis that keeps the style of
    /// the character it follows.
    fn candidate(&mut self, end: usize, ellipsis: bool) -> Abbreviation {
        self.shapes += 1;
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
            let styled_end = range.end.min(prefix.len());
            if range.start < styled_end {
                builder.push(property.clone(), range.start..styled_end);
            }
            if ellipsis && range.contains(&prefix.len().saturating_sub(1)) {
                builder.push(property.clone(), prefix.len()..text.len());
            }
        }
        builder.build_into(&mut layout, &text);
        layout.break_all_lines(self.max_advance);
        Abbreviation { text, layout }
    }
    fn fits(&self, layout: &Layout<BrushIndex>, lines: NonZeroUsize) -> bool {
        layout.len() <= lines.get()
            && self
                .max_advance
                .is_none_or(|width| layout.width() <= width + EPSILON)
    }
    /// An abbreviation of `original` within `lines`, or `None` when it already fits. If even the
    /// ellipsis cannot fit, the abbreviation is empty.
    pub(super) fn abbreviate(
        &mut self,
        original: &Layout<BrushIndex>,
        lines: NonZeroUsize,
    ) -> Option<Abbreviation> {
        if self.fits(original, lines) {
            return None;
        }
        let mut best = self.candidate(0, true);
        if !self.fits(&best.layout, lines) {
            return Some(self.candidate(0, false));
        }
        let Some(line) = original.get(lines.get().min(original.len()).saturating_sub(1)) else {
            return Some(best);
        };
        // Logical cluster ends on the last kept line; visual run order is not logical truncation
        // order in mixed-direction text.
        let mut boundaries = vec![line.text_range().start];
        for run in line.runs() {
            for cluster in run.clusters() {
                boundaries.push(cluster.text_range().end);
            }
        }
        boundaries.sort_unstable();
        boundaries.dedup();
        // Shaping is not strictly monotone, so this bounded search keeps only a verified fit and
        // may conservatively omit more text.
        let (mut low, mut high) = (0, boundaries.len());
        while low < high {
            let middle = low + (high - low) / 2;
            let candidate = self.candidate(boundaries[middle], true);
            if self.fits(&candidate.layout, lines) {
                best = candidate;
                low = middle + 1;
            } else {
                high = middle;
            }
        }
        Some(best)
    }
}
