use std::ops::Range;

use crate::{SourceSpan, source_location::position_for_byte_offset};

use super::super::lower::ManifestSourceFormat;
use super::{
    ManifestSpans, StringRole, TomlSpanIndex, document_start_span, source_section_range,
    top_level_key_span, top_level_object_value_span,
};

pub(crate) fn top_level_toml_number_token<'a>(
    source: &'a str,
    key: &str,
    spans: Option<&TomlSpanIndex>,
) -> Option<&'a str> {
    let range = spans?.value_range(&[key.to_owned()])?;
    source.get(range)
}

impl ManifestSpans {
    pub(crate) fn new_with_format(
        format: ManifestSourceFormat,
        toml_spans: Option<&TomlSpanIndex>,
    ) -> Self {
        Self {
            next_offsets: std::collections::BTreeMap::new(),
            active_range: None,
            active_section: None,
            format,
            toml_spans: toml_spans.cloned(),
        }
    }

    pub(crate) fn enter_section(&mut self, source: &str, section: &str) {
        self.active_section = Some(section.to_owned());
        self.active_range = match self.format {
            ManifestSourceFormat::Json => source_section_range(source, section),
            ManifestSourceFormat::Toml => None,
        };
        self.next_offsets.clear();
    }

    pub(crate) fn root_key_span(&self, file: &str, source: &str, key: &str) -> SourceSpan {
        if self.format == ManifestSourceFormat::Toml {
            return self
                .toml_spans
                .as_ref()
                .and_then(|spans| spans.key_range(&[key.to_owned()]))
                .map_or_else(
                    || document_start_span(file),
                    |range| source_span(file, source, range),
                );
        }
        top_level_key_span(file, source, key)
    }

    pub(crate) fn root_object_value_span(
        &self,
        file: &str,
        source: &str,
        object_key: &str,
        value_key: &str,
    ) -> SourceSpan {
        if self.format == ManifestSourceFormat::Toml {
            let path = [object_key.to_owned(), value_key.to_owned()];
            return self
                .toml_spans
                .as_ref()
                .and_then(|spans| spans.value_range(&path))
                .map_or_else(
                    || document_start_span(file),
                    |range| source_span(file, source, range),
                );
        }
        top_level_object_value_span(file, source, object_key, value_key)
    }

    /// Resolve a semantic field through the CST path supplied by the raw
    /// manifest walk. TOML must never fall back to matching text: the same
    /// string may occur in a comment, another declaration, or a sibling
    /// field. JSON retains its established occurrence-based spans.
    pub(crate) fn value_span_at(
        &mut self,
        file: &str,
        source: &str,
        path: &[String],
        fallback: &str,
    ) -> SourceSpan {
        self.path_span(file, source, path, fallback, StringRole::Value)
    }

    pub(crate) fn key_span_at(
        &mut self,
        file: &str,
        source: &str,
        path: &[String],
        fallback: &str,
    ) -> SourceSpan {
        self.path_span(file, source, path, fallback, StringRole::Key)
    }

    pub(crate) fn nested_key_span_at(
        &mut self,
        file: &str,
        source: &str,
        path: &[String],
        fallback: &str,
    ) -> SourceSpan {
        self.path_span(file, source, path, fallback, StringRole::AnyKey)
    }

    fn path_span(
        &mut self,
        file: &str,
        source: &str,
        path: &[String],
        fallback: &str,
        role: StringRole,
    ) -> SourceSpan {
        if path.is_empty() {
            return self.next_string_span(file, source, fallback, role);
        }
        if self.format == ManifestSourceFormat::Toml {
            return self
                .toml_spans
                .as_ref()
                .and_then(|index| match role {
                    StringRole::Key | StringRole::AnyKey => index.key_range(path),
                    StringRole::Value => index.value_range(path),
                })
                .map_or_else(
                    || document_start_span(file),
                    |range| source_span(file, source, range),
                );
        }
        self.next_string_span(file, source, fallback, role)
    }

    pub(super) fn toml_next_string_span(
        &mut self,
        file: &str,
        source: &str,
        needle: &str,
        role: StringRole,
    ) -> SourceSpan {
        let Some(index) = &self.toml_spans else {
            return document_start_span(file);
        };
        let search_key = format!("toml:{role:?}:{needle}");
        let search_start = self.next_offsets.get(&search_key).copied().unwrap_or(0);
        let Some(span_range) = index.find(
            self.active_section.as_deref(),
            needle,
            role == StringRole::Value,
            search_start,
        ) else {
            return document_start_span(file);
        };
        self.next_offsets.insert(search_key, span_range.end);
        source_span(file, source, span_range)
    }
}

fn source_span(file: &str, source: &str, range: Range<usize>) -> SourceSpan {
    SourceSpan::new(
        file,
        position_for_byte_offset(source, range.start),
        Some(position_for_byte_offset(source, range.end)),
    )
}
