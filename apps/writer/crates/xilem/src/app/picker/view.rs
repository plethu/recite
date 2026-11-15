//! The picker: a slip anchored near the top of the window over the backdrop, with the query, the
//! scope where there is one, one 32px line per result and one hint line.
use super::{App, Entry, Item, Mode, Picker, Purpose, Search};
use crate::app::keyboard::ModalSurface;
use crate::presentation::row::RowButton;
use crate::presentation::slip::slip;
use crate::presentation::tokens::size;
use crate::presentation::{Icon, Type, print, space};
use crate::presentation::{fit_content, group::dialog, search_label};
use xilem::core::{MessageResult, one_of::Either};
use xilem::{
    WidgetView,
    masonry::{
        core::{Property, UsesProperty},
        kurbo::Axis,
        layout::{AsUnit, Dim},
        properties::TextAreaLabel,
        widgets::TextInput,
    },
    style::Style,
    view::{
        CrossAxisAlignment, FlexExt, flex_col, flex_row, sized_box, text_input, virtual_scroll,
    },
};
mod controls;
mod results;
use controls::{query, scopes};
use recite_writer_app::shell::picker::{empty_message, hint};
use results::{more, result};
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
impl UsesProperty<ActiveResult> for RowButton {}

/// The picker's content width at `scale` in a window `viewport` wide.
fn width(viewport: f64, scale: f32) -> f64 {
    (size::PICKER_WIDTH * f64::from(scale)).min(viewport - 2. * space::GROUPS - 2. * space::KINDS)
}

/// How many result lines fit between the picker's top and the status bar in a window `height`
/// tall: the slip's padding, query line, gaps and hint line (and scope line) come first.
fn fitting_lines(height: f64, scale: f32, scoped: bool) -> usize {
    let scale = f64::from(scale);
    let line = f64::from(Type::Meta.metrics().1);
    let chrome = 2. * space::KINDS
        + size::ROW
        + 2. * space::TEXT
        + line
        + if scoped { size::ROW + space::TEXT } else { 0. };
    // The status bar, and as much desk below the slip as above the workspace.
    let below = 2. * space::UNIT + line + space::KINDS;
    let room = height - (size::PICKER_TOP + chrome + below) * scale;
    ((room / (size::ROW * scale)).floor().max(1.) as usize).min(size::PICKER_ROWS)
}

/// The result line at `position`, or nothing once the matches have changed beneath the list.
fn line(app: &App, position: usize, width: f64) -> impl WidgetView<App> + use<> {
    let entry = app.picker.as_ref().and_then(|picker| {
        picker
            .matches
            .get(position)
            .map(|index| (&picker.entries[*index], picker.active))
    });
    match entry {
        Some((entry, active)) => Either::A(result(app, entry, position, active, width)),
        None => Either::B(flex_col(())),
    }
}

pub(in crate::app) fn picker(app: &App) -> impl WidgetView<App> + use<> {
    let scale = crate::presentation::scale(&app.preferences.config.writer);
    let width = width(app.viewport.width, scale);
    let picker = app.picker.as_ref();
    let mode = picker.map_or(Mode::Commands, Picker::mode);
    let count = picker.map_or(0, |picker| picker.matches.len());
    let anchor = picker.map_or(0, |picker| picker.anchor);
    // Every match is one 32px line. Only the lines in view are mounted; the rest scroll, and
    // the list gives up lines before the slip would reach the status bar.
    let scoped = picker.is_some_and(|picker| {
        matches!(picker.purpose, Purpose::Search(_)) && picker.mode() != Mode::Commands
    });
    let lines = count.min(fitting_lines(app.viewport.height, scale, scoped)) as f64
        * size::ROW
        * f64::from(scale);
    dialog(
        mode.title(),
        slip(
            &app.theme.palette,
            fit_content(
                sized_box(
                    flex_col((
                        flex_row((
                            query(scale, picker, width, app.theme.palette.muted).flex(1.),
                            print::icon_action(
                                scale,
                                "Close search",
                                Icon::Close,
                                &app.theme.palette,
                                false,
                                |app: &mut App| app.dismiss_picker(),
                            ),
                        ))
                        .cross_axis_alignment(CrossAxisAlignment::Center)
                        .gap(space::SIBLINGS.px())
                        .width(width.px()),
                        scoped.then(|| scopes(app, width)),
                        (count == 0).then(|| {
                            print::meta(scale, empty_message(mode, picker))
                                .color(app.theme.palette.muted)
                        }),
                        (count > 0).then(|| {
                            virtual_scroll(count, move |app: &mut App, position| {
                                line(app, position, width)
                            })
                            .jump_to(Some(anchor))
                            .on_scroll(|app: &mut App, visible| {
                                app.picker_scrolled(visible);
                                MessageResult::Nop
                            })
                            .dims((width.px(), lines.px()))
                        }),
                        more(scale, picker),
                        print::meta(scale, hint(picker)).color(app.theme.palette.muted),
                    ))
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .gap(space::TEXT.px()),
                )
                .width((width + 2. * space::KINDS).px())
                .padding(space::KINDS.px())
                .prop(ModalSurface(true)),
            ),
        ),
    )
    .dims(Dim::FitContent)
}
