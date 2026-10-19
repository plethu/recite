use std::borrow::Cow;

use tree_sitter::Language;

/// A language definition used for syntax highlighting.
///
/// Bring your own tree-sitter grammar and its highlights query, so the editor
/// can highlight any language without the crate depending on specific grammars.
///
/// ```no_run
/// # use freya_code_editor::prelude::EditorLanguage;
/// let language = EditorLanguage::new(
///     tree_sitter_rust::LANGUAGE,
///     tree_sitter_rust::HIGHLIGHTS_QUERY,
/// );
/// ```
#[derive(Clone)]
pub struct EditorLanguage {
    pub language: Language,
    pub(crate) line_isolated: bool,
    pub highlights_query: Cow<'static, str>,
}

impl EditorLanguage {
    /// Opt into independently parsing clean physical lines after a clean full parse.
    /// The grammar and query must have identical captures when composed by line;
    /// multiline tokens and ancestor-dependent captures do not satisfy this contract.
    pub fn with_line_isolated_highlighting(mut self) -> Self {
        self.line_isolated = true;
        self
    }

    pub fn new(
        language: impl Into<Language>,
        highlights_query: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            language: language.into(),
            line_isolated: false,
            highlights_query: highlights_query.into(),
        }
    }
}
