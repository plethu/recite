//! A change drawn side by side: the version before and the version after on inset paper, each
//! captioned above its text, the changed span emphasised in both. A host too narrow for two
//! columns stacks them. Translation updates, rename references and disk changes all draw their
//! differences this way.
use super::emphasized_text::{manuscript_comparison, source_comparison};
use super::{label, space};
use recite_writer_session::palette::Palette;
use xilem::{
    WidgetView,
    core::one_of::Either,
    masonry::{core::ArcStr, kurbo::Axis, layout::AsUnit},
    peniko::Color,
    style::Style,
    view::{CrossAxisAlignment, flex_col, flex_row, sized_box},
};

/// How a diff sets its text: manuscript prose or source, at a text size.
#[derive(Clone, Copy)]
pub(crate) enum Text {
    Manuscript(f32),
    Source(f32),
}

/// One side of a diff: its caption and its text, if that side has any.
pub(crate) struct Side<'a> {
    pub caption: &'static str,
    pub text: Option<&'a str>,
}

/// The two sides of a change `width` wide, side by side from `two_columns` wide.
pub(crate) fn side_by_side<S: 'static>(
    palette: &Palette<Color>,
    scale: f32,
    (width, two_columns): (f64, f64),
    (before, after): (Side<'_>, Side<'_>),
    text: Text,
) -> impl WidgetView<S> + use<S> {
    let wide = width >= two_columns;
    let column = if wide {
        (width - space::TEXT) / 2.
    } else {
        width
    };
    let first = side(palette, scale, column, &before, after.text, text);
    let second = side(palette, scale, column, &after, before.text, text);
    flex_row((first, second))
        .direction(if wide {
            Axis::Horizontal
        } else {
            Axis::Vertical
        })
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(space::TEXT.px())
}

fn side<S: 'static>(
    palette: &Palette<Color>,
    scale: f32,
    width: f64,
    this: &Side<'_>,
    other: Option<&str>,
    text: Text,
) -> impl WidgetView<S> + use<S> {
    let value = this.text.unwrap_or("(No text)").to_owned();
    let other = other.unwrap_or_default().to_owned();
    let inner = (width - 2. * space::KINDS).max(0.);
    let body = match text {
        Text::Manuscript(size) => Either::A(manuscript_comparison(size, value, other)),
        Text::Source(size) => Either::B(source_comparison(size, value, other)),
    };
    sized_box(
        flex_col((
            label(scale, ArcStr::from(this.caption)),
            flex_row((sized_box(body).width(inner.px()),)),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(space::SIBLINGS.px()),
    )
    .width(width.px())
    .padding(space::KINDS.px())
    .background_color(palette.inset)
}
