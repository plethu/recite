//! The chosen declaration: its name and kind, then a condition's or effect's parameters as a
//! definition list, or a speaker's or registry's prose.
use super::*;

/// The chosen declaration, or a prompt to choose one.
pub(super) fn detail(writer: &Writer, width: f64) -> impl WidgetView<Writer> + use<> {
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
                    Detail::Parameters(list) => Either::A(parameters(writer, list, width)),
                    Detail::Pages(pages) => Either::B(prose(writer, pages, width)),
                },
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

/// A speaker's display name, a registry's values or a pointer to the source, a page at a time
/// when it is enormous.
fn prose(writer: &Writer, pages: &[String], width: f64) -> impl WidgetView<Writer> + use<> {
    let panel = &writer.workspaces.declarations;
    let scale = writer.ui_scale();
    flex_col((
        paragraph(
            scale,
            pages.get(panel.detail_page).cloned().unwrap_or_default(),
            width,
        ),
        (pages.len() > 1).then(|| {
            flex_row((
                text_button(
                    scale,
                    "Previous declaration detail",
                    |writer: &mut Writer| {
                        let panel = &mut writer.workspaces.declarations;
                        panel.detail_page = panel.detail_page.saturating_sub(1);
                    },
                )
                .disabled(panel.detail_page == 0),
                text_button(scale, "More declaration detail", |writer: &mut Writer| {
                    writer.workspaces.declarations.detail_page += 1
                })
                .disabled(pages.len() <= panel.detail_page + 1),
            ))
            .gap(space::SIBLINGS.px())
        }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::SIBLINGS.px())
}
