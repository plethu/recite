use std::ops::Range;

use toml_edit::Document;

use crate::{
    SourceSpan,
    source_location::{point_one, position_for_byte_offset},
    toml_spans::TomlSpanIndex,
};

#[must_use]
pub fn project_scene_key_span(
    file: &str,
    source: &str,
    scene_index: usize,
    key: &str,
) -> SourceSpan {
    let spans = Document::parse(source.to_owned())
        .ok()
        .map(|document| TomlSpanIndex::from_document(&document));
    spans.as_ref().map_or_else(
        || manifest_span(file),
        |spans| scene_key_span_with_index(file, source, spans, scene_index, key),
    )
}

pub(super) fn scene_key_span(
    file: &str,
    source: &str,
    spans: Option<&TomlSpanIndex>,
    scene_index: usize,
    key: &str,
) -> SourceSpan {
    spans.map_or_else(
        || manifest_span(file),
        |spans| scene_key_span_with_index(file, source, spans, scene_index, key),
    )
}

pub(super) fn toml_error_span(file: &str, source: &str, range: Option<Range<usize>>) -> SourceSpan {
    let Some(range) = range else {
        return manifest_span(file);
    };
    SourceSpan::point(
        file.to_owned(),
        position_for_byte_offset(source, range.start),
    )
}

pub(super) fn scene_key_span_with_index(
    file: &str,
    source: &str,
    spans: &TomlSpanIndex,
    scene_index: usize,
    key: &str,
) -> SourceSpan {
    let scene_path = ["scenes".to_owned(), format!("[{scene_index}]")];
    let mut key_path = scene_path.to_vec();
    key_path.push(key.to_owned());
    spans.key_range(&key_path).map_or_else(
        || {
            spans.table_range(&scene_path).map_or_else(
                || manifest_span(file),
                |range| source_span(file, source, range),
            )
        },
        |range| source_span(file, source, range),
    )
}

fn manifest_span(file: &str) -> SourceSpan {
    SourceSpan::point(file.to_owned(), point_one())
}

fn source_span(file: &str, source: &str, range: Range<usize>) -> SourceSpan {
    SourceSpan::new(
        file,
        position_for_byte_offset(source, range.start),
        Some(position_for_byte_offset(source, range.end)),
    )
}
