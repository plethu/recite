//! The manuscript's place is projected at an accepted reading boundary.
use super::*;
use crate::presentation::print;
use xilem::{
    masonry::{
        kurbo::Axis,
        properties::{ContentColor, LineBreaking},
    },
    style::Padding,
    view::{FlexSpacer, divider_h},
};

pub(super) fn footer(state: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let beat = state.reading.selected.as_ref()?;
    let position = state.reading.position.as_ref()?;
    Some(
        flex_col((
            divider_h()
                .thickness(1.5.px())
                .prop(ContentColor::new(state.palette.accent)),
            flex_row((
                print::meta(
                    state.ui_scale(),
                    format!("{} · {beat}", state.session.model().document().key()),
                )
                .prop(LineBreaking::WordWrap)
                .color(state.palette.muted)
                .flex(1.),
                FlexSpacer::Flex(1.),
                print::meta(
                    state.ui_scale(),
                    format!("beat {} / {}", position.ordinal(), position.total()),
                )
                .color(state.palette.muted),
            ))
            .direction(
                if state.script_width() < 560. * f64::from(state.ui_scale()) {
                    Axis::Vertical
                } else {
                    Axis::Horizontal
                },
            )
            .gap(8.px()),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .gap(8.px())
        .padding(Padding::top(24.px()))
        .width(state.script_width().px()),
    )
}
