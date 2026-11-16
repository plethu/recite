use super::*;
use crate::attention::PaneEntry;
use crate::presentation::control_style::{ControlStyle, styled};
use crate::presentation::segments::{segment_titled, segments};
use crate::presentation::sheet_text::one_line;
use crate::presentation::space;
use crate::presentation::tokens::size;
use crate::presentation::{Type, disclosure, print};
use recite_writer_app::localisation::refresh::Update;
use xilem::view::{FlexExt, button};

pub(super) fn refresh(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let state = &writer.localisation;
    let refresh = state.refresh.as_ref()?;
    let scale = writer.ui_scale();
    let blocked = writer.project_action_blocked() || state.slot.read_only();
    let current = state
        .slot
        .owner()
        .is_some_and(|owner| refresh.preview.is_current(&owner.catalogue));
    Some(
        flex_col((
            crate::view::captioned_head(
                writer,
                (
                    "Source updates",
                    Some(refresh.preview.summary.clone().into()),
                    (
                        styled(
                            ControlStyle::Action,
                            button(
                                Type::Control.label(scale, "Recheck"),
                                Writer::review_catalogue_sources,
                            )
                            .accessible_label("Recheck source updates")
                            .disabled(blocked || state.dirty()),
                        ),
                        print::primary_action(
                            scale,
                            "Apply source updates to PO",
                            &writer.palette,
                            blocked || !current,
                            Writer::apply_catalogue_sources,
                        ),
                    ),
                ),
                // Closing the review returns to the queue it was opened from.
                (
                    "Return to translation queue",
                    Writer::return_to_translation_queue,
                ),
            )
            .prop(PaneEntry(true)),
            (!refresh.rows.is_empty()).then(|| updates(writer, blocked)),
            refresh.rows.is_empty().then(|| {
                paragraph(
                    scale,
                    "No source text changes. Applying also refreshes source metadata.",
                    size::MEASURE,
                )
            }),
            (!current).then(|| {
                paragraph(
                    scale,
                    "The catalogue changed after this review. Recheck before \
                     applying source updates.",
                    size::MEASURE,
                )
            }),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(space::TEXT.px()),
    )
}
/// The most rows the list mounts; Apply covers every change however many there are.
const SHOWN_ROWS: usize = 200;
/// Every changed entry as a row: its caption as a disclosure for its diff, and Accept or Keep.
/// Kept entries stay as they are when the updates are applied.
fn updates(writer: &Writer, blocked: bool) -> impl WidgetView<Writer> + use<> {
    let refresh = writer.localisation.refresh.as_ref();
    let total = refresh.map_or(0, |refresh| refresh.rows.len());
    let rows = refresh
        .map(|refresh| {
            refresh
                .rows
                .iter()
                .take(SHOWN_ROWS)
                .enumerate()
                .map(|(index, update)| row(writer, index, update, blocked))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    flex_col((
        flex_col(rows)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .gap(space::UNIT.px()),
        (total > SHOWN_ROWS).then(|| {
            print::meta(
                writer.ui_scale(),
                format!(
                    "The first {SHOWN_ROWS} of {total} changes are listed; Apply covers them all."
                ),
            )
            .color(writer.palette.muted)
        }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Stretch)
    .gap(space::SIBLINGS.px())
    .width(writer.content_width().px())
}
fn row(
    writer: &Writer,
    index: usize,
    update: &Update,
    blocked: bool,
) -> impl WidgetView<Writer> + use<> {
    let scale = writer.ui_scale();
    let palette = &writer.palette;
    let refresh = writer.localisation.refresh.as_ref();
    let open = refresh.is_some_and(|refresh| refresh.open.contains(&index));
    let kept = refresh.is_some_and(|refresh| refresh.is_kept(index));
    let caption = update.source.caption.clone();
    let choice = segments(
        palette,
        (
            segment_titled(
                scale,
                palette,
                ("Accept", format!("Accept · {caption}")),
                !kept,
                blocked,
                move |writer: &mut Writer| writer.keep_source_update(index, false),
            ),
            segment_titled(
                scale,
                palette,
                ("Keep", format!("Keep · {caption}")),
                kept,
                blocked,
                move |writer: &mut Writer| writer.keep_source_update(index, true),
            ),
        ),
    );
    flex_col((
        flex_row((
            styled(
                ControlStyle::Navigation,
                button(
                    disclosure(
                        scale,
                        open,
                        palette.ink,
                        one_line(caption, Type::Control, scale, palette.ink),
                    ),
                    move |writer: &mut Writer| writer.toggle_source_update(index),
                )
                .expanded(open),
            )
            .flex(1.),
            choice,
        ))
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .gap(space::TEXT.px()),
        open.then(|| {
            flex_col((
                versions::versions(
                    writer,
                    &update.source,
                    "Previous source",
                    "Current project source",
                ),
                update
                    .detail
                    .as_ref()
                    .map(|detail| paragraph(scale, detail.clone(), size::MEASURE)),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(space::SIBLINGS.px())
        }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Stretch)
    .gap(space::SIBLINGS.px())
}
