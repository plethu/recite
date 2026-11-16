//! The fixed writing shell retains content while its contextual panel comes and goes.
use super::{Phase, Writer, document_actions, navigation, reference, retained, toolbar};
use crate::presentation::group::inert;
use crate::presentation::{fit_content, pane, space};
use xilem::{
    WidgetView,
    masonry::layout::{AsUnit, UnitPoint},
    peniko::Color,
    style::{Padding, Style},
    view::{
        CrossAxisAlignment, FlexExt, ZStackExt, flex_col, flex_row, indexed_stack, sized_box,
        zstack,
    },
};

pub(super) fn frame<V: WidgetView<Writer>>(
    state: &Writer,
    content: V,
) -> impl WidgetView<Writer> + use<V> {
    let writing = state.workspaces.screen == crate::workspaces::Screen::Write;
    zstack((
        // Behind the close decision the document is inert.
        inert(
            state.phase() != Phase::Writing,
            flex_col((
                flex_row((
                    retained(navigation(state), state.navigation_width() > 0.),
                    flex_col((
                        // Tool and Localise screens share Script's one-row toolbar.
                        indexed_stack((
                            toolbar::toolbar(state),
                            document_actions::focus_header(state),
                        ))
                        .active(usize::from(writing && state.attention.is_focused())),
                        pane("Workspace", content)
                            .constrain_horizontal(true)
                            .constrain_vertical(writing)
                            // Writing sits on the desk; tool and Localise panels keep the surface.
                            .background_color(if writing {
                                Color::TRANSPARENT
                            } else {
                                state.palette.surface
                            })
                            .flex(1.),
                        retained(
                            flex_col((reference::pinned(state),))
                                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                                .gap(0.px()),
                            !state.trial.visible || state.attention.is_focused(),
                        ),
                    ))
                    .cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .gap(0.px())
                    .flex(1.),
                ))
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .gap(0.px())
                .flex(1.),
                toolbar::status(state),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .gap(0.px()),
        ),
        state
            .document_menu()
            .is_open()
            .then(|| document_actions::panel(state)),
        // A build's result waits on a slip just above the status line that reports it.
        crate::workspaces::build_result(state).map(|result| {
            sized_box(fit_content(result))
                .padding(Padding {
                    left: space::KINDS.px(),
                    bottom: (toolbar::status_height(state.ui_scale()) + space::SIBLINGS).px(),
                    ..Padding::ZERO
                })
                .alignment(UnitPoint::BOTTOM_LEFT)
        }),
        // Closing with unsaved work is decided on a slip over the dimmed document.
        document_actions::close_decision(state),
    ))
    .alignment(UnitPoint::TOP_RIGHT)
}
