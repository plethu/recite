use super::{App, Entry, Item, Mode, PAGE, Picker};
use crate::presentation::{label, paragraph, text_button};
use recite_writer_session::choices::display_name;
use xilem::masonry::widgets;
use xilem::{
    WidgetView,
    masonry::{
        core::{Property, UsesProperty},
        kurbo::Axis,
        layout::AsUnit,
        properties::{LineBreaking, TextAreaLabel},
        widgets::TextInput,
    },
    style::Style,
    view::{
        CrossAxisAlignment, FlexExt, button, flex_col, flex_row, portal, sized_box, text_input,
    },
};

#[derive(Clone, Copy, Default, PartialEq)]
pub(crate) struct QueryField(pub bool);
impl Property for QueryField {
    fn static_default() -> &'static Self {
        static OTHER: QueryField = QueryField(false);
        &OTHER
    }
}
impl UsesProperty<QueryField> for TextInput {}
#[derive(Clone, Copy, Default, PartialEq)]
pub(crate) struct ActiveResult(pub bool);
impl Property for ActiveResult {
    fn static_default() -> &'static Self {
        static OTHER: ActiveResult = ActiveResult(false);
        &OTHER
    }
}
impl UsesProperty<ActiveResult> for widgets::Button {}

pub(in crate::app) fn picker(app: &App) -> impl WidgetView<App> + use<> {
    let scale = crate::presentation::scale(&app.preferences.config.writer);
    let compact = app.viewport.height < 600. * f64::from(scale);
    let padding = if compact { 16. } else { 24. };
    let width = (app.viewport.width - 2. * padding).clamp(240., 700.);
    let picker = app.picker.as_ref();
    let mode = picker.map_or(Mode::Commands, Picker::mode);
    let page = picker.map_or(0, |picker| picker.active / PAGE);
    let count = picker.map_or(0, |picker| picker.matches.len());
    let rows = picker
        .into_iter()
        .flat_map(|picker| {
            picker
                .matches
                .iter()
                .enumerate()
                .skip(page * PAGE)
                .take(PAGE)
                .map(move |(position, index)| {
                    result(app, &picker.entries[*index], position, picker.active, width)
                })
        })
        .collect::<Vec<_>>();
    sized_box(
        flex_col((
            heading(scale, mode, width),
            app.writer.as_ref().map(|writer| {
                let beat = writer.reading.selected.as_deref().map(display_name);
                paragraph(
                    scale,
                    beat.map_or_else(
                        || writer.session.model().document().key().to_string(),
                        |beat| format!("{beat} · {}", writer.session.model().document().key()),
                    ),
                    width,
                )
            }),
            query(scale, mode, picker, width),
            paragraph(scale, guidance(mode), width),
            label(scale, result_count(picker, page, count)),
            (count == 0).then(|| label(scale, empty_message(mode, picker))),
            portal(
                flex_col(rows)
                    .cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .gap(8.px()),
            )
            .constrain_horizontal(true)
            .flex(1.),
            pagination(scale, picker, page, count),
            label(scale, keyboard_help(mode)),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(if compact { 8.px() } else { 12.px() }),
    )
    .width((width + 2. * padding).px())
    .padding(padding.px())
    .background_color(app.theme.palette.canvas)
}

fn heading(scale: f32, mode: Mode, width: f64) -> impl WidgetView<App> + use<> {
    let wide = width >= 300. * f64::from(scale);
    flex_row((
        label(scale, mode.title())
            .text_size(20. * scale)
            .prop(LineBreaking::WordWrap)
            .flex(if wide { 1. } else { 0. }),
        text_button(scale, "Close search", App::dismiss_picker),
    ))
    .direction(if wide {
        Axis::Horizontal
    } else {
        Axis::Vertical
    })
    .width(width.px())
    .gap(8.px())
}

fn query(
    scale: f32,
    mode: Mode,
    picker: Option<&Picker>,
    width: f64,
) -> impl WidgetView<App> + use<> {
    sized_box(
        text_input(
            picker.map_or_else(String::new, |picker| picker.query.clone()),
            App::filter_picker,
        )
        .placeholder(mode.title())
        .text_size(14. * scale)
        .on_enter(|app: &mut App, _| app.submit_picker())
        .on_cancel(|app: &mut App, _| app.dismiss_picker())
        .prop(TextAreaLabel::new("Search query"))
        .prop(QueryField(true)),
    )
    .width(width.px())
}

fn result(
    app: &App,
    entry: &Entry,
    position: usize,
    active: usize,
    width: f64,
) -> impl WidgetView<App> + use<> {
    let scale = crate::presentation::scale(&app.preferences.config.writer);
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
                super::Purpose::Editor(request) => request.is_current(writer),
                super::Purpose::Search(_) => false,
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
            label(scale, title).prop(LineBreaking::WordWrap),
            (!detail.is_empty())
                .then(|| paragraph(scale, detail, (width - 24. * f64::from(scale)).max(160.))),
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

fn pagination(
    scale: f32,
    picker: Option<&Picker>,
    page: usize,
    count: usize,
) -> Option<impl WidgetView<App> + use<>> {
    (count > PAGE || picker.is_some_and(|picker| picker.total > count)).then(|| {
        flex_row((
            text_button(scale, "Previous results", App::previous_picker_page).disabled(page == 0),
            text_button(scale, "Next results", App::next_picker_page)
                .disabled((page + 1) * PAGE >= count),
            picker
                .filter(|picker| picker.total > picker.entries.len())
                .map(|_| text_button(scale, "Load more matches", App::load_picker_matches)),
        ))
        .gap(8.px())
    })
}

fn result_count(picker: Option<&Picker>, page: usize, count: usize) -> String {
    if count <= PAGE && picker.is_none_or(|picker| picker.total <= count) {
        return format!("{count} {}", if count == 1 { "result" } else { "results" });
    }
    let total = picker
        .filter(|picker| picker.total > count)
        .map_or_else(String::new, |picker| format!(" of {}", picker.total));
    format!(
        "{count}{total} results · page {} of {}",
        page + 1,
        count.div_ceil(PAGE).max(1)
    )
}

fn empty_message(mode: Mode, picker: Option<&Picker>) -> &'static str {
    if mode == Mode::ProjectText && picker.is_some_and(|picker| picker.query.trim().is_empty()) {
        "Enter words to search the saved project."
    } else {
        "No matching results."
    }
}

fn guidance(mode: Mode) -> &'static str {
    match mode {
        Mode::Commands => {
            "Up/Down to choose. Enter runs the command. Esc returns to your document."
        }
        Mode::GoTo => {
            "Find a scene or beat by name. Your current draft is applied before navigation."
        }
        Mode::ProjectText => "Saved text · Save to include edits.",
        Mode::Passages => "Accepted lines and replies. Opening applies your current draft.",
        Mode::RulesChoices => {
            "Enter stages a value. Apply or Save commits it; Esc keeps your current draft."
        }
        Mode::Destinations => {
            "Choose a beat or END. Your current draft is applied; Save writes the change to disk."
        }
        Mode::RenameBeat => {
            "Choose a beat to rename. This updates the proposal; Esc keeps the current proposal."
        }
    }
}

fn keyboard_help(mode: Mode) -> &'static str {
    match mode {
        Mode::Commands => "Up/Down: choose · Enter: run · Esc: return",
        Mode::RulesChoices | Mode::Destinations | Mode::RenameBeat => {
            "Up/Down: choose · Enter: choose · Esc: return"
        }
        _ => "Up/Down: choose · Enter: open · Esc: return",
    }
}
