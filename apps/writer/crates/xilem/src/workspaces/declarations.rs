//! Frontend views and native glue over `recite_writer_app::workspaces::declarations`.
use super::*;
use crate::presentation::row::{listed, row};
use crate::presentation::segments::{segment, segments};
use crate::presentation::sheet_text::one_line;
use crate::presentation::space;
use crate::presentation::tokens::{breakpoint, size};
use crate::presentation::{Type, Typeset, control_inset, print, search_field};
pub(crate) use recite_writer_app::workspaces::declarations::*;
mod outcome;
mod producer;
mod source;
#[cfg(test)]
mod tests;
use crate::attention::{Pane, Region};
use crate::presentation::{label, paragraph, text_button};
use recite_writer_session::declarations::SchemaOutputStatus;
use xilem::{
    WidgetView,
    masonry::properties::TextAreaLabel,
    masonry::{kurbo::Axis, layout::AsUnit},
    style::Style,
    view::{CrossAxisAlignment, flex_col, flex_row, text_input},
};

pub(crate) fn panel(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = writer.ui_scale();
    flex_col((
        crate::view::tool_head(writer, "Declarations", (source::apply(writer),)),
        segments(
            &writer.palette,
            (
                segment(
                    scale,
                    &writer.palette,
                    "Inspect declarations",
                    writer.workspaces.declarations.mode == Mode::Inspect,
                    false,
                    |writer: &mut Writer| writer.workspaces.declarations.mode = Mode::Inspect,
                ),
                segment(
                    scale,
                    &writer.palette,
                    "Declaration source",
                    writer.workspaces.declarations.mode == Mode::Source,
                    false,
                    Writer::show_declaration_source,
                ),
                segment(
                    scale,
                    &writer.palette,
                    "Producer",
                    writer.workspaces.declarations.mode == Mode::Producer,
                    false,
                    Writer::show_declaration_producer,
                ),
            ),
        ),
        xilem::view::indexed_stack((
            inspection(writer),
            source::panel(writer),
            producer::controls(writer),
        ))
        .active(match writer.workspaces.declarations.mode {
            Mode::Inspect => 0,
            Mode::Source => 1,
            Mode::Producer => 2,
        }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::TEXT.px())
    .prop(Region(Some(Pane::Declarations)))
}

/// The declarations as master and detail on one sheet: the searchable list in a column, and the
/// chosen declaration beside it; on a narrow sheet the detail follows the list.
fn inspection(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = f64::from(writer.ui_scale());
    let width = writer.content_width();
    let wide = width >= breakpoint::MASTER_DETAIL * scale;
    let master = if wide {
        size::MASTER_COLUMN * scale
    } else {
        width
    };
    let detail = if wide {
        width - master - space::GROUPS
    } else {
        width
    };
    flex_row((
        flex_col((list(writer, master),)).width(master.px()),
        flex_col((declaration_detail(writer, detail),))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .width(detail.px()),
    ))
    .direction(if wide {
        Axis::Horizontal
    } else {
        Axis::Vertical
    })
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::GROUPS.px())
}

fn list(writer: &Writer, width: f64) -> impl WidgetView<Writer> + use<> {
    let scale = writer.ui_scale();
    let panel = &writer.workspaces.declarations;
    flex_col((
        writer
            .declaration_session()
            .filter(|session| session.output_status() == SchemaOutputStatus::Damaged)
            .map(|_| {
                paragraph(
                    scale,
                    "Generated schema is damaged; showing the last valid declarations.",
                    width,
                )
            }),
        label(scale, "Search declarations"),
        search_field(
            scale,
            writer.palette.muted,
            false,
            None,
            text_input(panel.query.clone(), |writer: &mut Writer, query| {
                writer.workspaces.declarations.filter(query)
            })
            .placeholder("Name or kind")
            .typeset(Type::Body, scale)
            .prop(TextAreaLabel::new("Search declarations")),
        ),
        print::meta(
            scale,
            if panel.matches.is_empty() {
                if panel.query.trim().is_empty() {
                    "No declarations in this project.".into()
                } else {
                    "No declarations match this search.".into()
                }
            } else {
                format!(
                    "Showing {}–{} of {} {}",
                    panel.page * PAGE + 1,
                    ((panel.page + 1) * PAGE).min(panel.matches.len()),
                    panel.matches.len(),
                    if panel.matches.len() == 1 {
                        "declaration"
                    } else {
                        "declarations"
                    },
                )
            },
        )
        .color(writer.palette.muted),
        declaration_rows(writer, width),
        (panel.matches.len() > PAGE).then(|| {
            flex_row((
                text_button(scale, "Previous declarations", |writer: &mut Writer| {
                    writer.workspaces.declarations.page =
                        writer.workspaces.declarations.page.saturating_sub(1);
                })
                .disabled(panel.page == 0),
                text_button(scale, "Next declarations", |writer: &mut Writer| {
                    writer.workspaces.declarations.page += 1
                })
                .disabled((panel.page + 1) * PAGE >= panel.matches.len()),
            ))
            .gap(space::SIBLINGS.px())
        }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Stretch)
    .gap(space::SIBLINGS.px())
}

/// One row per declaration on this page, "name · kind" on one line from the row's start.
fn declaration_rows(writer: &Writer, width: f64) -> impl WidgetView<Writer> + use<> {
    let scale = writer.ui_scale();
    let panel = &writer.workspaces.declarations;
    // The row's padding and border either side, so the line fills the row.
    let line = width - 2. * control_inset(scale);
    flex_col(
        panel
            .matches
            .iter()
            .skip(panel.page * PAGE)
            .take(PAGE)
            .map(|index| {
                let Row { name, kind, .. } = &panel.rows[*index];
                let (name, kind) = (name.clone(), kind.clone());
                let selected = panel.selected == Some(*index);
                let caption = format!("{name} · {kind}");
                listed(
                    &writer.palette,
                    selected,
                    row(
                        selected,
                        one_line(caption, Type::Control, scale, writer.palette.ink)
                            .width(line.px()),
                        move |writer: &mut Writer| {
                            writer.workspaces.declarations.choose(&name, &kind)
                        },
                    ),
                )
            })
            .collect::<Vec<_>>(),
    )
    .cross_axis_alignment(CrossAxisAlignment::Stretch)
    .gap(0.px())
}

fn declaration_detail(writer: &Writer, width: f64) -> impl WidgetView<Writer> + use<> {
    let panel = &writer.workspaces.declarations;
    let scale = writer.ui_scale();
    let chosen = panel.selected.and_then(|index| panel.rows.get(index));
    flex_col((
        chosen.is_none().then(|| {
            print::meta(scale, "Choose a declaration to see its fields.")
                .color(writer.palette.muted)
        }),
        chosen.map(
            |Row {
                 name, kind, detail, ..
             }| {
                flex_col((
                    label(scale, name.clone()).typeset(Type::Section, scale),
                    print::meta(scale, kind.clone()).color(writer.palette.muted),
                    paragraph(
                        scale,
                        detail.get(panel.detail_page).cloned().unwrap_or_default(),
                        width,
                    ),
                    (detail.len() > 1).then(|| {
                        flex_row((
                            text_button(
                                scale,
                                "Previous declaration detail",
                                |writer: &mut Writer| {
                                    writer.workspaces.declarations.detail_page = writer
                                        .workspaces
                                        .declarations
                                        .detail_page
                                        .saturating_sub(1);
                                },
                            )
                            .disabled(panel.detail_page == 0),
                            text_button(scale, "More declaration detail", |writer: &mut Writer| {
                                writer.workspaces.declarations.detail_page += 1
                            })
                            .disabled(detail.len() <= panel.detail_page + 1),
                        ))
                        .gap(space::SIBLINGS.px())
                    }),
                ))
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .gap(space::SIBLINGS.px())
            },
        ),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
}
