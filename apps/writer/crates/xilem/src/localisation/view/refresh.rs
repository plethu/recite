use super::*;
use crate::WriterLayout;
use crate::attention::PaneEntry;
use crate::presentation::control_style::{ControlStyle, styled};
use crate::presentation::segments::{segment_titled, segments};
use crate::presentation::sheet_text::one_line;
use crate::presentation::space;
use crate::presentation::tokens::size;
use crate::presentation::{Type, disclosure, print};
use recite_writer_app::localisation::refresh::Update;
use recite_writer_session::typography::FONT_PROSE;
use xilem::masonry::parley::style::FontWeight;
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
/// Every changed entry as a row under its file's heading: its source line as a disclosure for its
/// diff, and Accept or Keep. Kept entries stay as they are when the updates are applied.
fn updates(writer: &Writer, blocked: bool) -> impl WidgetView<Writer> + use<> {
    let refresh = writer.localisation.refresh.as_ref();
    let total = refresh.map_or(0, |refresh| refresh.rows.len());
    let mut groups: Vec<(String, Vec<_>)> = Vec::new();
    for (index, update) in refresh
        .map(|refresh| refresh.rows.as_slice())
        .unwrap_or_default()
        .iter()
        .take(SHOWN_ROWS)
        .enumerate()
    {
        let row = row(writer, index, update, blocked);
        match groups.last_mut() {
            Some((file, rows)) if *file == update.file => rows.push(row),
            _ => groups.push((update.file.clone(), vec![row])),
        }
    }
    let scale = writer.ui_scale();
    let groups = groups
        .into_iter()
        .map(|(file, rows)| {
            flex_col((
                // The file once, over the rows that come from it.
                one_line(
                    if file.is_empty() {
                        "Other entries".to_owned()
                    } else {
                        file
                    },
                    Type::Control,
                    scale,
                    writer.palette.ink,
                )
                .weight(FontWeight::SEMI_BOLD),
                flex_col(rows)
                    .cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .gap(space::UNIT.px()),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .gap(space::SIBLINGS.px())
        })
        .collect::<Vec<_>>();
    flex_col((
        flex_col(groups)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .gap(space::KINDS.px()),
        (total > SHOWN_ROWS).then(|| {
            print::meta(
                scale,
                format!(
                    "The first {SHOWN_ROWS} of {total} changes are listed; Apply covers them all."
                ),
            )
            .color(writer.palette.muted)
        }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Stretch)
    .gap(space::SIBLINGS.px())
    .width(writer.page_width().px())
}
/// A row's own name: its kind, its source line and its line id, so that every row and its
/// Accept and Keep read apart however alike their lines.
fn name(update: &Update) -> String {
    format!("{} · {} · {}", update.kind, update.excerpt, update.context)
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
    let name = name(update);
    let meta = if update.beat.is_empty() {
        update.kind.to_owned()
    } else {
        format!("{} · {}", update.beat, update.kind)
    };
    let choice = segments(
        palette,
        (
            segment_titled(
                scale,
                palette,
                ("Accept", format!("Accept · {name}")),
                !kept,
                blocked,
                move |writer: &mut Writer| writer.keep_source_update(index, false),
            ),
            segment_titled(
                scale,
                palette,
                ("Keep", format!("Keep · {name}")),
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
                        flex_col((
                            // The source line in the reading face, then where it is, as meta.
                            one_line(update.excerpt.clone(), Type::Body, scale, palette.ink)
                                .families(FONT_PROSE),
                            one_line(meta, Type::Meta, scale, palette.muted),
                        ))
                        .cross_axis_alignment(CrossAxisAlignment::Stretch)
                        .gap(0.px()),
                    ),
                    move |writer: &mut Writer| writer.toggle_source_update(index),
                )
                .accessible_label(name)
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
