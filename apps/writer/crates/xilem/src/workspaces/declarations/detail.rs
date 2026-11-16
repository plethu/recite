//! The chosen declaration: its name and kind, then a condition's or effect's parameters as a
//! definition list, or a speaker's or registry's prose, wrapped and scrolling when it is long.
use super::*;
use crate::presentation::pane;

/// The chosen declaration, or a prompt to choose one.
/// `height` is the column's: the prose area takes what the name and kind leave of it.
pub(super) fn detail(writer: &Writer, width: f64, height: f64) -> impl WidgetView<Writer> + use<> {
    let panel = &writer.workspaces.declarations;
    let scale = writer.ui_scale();
    let chosen = panel.selected.and_then(|index| panel.rows.get(index));
    flex_col((
        chosen.is_none().then(|| {
            print::meta(scale, "Choose a declaration to see its fields.")
                .color(writer.palette.muted)
        }),
        chosen.map(|chosen| {
            flex_col((
                label(scale, chosen.name.clone()).typeset(Type::Section, scale),
                print::meta(scale, chosen.kind.clone()).color(writer.palette.muted),
                match &chosen.detail {
                    Detail::Parameters(list) => Some(parameters(writer, list, width)),
                    Detail::Prose(_) => None,
                },
                prose(writer, &chosen.detail, width, height),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(space::SIBLINGS.px())
        }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
}

/// A condition's or effect's parameters as a definition list: each name in Plex Mono ink, its
/// type in muted Sans beside it, one row each, read name then type.
fn parameters(
    writer: &Writer,
    list: &[(String, String)],
    width: f64,
) -> impl WidgetView<Writer> + use<> {
    let scale = writer.ui_scale();
    let palette = &writer.palette;
    let term = (size::TERM_COLUMN * f64::from(scale)).min(width / 2.);
    let height = size::ROW * f64::from(scale);
    flex_col((
        print::caps(scale, "Parameters").color(palette.muted),
        list.is_empty()
            .then(|| print::meta(scale, "None").color(palette.muted)),
        flex_col(
            list.iter()
                .map(|(name, kind)| {
                    flex_row((
                        one_line(name.clone(), Type::Body, scale, palette.ink)
                            .families(Type::Id.families())
                            .width(term.px()),
                        one_line(kind.clone(), Type::Body, scale, palette.muted).flex(1.),
                    ))
                    .cross_axis_alignment(CrossAxisAlignment::Center)
                    .dims(
                        Dimensions::AUTO
                            .with_width(width.px())
                            .with_height(height.px()),
                    )
                })
                .collect::<Vec<_>>(),
        )
        .gap(0.px()),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::TEXT.px())
}

/// A speaker's display name, a registry's values or a pointer to the source: wrapped in a
/// scrolling detail area that takes the column's remaining height. A single unbroken token cannot
/// wrap, so it sits on one line ending in an ellipsis, its full text still its accessible name.
fn prose(
    writer: &Writer,
    detail: &Detail,
    width: f64,
    height: f64,
) -> (
    Option<impl WidgetView<Writer> + use<>>,
    Option<impl WidgetView<Writer> + use<>>,
) {
    let scale = writer.ui_scale();
    let Detail::Prose(text) = detail else {
        return (None, None);
    };
    let unbroken = !text.is_empty() && !text.contains(char::is_whitespace);
    // Below the name in Section and the kind in Meta, each a sibling gap apart.
    let heading = (Type::Section.metrics().1 + Type::Meta.metrics().1) * scale;
    let area = height - f64::from(heading) - 2. * space::SIBLINGS;
    (
        (!unbroken).then(|| {
            pane("Declaration detail", paragraph(scale, text.clone(), width))
                .constrain_horizontal(true)
                .dims((
                    width.px(),
                    area.max(f64::from(Type::Body.metrics().1 * scale)).px(),
                ))
        }),
        unbroken.then(|| {
            one_line(text.clone(), Type::Body, scale, writer.palette.ink).width(width.px())
        }),
    )
}
