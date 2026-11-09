//! Beat identity and reading actions adapt without replacing the manuscript below.
use super::*;
use crate::WriterLayout;
use xilem::masonry::properties::LineBreaking;

pub(super) fn heading(state: &Writer, id: &str) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let compact = state.compact_chrome();
    let wide = state.script_width() >= 240. * f64::from(scale);
    let blocked = state.phase() != Phase::Writing || state.navigation.busy() || state.work.busy();
    let replies = if state.reading.replies == 1 {
        "1 reply".to_owned()
    } else {
        format!("{} replies", state.reading.replies)
    };
    flex_col((
        flex_row((
            print::title(scale, display_name(id))
                .text_size(if compact { 24. } else { 32. } * scale)
                .prop(LineBreaking::WordWrap)
                .flex(if wide { 1. } else { 0. }),
            print::action(scale, "Pin reference", blocked, Writer::pin_reference),
        ))
        .direction(if wide {
            Axis::Horizontal
        } else {
            Axis::Vertical
        })
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(8.px())
        .width(state.script_width().px()),
        flex_row((
            print::meta(
                scale,
                if workspace::Layout::resolve(state).compact_split {
                    "Script shown · widen the window for Split".to_owned()
                } else if compact {
                    format!(
                        "{}{}",
                        if state.reading.is_default {
                            "Start · "
                        } else {
                            ""
                        },
                        replies
                    )
                } else {
                    format!(
                        "{}  ::  {id}    {}{}",
                        state.session.model().document().key(),
                        if state.reading.is_default {
                            "default  "
                        } else {
                            ""
                        },
                        replies
                    )
                },
            )
            .prop(LineBreaking::WordWrap)
            .color(state.palette.muted)
            .flex(1.),
            workspace::Layout::resolve(state).compact_split.then(|| {
                print::action(scale, "Open map", blocked, |state: &mut Writer| {
                    state.show_workspace(crate::map::Workspace::Map)
                })
            }),
        ))
        .direction(if wide {
            Axis::Horizontal
        } else {
            Axis::Vertical
        })
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(8.px())
        .width(state.script_width().px()),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(8.px())
}
