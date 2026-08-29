use super::super::spans::ManifestSpans;
use crate::{Diagnostic, SourceSpan};

/// Shared source and reporting state for one manifest lowering pass.
///
/// Lowering helpers own their semantic inputs (schema definitions, pending
/// references, or provenance mappings), while this context owns the source
/// location machinery and diagnostic sink that all of them share.
pub(super) struct LoweringContext<'a> {
    pub(super) file: &'a str,
    pub(super) source: &'a str,
    pub(super) spans: &'a mut ManifestSpans,
    pub(super) diagnostics: &'a mut Vec<Diagnostic>,
}

impl<'a> LoweringContext<'a> {
    pub(super) fn new(
        file: &'a str,
        source: &'a str,
        spans: &'a mut ManifestSpans,
        diagnostics: &'a mut Vec<Diagnostic>,
    ) -> Self {
        Self {
            file,
            source,
            spans,
            diagnostics,
        }
    }

    pub(super) fn value_span_at(&mut self, path: &[String], fallback: &str) -> SourceSpan {
        self.spans
            .value_span_at(self.file, self.source, path, fallback)
    }

    pub(super) fn key_span_at(&mut self, path: &[String], fallback: &str) -> SourceSpan {
        self.spans
            .key_span_at(self.file, self.source, path, fallback)
    }

    pub(super) fn nested_key_span_at(&mut self, path: &[String], fallback: &str) -> SourceSpan {
        self.spans
            .nested_key_span_at(self.file, self.source, path, fallback)
    }

    pub(super) fn root_key_span(&self, key: &str) -> SourceSpan {
        self.spans.root_key_span(self.file, self.source, key)
    }

    pub(super) fn root_object_value_span(&self, object_key: &str, value_key: &str) -> SourceSpan {
        self.spans
            .root_object_value_span(self.file, self.source, object_key, value_key)
    }
}
