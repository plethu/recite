//! Focus mode's header: one quiet way back out at the top right.
use super::*;
use crate::presentation::Type;
use crate::presentation::control_style::{ControlStyle, styled};
use crate::presentation::row::named_row;
use crate::presentation::space;
use xilem::view::FlexSpacer;

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
