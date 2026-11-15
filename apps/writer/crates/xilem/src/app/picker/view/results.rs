//! One 32px line per result: the title on the left, its shortcut or description as Meta on the
//! right, and the active result marked by the row's `selected` state.
use super::*;
use crate::presentation::Type;
use crate::presentation::row::{listed, row};
use crate::presentation::sheet_text::one_line;
use xilem::masonry::properties::Dimensions;

/// The title's share of a result line against its description's one.
const TITLE_SHARE: f64 = 2.;

pub(super) fn result(
    app: &App,
    entry: &Entry,
    position: usize,
    active: usize,
    width: f64,
) -> impl WidgetView<App> + use<> {
    let scale = crate::presentation::scale(&app.preferences.config.writer);
    let palette = &app.theme.palette;
    let query = app
        .picker
        .as_ref()
        .map_or_else(String::new, |picker| picker.query.clone());
    let selected = position == active;
    let enabled = match entry.item {
        Item::Command(command) => app.command_enabled(command),
        Item::Location(_) | Item::Lookup(_) | Item::Passage { .. } => app
            .writer
            .as_ref()
            .is_some_and(|writer| !writer.navigation.busy() && !writer.work.busy()),
        Item::Rule(_) | Item::Destination(_) | Item::RenameBeat(_) => app
            .picker
            .as_ref()
            .zip(app.writer.as_ref())
            .is_some_and(|(picker, writer)| match &picker.purpose {
                Purpose::Editor(request) => request.is_current(writer),
                Purpose::Search(_) => false,
            }),
    };
    let detail = if enabled {
        match &entry.item {
            Item::Passage { .. } => app
                .picker
                .as_ref()
                .and_then(|picker| picker.previews.get(position))
                .unwrap_or_default()
                .to_owned(),
            _ => entry.detail.clone(),
        }
    } else {
        "Unavailable in this context".into()
    };
    let name = if matches!(entry.item, Item::Passage { .. }) && !detail.is_empty() {
        format!("{} {detail}", entry.title)
    } else {
        entry.title.clone()
    };
    // The row's padding and border either side, so the line fills the row; one Control line
    // tall, so every result is one 32px row.
    let line = width - 2. * crate::presentation::control_inset(scale);
    let height = f64::from(Type::Control.metrics().1 * scale);
    listed(
        palette,
        selected,
        row(
            selected,
            flex_row((
                search_label(Type::Control.size(scale), entry.title.clone(), query)
                    .single_line()
                    .flex(TITLE_SHARE),
                // Descriptions give way to the title, set against the row's right edge.
                (!detail.is_empty()).then(|| {
                    flex_col((one_line(detail, Type::Meta, scale, palette.muted),))
                        .cross_axis_alignment(CrossAxisAlignment::End)
                        .flex(1.)
                }),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Center)
            .gap(space::TEXT.px())
            .dims(
                Dimensions::AUTO
                    .with_width(line.px())
                    .with_height(height.px()),
            ),
            move |app: &mut App| app.activate_picker(position),
        )
        // A shortcut or kind is not part of the row's name; a passage's words are.
        .accessible_label(name)
        .disabled(!enabled)
        .prop(ActiveResult(selected)),
    )
}

/// Project text search lists its first hundred matches; this reaches the rest.
pub(super) fn more(scale: f32, picker: Option<&Picker>) -> Option<impl WidgetView<App> + use<>> {
    picker
        .filter(|picker| picker.total > picker.entries.len())
        .map(|_| {
            crate::presentation::text_button(scale, "Load more matches", |app: &mut App| {
                app.load_picker_matches()
            })
        })
}
