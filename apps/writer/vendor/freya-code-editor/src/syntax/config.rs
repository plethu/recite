//! Upstream capture-to-theme mapping and language query configuration.
use crate::{editor_theme::EditorSyntaxTheme, languages::EditorLanguage};
use freya_core::prelude::Color;
use tree_sitter::{Language, Query};

fn capture_color(name: &str, theme: &EditorSyntaxTheme) -> Color {
    match name {
        "attribute" => theme.attribute,
        "boolean" => theme.boolean,
        "comment" | "comment.documentation" => theme.comment,
        "constant" | "constant.builtin" => theme.constant,
        "constructor" => theme.constructor,
        "escape" => theme.escape,
        "function" | "function.builtin" => theme.function,
        "function.macro" => theme.function_macro,
        "function.method" => theme.function_method,
        "keyword" => theme.keyword,
        "label" => theme.label,
        "module" => theme.module,
        "number" => theme.number,
        "operator" => theme.operator,
        "property" => theme.property,
        "punctuation" => theme.punctuation,
        "punctuation.bracket" => theme.punctuation_bracket,
        "punctuation.delimiter" => theme.punctuation_delimiter,
        "punctuation.special" => theme.punctuation_special,
        "string" => theme.string,
        "string.escape" => theme.string_escape,
        "string.special" | "string.special.key" | "string.special.symbol" => theme.string_special,
        "tag" => theme.tag,
        "text.literal" => theme.text_literal,
        "text.reference" => theme.text_reference,
        "text.title" => theme.text_title,
        "text.uri" => theme.text_uri,
        "text.emphasis" | "text.strong" => theme.text_emphasis,
        "type" | "type.builtin" => theme.type_,
        "variable" => theme.variable,
        "variable.builtin" => theme.variable_builtin,
        "variable.parameter" => theme.variable_parameter,
        _ => theme.text,
    }
}

/// Tries exact match, then strips trailing dot-segments for hierarchical fallback.
fn resolve_capture_color(name: &str, theme: &EditorSyntaxTheme) -> Color {
    let color = capture_color(name, theme);
    if color != theme.text {
        return color;
    }
    let mut candidate = name;
    while let Some(pos) = candidate.rfind('.') {
        candidate = &candidate[..pos];
        let c = capture_color(candidate, theme);
        if c != theme.text {
            return c;
        }
    }
    theme.text
}

pub(super) struct LangConfig {
    pub(super) language: Language,
    pub(super) query: Query,
    pub(super) capture_colors: Vec<Color>,
}

impl EditorLanguage {
    pub(super) fn lang_config(&self, theme: &EditorSyntaxTheme) -> Option<LangConfig> {
        let language = self.language.clone();
        let query = Query::new(&language, self.highlights_query.as_ref()).ok()?;
        let capture_colors: Vec<Color> = query
            .capture_names()
            .iter()
            .map(|name| resolve_capture_color(name, theme))
            .collect();

        Some(LangConfig {
            language,
            query,
            capture_colors,
        })
    }
}
