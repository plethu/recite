//! The fixed writing shell retains content while its contextual panel comes and goes.
use super::{Phase, Writer, document_actions, navigation, reference, retained, toolbar};
use xilem::{
    WidgetView,
    masonry::layout::{AsUnit, UnitPoint},
    style::Style,
    view::{CrossAxisAlignment, FlexExt, flex_col, flex_row, indexed_stack, portal, zstack},
};

pub(super) fn frame<V: WidgetView<Writer>>(
    state: &Writer,
    content: V,
) -> impl WidgetView<Writer> + use<V> {
    indexed_stack((
        zstack((
            flex_col((
                flex_row((
                    retained(navigation(state), state.navigation_width() > 0.),
                    flex_col((
                        indexed_stack((
                            toolbar::toolbar(state),
                            document_actions::focus_header(state),
                            document_actions::project_header(state),
                        ))
                        .active(
                            if state.workspaces.screen != crate::workspaces::Screen::Write {
                                2
                            } else {
                                usize::from(state.attention.is_focused())
                            },
                        ),
                        portal(content)
                            .constrain_horizontal(true)
                            .constrain_vertical(
                                state.workspaces.screen == crate::workspaces::Screen::Write,
                            )
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
                    .background_color(state.palette.surface)
                    .flex(1.),
                ))
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .gap(0.px())
                .flex(1.),
                toolbar::status(state),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .gap(0.px()),
            state
                .document_menu()
                .is_open()
                .then(|| document_actions::panel(state)),
        ))
        .alignment(UnitPoint::TOP_RIGHT)
        .boxed(),
        document_actions::close_view(state).boxed(),
    ))
    .active(usize::from(state.phase() != Phase::Writing))
}
