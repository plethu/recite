//! Bound the changed region at word boundaries without altering source text.
use crate::design::tokens as t;
use freya::prelude::*;
use recite_writer_session::comparison::changed_range as range;
pub(super) fn render(text: &str, other: &str, code: bool) -> Element {
    let changed = range(text, other);
    paragraph()
        .width(Size::fill())
        .font_size(if code {
            t::code_size()
        } else {
            t::prose_size()
        })
        .font_family(if code { "monospace" } else { "serif" })
        .span(Span::new(text[..changed.start].to_owned()))
        .span(
            Span::new(text[changed.clone()].to_owned())
                .font_weight(FontWeight::BOLD)
                .text_decoration(TextDecoration::Underline),
        )
        .span(Span::new(text[changed.end..].to_owned()))
        .into_element()
}
