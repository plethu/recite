//! Bounded, selectable native search results and their page controls.
use super::*;

pub(super) fn result(
    app: &App,
    entry: &Entry,
    position: usize,
    active: usize,
    width: f64,
) -> impl WidgetView<App> + use<> {
    let scale = crate::presentation::scale(&app.preferences.config.writer);
    let query = app
        .picker
        .as_ref()
        .map_or_else(String::new, |picker| picker.query.clone());
    let selected = position == active;
    let reading_size = app
        .writer
        .as_ref()
        .map_or(20., |writer| writer.reading_size());
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
    let title = format!(
        "{}{}",
        if selected { "Selected · " } else { "" },
        entry.title
    );
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
    button(
        flex_col((
            search_label((14. * scale).min(0.75 * reading_size), title, query.clone()),
            (!detail.is_empty()).then(|| {
                let prose = matches!(entry.item, Item::Passage { .. } | Item::Location(_));
                let width = (width - 24. * f64::from(scale)).max(160.);
                if prose {
                    Either::A(manuscript_search(reading_size, detail, query).width(width.px()))
                } else {
                    Either::B(paragraph(scale, detail, width))
                }
            }),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(4.px()),
        move |app: &mut App| app.activate_picker(position),
    )
    .disabled(!enabled)
    .background_color(if selected {
        app.theme.palette.selection
    } else {
        app.theme.palette.surface
    })
    .prop(ActiveResult(selected))
}

pub(super) fn pagination(
    scale: f32,
    picker: Option<&Picker>,
    page: usize,
    count: usize,
) -> Option<impl WidgetView<App> + use<>> {
    (count > PAGE || picker.is_some_and(|picker| picker.total > count)).then(|| {
        flex_row((
            text_button(scale, "Previous results", |app: &mut App| {
                app.previous_picker_page()
            })
            .disabled(page == 0),
            text_button(scale, "Next results", |app: &mut App| {
                app.next_picker_page()
            })
            .disabled((page + 1) * PAGE >= count),
            picker
                .filter(|picker| picker.total > picker.entries.len())
                .map(|_| {
                    text_button(scale, "Load more matches", |app: &mut App| {
                        app.load_picker_matches()
                    })
                }),
        ))
        .gap(8.px())
    })
}
