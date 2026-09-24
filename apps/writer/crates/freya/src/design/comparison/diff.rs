//! Bound the changed region at word boundaries without altering source text.
use crate::design::tokens as t;
use freya::prelude::*;
fn range(text: &str, other: &str) -> std::ops::Range<usize> {
    let words: Vec<_> = text.split_inclusive(char::is_whitespace).collect();
    let other: Vec<_> = other.split_inclusive(char::is_whitespace).collect();
    let prefix = words.iter().zip(&other).take_while(|(a, b)| a == b).count();
    let suffix = words[prefix..]
        .iter()
        .rev()
        .zip(other[prefix..].iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    words[..prefix].iter().map(|s| s.len()).sum()
        ..words[..words.len() - suffix].iter().map(|s| s.len()).sum()
}
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
#[cfg(test)]
mod tests;
