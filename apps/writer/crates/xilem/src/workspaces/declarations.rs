//! Frontend views and native glue over `recite_writer_app::workspaces::declarations`.
use super::*;
use crate::presentation::row::{listed, row};
use crate::presentation::segments::{segment, segments};
use crate::presentation::sheet_text::one_line;
use crate::presentation::space;
use crate::presentation::tokens::{breakpoint, size};
use crate::presentation::{Type, Typeset, control_inset, print, search_field};
pub(crate) use recite_writer_app::workspaces::declarations::*;
mod detail;
mod outcome;
mod producer;
mod source;
#[cfg(test)]
mod tests;
use crate::attention::{Pane, Region};
use crate::presentation::{label, paragraph, text_button};
use recite_writer_session::declarations::SchemaOutputStatus;
use xilem::core::one_of::Either;
use xilem::{
    WidgetView,
    masonry::properties::TextAreaLabel,
    masonry::{kurbo::Axis, layout::AsUnit, properties::Dimensions},
    style::Style,
    view::{CrossAxisAlignment, FlexExt, flex_col, flex_row, text_input, virtual_scroll},
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
    // The detail takes the list's height beside it, or a picker's below it.
    let rows = size::PICKER_ROWS as f64 * size::ROW * scale;
    let room = if wide {
        (writer.viewport.height - size::MASTER_CHROME * scale).max(rows)
    } else {
        rows
    };
    flex_row((
        flex_col((list(writer, master, wide),)).width(master.px()),
        flex_col((detail::detail(writer, detail, room),))
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

fn list(writer: &Writer, width: f64, wide: bool) -> impl WidgetView<Writer> + use<> {
    let scale = writer.ui_scale();
    let panel = &writer.workspaces.declarations;
    let count = panel.matches.len();
    let total = panel.rows.len();
    let noun = if total == 1 {
        "declaration"
    } else {
        "declarations"
    };
    // Only the rows in view are mounted; a wide sheet gives the column the sheet's height and a
    // stacked one keeps room for the detail below.
    let fitting = if wide {
        ((writer.viewport.height / f64::from(scale) - size::MASTER_CHROME) / size::ROW).floor()
            as usize
    } else {
        size::PICKER_ROWS
    };
    let lines = count.min(fitting.max(1)) as f64 * size::ROW * f64::from(scale);
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
        // The icon and placeholder say what the field is; its accessible name says it in full.
        search_field(
            scale,
            writer.palette.muted,
            false,
            None,
            text_input(panel.query.clone(), |writer: &mut Writer, query| {
                writer.workspaces.declarations.filter(query)
            })
            .placeholder("Search by name or kind")
            .typeset(Type::Body, scale)
            .prop(TextAreaLabel::new("Search declarations")),
        ),
        print::meta(
            scale,
            if total == 0 {
                "No declarations in this project.".into()
            } else if count == 0 {
                "No declarations match this search.".into()
            } else if panel.query.trim().is_empty() {
                format!("{total} {noun}")
            } else {
                format!("{count} of {total} {noun}")
            },
        )
        .color(writer.palette.muted),
        (count > 0).then(|| {
            virtual_scroll(count, move |writer: &mut Writer, position| {
                declaration_row(writer, position, width)
            })
            // Each row is a Tab stop; the list itself is not.
            .accessible_label("Declarations")
            .focusable(false)
            .jump_to(Some(panel.chosen_position()))
            .dims((width.px(), lines.px()))
        }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Stretch)
    .gap(space::TEXT.px())
}

/// The match at `position` as one 32px row, the name from the row's start and the kind set
/// against its end, read as "name · kind"; nothing once the matches change beneath the list.
fn declaration_row(
    writer: &Writer,
    position: usize,
    width: f64,
) -> impl WidgetView<Writer> + use<> {
    let scale = writer.ui_scale();
    let panel = &writer.workspaces.declarations;
    let Some(index) = panel.matches.get(position).copied() else {
        return Either::B(flex_col(()));
    };
    let Row { name, kind, .. } = &panel.rows[index];
    let (name, kind) = (name.clone(), kind.clone());
    let selected = panel.selected == Some(index);
    // The row's padding and border either side, so the line fills the row.
    let line = width - 2. * control_inset(scale);
    let height = f64::from(Type::Control.metrics().1 * scale);
    let caption = format!("{name} · {kind}");
    Either::A(listed(
        &writer.palette,
        selected,
        row(
            selected,
            flex_row((
                one_line(name.clone(), Type::Control, scale, writer.palette.ink).flex(1.),
                one_line(kind.clone(), Type::Meta, scale, writer.palette.muted),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Center)
            .gap(space::TEXT.px())
            .dims(
                Dimensions::AUTO
                    .with_width(line.px())
                    .with_height(height.px()),
            ),
            move |writer: &mut Writer| writer.workspaces.declarations.choose(&name, &kind),
        )
        .accessible_label(caption),
    ))
}
