use std::ops::Range;

use crate::{
    SourceSpan,
    source_location::{point_one, position_for_byte_offset},
    toml_spans::TomlSpanIndex,
};

/// Return a value-oriented span for a TOML parser error.
pub(super) fn error_span(file: &str, source: &str, range: Option<Range<usize>>) -> SourceSpan {
    let Some(range) = range else {
        return SourceSpan::point(file, point_one());
    };
    SourceSpan::new(
        file,
        position_for_byte_offset(source, range.start),
        Some(position_for_byte_offset(
            source,
            range.end.max(range.start + 1),
        )),
    )
}

/// Resolve a key or value through the immutable TOML CST index.
pub(super) fn key_span(
    file: &str,
    source: &str,
    spans: &TomlSpanIndex,
    table_path: &[String],
    key: &str,
    value: bool,
) -> SourceSpan {
    let mut path = table_path.to_vec();
    path.push(key.to_owned());
    error_span(
        file,
        source,
        if value {
            spans.value_range(&path)
        } else {
            spans.key_range(&path)
        },
    )
}

pub(super) fn table_span(
    file: &str,
    source: &str,
    spans: &TomlSpanIndex,
    table_path: &[String],
) -> SourceSpan {
    error_span(file, source, spans.key_range(table_path))
}

pub(super) fn document_span(file: &str) -> SourceSpan {
    SourceSpan::point(file, point_one())
}

pub(super) fn range_span(file: &str, source: &str, range: Option<Range<usize>>) -> SourceSpan {
    range.map_or_else(
        || document_span(file),
        |range| {
            SourceSpan::new(
                file,
                position_for_byte_offset(source, range.start),
                Some(position_for_byte_offset(source, range.end)),
            )
        },
    )
}
