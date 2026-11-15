//! Adaptive document navigation keeps project tools available without displacing their content.
use super::*;
use crate::presentation::control_style::{ControlStyle, styled};
use crate::presentation::row::named_row;
use crate::presentation::space;
use crate::presentation::{Icon, Type};
use xilem::view::FlexSpacer;
use xilem::view::{GridExt, GridTrackSize, grid};

pub(in crate::view) fn project_header(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let blocked = state.phase() != Phase::Writing || state.work.busy() || state.navigation.busy();
    let scale = state.ui_scale();
    let wide = state.content_width()
        >= if state.compact_chrome() { 520. } else { 760. } * f64::from(scale);
    grid((
        history_buttons(state).grid_pos(0, 0),
        flex_row((
            print::action(
                scale,
                "Return to writing",
                state.phase() != Phase::Writing,
                Writer::return_to_writing,
            ),
            // Tool and Localise sheets keep their own primary; Save all stays tonal.
            print::emphasised_action(
                scale,
                "Save all",
                &state.palette,
                false,
                blocked
                    || state.session.read_only()
                    || state.source_repair_pending()
                    || state.external_resolution_pending()
                    || state.project_jobs_busy(),
                Writer::save_all,
            ),
            print::action(
                scale,
                "Close",
                blocked || state.project_jobs_busy(),
                Writer::request_close,
            ),
        ))
        .gap(space::SIBLINGS.px())
        .grid_pos(u16::from(wide), u16::from(!wide)),
    ))
    .columns([
        GridTrackSize::FRACTION,
        if wide {
            GridTrackSize::Auto
        } else {
            GridTrackSize::Fixed(0.px())
        },
    ])
    .rows([GridTrackSize::Auto; 2])
    .gap(space::UNIT.px())
    .padding(xilem::style::Padding::from_vh(
        space::UNIT.px(),
        space::KINDS.px(),
    ))
}

/// Focus mode keeps one quiet way out at the top right; Save, Close and Copy link live in
/// Commands.
pub(in crate::view) fn focus_header(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    flex_row((
        // Example changes are discarded on close; focus mode keeps saying so.
        state.session.project_root().is_none().then(|| {
            print::meta(
                scale,
                "Temporary example · changes are discarded when this window closes",
            )
            .color(state.palette.muted)
        }),
        FlexSpacer::Flex(1.),
        styled(
            ControlStyle::Action,
            named_row(
                "Exit focus",
                Type::Control.label(scale, "Exit focus · Esc"),
                Writer::toggle_focus,
            )
            .disabled(
                state.phase() != Phase::Writing || state.navigation.busy() || state.work.busy(),
            ),
        ),
    ))
    .padding(xilem::style::Padding::from_vh(
        space::UNIT.px(),
        space::KINDS.px(),
    ))
}

fn history_buttons(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let closing = state.phase() != Phase::Writing;
    flex_row((
        print::icon_action(
            scale,
            "Back",
            Icon::Back,
            &state.palette,
            closing
                || state.work.busy()
                || state.navigation.busy()
                || state.navigation.history.step(false).is_none(),
            |state: &mut Writer| state.history_step(false),
        ),
        print::icon_action(
            scale,
            "Forward",
            Icon::Forward,
            &state.palette,
            closing
                || state.work.busy()
                || state.navigation.busy()
                || state.navigation.history.step(true).is_none(),
            |state: &mut Writer| state.history_step(true),
        ),
        (!state.compact_chrome()).then(|| {
            print::meta(scale, state.document_caption())
                .prop(LineBreaking::WordWrap)
                .flex(1.)
        }),
        print::icon_action(
            scale,
            "Copy link",
            Icon::Link,
            &state.palette,
            closing || state.project_action_blocked(),
            Writer::copy_link,
        ),
    ))
    .gap(space::SIBLINGS.px())
}
