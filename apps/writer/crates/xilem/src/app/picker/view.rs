use super::{App, Entry, Item, Mode, PAGE, Picker, Purpose, Search};
use crate::presentation::{fit_content, label, paragraph, text_button};
use crate::presentation::{group::dialog, manuscript_search, radio_button, search_label};
use recite_writer_session::choices::display_name;
use xilem::core::one_of::Either;
use xilem::masonry::widgets;
use xilem::{
    WidgetView,
    masonry::{
        core::{Property, UsesProperty},
        kurbo::Axis,
        layout::{AsUnit, Dim},
        properties::{LineBreaking, TextAreaLabel},
        widgets::TextInput,
    },
    style::Style,
    view::{
        CrossAxisAlignment, FlexExt, button, flex_col, flex_row, portal, radio_group, sized_box,
        text_input,
    },
};
mod controls;
mod results;
use controls::{guidance, heading, keyboard_help, query, scopes};
use results::{empty_message, pagination, result, result_count};
#[cfg(test)]
mod tests;

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
    let width =
        (app.viewport.width - 4. * padding).clamp(240., (760. * f64::from(scale)).min(1000.));
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
    dialog(
        mode.title(),
        fit_content(
            sized_box(
                flex_col((
                    heading(scale, mode, width),
                    picker
                        .filter(|picker| {
                            matches!(picker.purpose, super::Purpose::Search(_))
                                && picker.mode() != Mode::Commands
                        })
                        .map(|_| scopes(app, width)),
                    app.writer.as_ref().map(|writer| {
                        let beat = writer.reading.selected.as_deref().map(display_name);
                        paragraph(
                            scale,
                            beat.map_or_else(
                                || writer.session.model().document().key().to_string(),
                                |beat| {
                                    format!("{beat} · {}", writer.session.model().document().key())
                                },
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
            .border_width(1.px())
            .border_color(app.theme.palette.accent),
        ),
    )
    .dims(Dim::FitContent)
}
