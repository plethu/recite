//! The fixed writing shell retains content while its contextual panel comes and goes.
use super::{
    Phase, Writer, document_actions, navigation, reference, retained, status_line, toolbar,
};
use crate::presentation::group::inert;
use crate::presentation::material::{Texture, cover, screen};
use crate::presentation::pane;
use crate::workspaces::rename::inline::floats;
use xilem::{
    WidgetView,
    masonry::layout::{AsUnit, UnitPoint},
    style::Style,
    view::{CrossAxisAlignment, FlexExt, flex_col, flex_row, indexed_stack, zstack},
};

pub(super) fn frame<V: WidgetView<Writer>>(
    state: &Writer,
    content: V,
) -> impl WidgetView<Writer> + use<V> {
    let writing = state.workspaces.screen == crate::workspaces::Screen::Write;
    let inert_document = state.phase() != Phase::Writing;
    flex_col((
        zstack((
            // Behind the close decision the document is inert.
            inert(
                inert_document,
                flex_col((
                    flex_row((
                        under_sheet(
                            state,
                            retained(navigation(state), state.navigation_width() > 0.),
                        ),
                        flex_col((
                            // Tool and Localise screens share Script's one-row toolbar, which
                            // stays clear of the Settings sheet's dim (see `under_sheet`).
                            indexed_stack((
                                toolbar::toolbar(state),
                                document_actions::focus_header(state),
                            ))
                            .active(usize::from(writing && state.attention.is_focused())),
                            // Writing sits on the desk; tool and Localise pages rest on paper.
                            under_sheet(
                                state,
                                super::workspace::sheet::page(
                                    state,
                                    writing,
                                    pane("Workspace", content)
                                        .constrain_horizontal(true)
                                        .constrain_vertical(writing),
                                ),
                            )
                            .flex(1.),
                            under_sheet(
                                state,
                                retained(
                                    flex_col((reference::pinned(state),))
                                        .cross_axis_alignment(CrossAxisAlignment::Stretch)
                                        .gap(0.px()),
                                    !state.trial.visible || state.attention.is_focused(),
                                ),
                            ),
                        ))
                        .cross_axis_alignment(CrossAxisAlignment::Stretch)
                        .gap(0.px())
                        .flex(1.),
                    ))
                    .cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .gap(0.px())
                    .flex(1.),
                    // A build's result is a bar seated on the status line that reports it, across
                    // the window: the work above gives up only the bar's height.
                    crate::workspaces::build_result(state).map(|bar| under_sheet(state, bar)),
                ))
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .gap(0.px()),
            ),
            state
                .document_menu()
                .is_open()
                .then(|| document_actions::panel(state)),
            // Closing with unsaved work is decided on a slip over the dimmed document.
            document_actions::close_decision(state),
        ))
        .alignment(UnitPoint::TOP_RIGHT)
        .flex(1.),
        // The status line carries state, so no dim lies over it: behind a modal it is inert but
        // its words keep their full contrast against the bar.
        inert(
            inert_document || floats(state) || state.settings_sheet(),
            status_line::status_line(state),
        ),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Stretch)
    .gap(0.px())
}

/// `child` behind the Settings sheet, which hangs below the toolbar: inert and dimmed while the
/// sheet is open. The toolbar stays clear, as its Workspace control is the sheet's toggle.
fn under_sheet<V: WidgetView<Writer>>(
    state: &Writer,
    child: V,
) -> impl WidgetView<Writer> + use<V> {
    let open = state.settings_sheet();
    inert(
        open,
        cover(
            open.then(|| {
                (
                    state.palette.backdrop,
                    screen(Texture::default(), state.palette.screen),
                )
            }),
            child,
        ),
    )
}
