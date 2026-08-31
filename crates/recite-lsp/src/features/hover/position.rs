use lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Range};

pub(super) fn hover_response(value: &str, range: Range) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::PlainText,
            value: value.to_owned(),
        }),
        range: Some(range),
    }
}
