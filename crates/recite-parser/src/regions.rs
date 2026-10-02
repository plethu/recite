//! Grammar-owned restart boundaries for incremental authoring analysis.
use std::ops::Range;

use crate::{Parse, markers::StatementMarker, parser::parse_region, source::LogicalLines};

/// A source slice that can be parsed independently with original file positions.
///
/// Regions begin at unindented block markers (or the start of the file).
/// Indented markers remain inside their surrounding region, including during
/// error recovery. Construct regions with [`source_regions`].
#[derive(Clone, Debug)]
pub struct SourceRegion<'a> {
    text: &'a str,
    bytes: Range<usize>,
    first_line: u32,
}

impl SourceRegion<'_> {
    #[must_use]
    pub fn text(&self) -> &str {
        self.text
    }

    #[must_use]
    pub fn byte_range(&self) -> Range<usize> {
        self.bytes.clone()
    }

    #[must_use]
    pub fn first_line(&self) -> u32 {
        self.first_line
    }

    /// Parses this region, keeping diagnostics and lowered spans file-relative.
    /// The lossless syntax tree contains only this region's text.
    #[must_use]
    pub fn parse(&self, path: impl Into<String>) -> Parse {
        parse_region(path.into(), self.text.into(), self.first_line)
    }
}

/// Partitions source at grammar-safe top-level block boundaries.
///
/// Concatenating the lowered blocks preserves full-file lowering. Recovery
/// completeness must still be combined across all regions before validation.
#[must_use]
pub fn source_regions(source: &str) -> Vec<SourceRegion<'_>> {
    let mut regions = Vec::new();
    let mut start = 0;
    let mut first_line = 1;
    let mut offset = 0;
    for line in LogicalLines::new(source) {
        if offset != 0 && StatementMarker::parse(line.text) == Some(StatementMarker::Block) {
            regions.push(SourceRegion {
                text: &source[start..offset],
                bytes: start..offset,
                first_line,
            });
            start = offset;
            first_line = line.number;
        }
        offset += line.text.len() + line.newline.len();
    }
    regions.push(SourceRegion {
        text: &source[start..],
        bytes: start..source.len(),
        first_line,
    });
    regions
}
