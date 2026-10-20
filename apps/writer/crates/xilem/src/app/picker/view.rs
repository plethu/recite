use super::{App, Entry, Item, Mode, PAGE, Picker};
use crate::presentation::{label, paragraph, text_button};
use xilem::{
    WidgetView,
    masonry::{
        core::{Property, UsesProperty},
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
impl UsesProperty<ActiveResult> for xilem::masonry::widgets::Button {}

pub(in crate::app) fn picker(app: &App) -> impl WidgetView<App> + use<> {
    let scale = crate::presentation::scale(&app.preferences.config.writer);
    let picker = app.picker.as_ref();
    let mode = picker.map_or(Mode::Commands, |picker| picker.mode);
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
                    result(app, &picker.entries[*index], position, picker.active)
                })
        })
        .collect::<Vec<_>>();
    sized_box(
        flex_col((
            heading(scale, mode),
            query(scale, mode, picker),
            paragraph(scale, guidance(mode), 700.),
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
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(12.px()),
    )
    .width(752.px())
    .padding(24.px())
    .background_color(app.theme.palette.canvas)
}

fn heading(scale: f32, mode: Mode) -> impl WidgetView<App> + use<> {
    flex_row((
        label(scale, mode.title()).text_size(24. * scale),
        text_button(scale, "Close search", App::dismiss_picker),
    ))
    .gap(24.px())
}

fn query(scale: f32, mode: Mode, picker: Option<&Picker>) -> impl WidgetView<App> + use<> {
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
    .width(700.px())
}

fn result(
    app: &App,
    entry: &Entry,
    position: usize,
    active: usize,
) -> impl WidgetView<App> + use<> {
    let scale = crate::presentation::scale(&app.preferences.config.writer);
    let selected = position == active;
    let enabled = match entry.item {
        Item::Command(command) => app.command_enabled(command),
        Item::Location(_) => app
            .writer
            .as_ref()
            .is_some_and(|writer| !writer.navigation.busy() && !writer.work.busy()),
    };
    let title = format!(
        "{}{}",
        if selected { "Selected · " } else { "" },
        entry.title
    );
    let detail = if enabled {
        entry.detail.clone()
    } else {
        "Unavailable in this context".into()
    };
    button(
        flex_col((
            label(scale, title).prop(LineBreaking::WordWrap),
            (!detail.is_empty()).then(|| paragraph(scale, detail, 640.)),
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
) -> impl WidgetView<App> + use<> {
    flex_row((
        text_button(scale, "Previous results", App::previous_picker_page).disabled(page == 0),
        text_button(scale, "Next results", App::next_picker_page)
            .disabled((page + 1) * PAGE >= count),
        picker
            .filter(|picker| picker.total > picker.entries.len())
            .map(|_| text_button(scale, "Load more matches", App::load_picker_matches)),
    ))
    .gap(8.px())
}

fn result_count(picker: Option<&Picker>, page: usize, count: usize) -> String {
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
            "Type a command. Use Up and Down to choose; Enter runs it. Escape returns to your document."
        }
        Mode::GoTo => {
            "Find an accepted scene or beat by name. Linked navigation applies your current draft before leaving it."
        }
        Mode::ProjectText => {
            "Search saved project text. Accepted edits appear here after saving; field drafts are excluded."
        }
    }
}
