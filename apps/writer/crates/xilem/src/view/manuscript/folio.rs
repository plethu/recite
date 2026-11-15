//! The manuscript's place is projected at an accepted reading boundary.
use super::*;
use crate::presentation::print;
use crate::presentation::running::running_head;
use crate::presentation::space;
use crate::view::workspace::Layout;
use xilem::masonry::parley::style::FontWeight;
use xilem::{
    masonry::properties::{ContentColor, LineBreaking},
    style::Padding,
    view::divider_h,
};

/// The running foot: the file and beat ids in mono and the beat's place in the scene, held at
/// the sheet's foot while the manuscript scrolls.
pub(in crate::view) fn footer(state: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let beat = state.reading.selected.as_ref()?;
    let position = state.reading.position.as_ref()?;
    let grid = Layout::resolve(state).grid;
    Some(
        flex_col((
            divider_h()
                .thickness(1.px())
                .prop(ContentColor::new(state.palette.rule)),
            flex_row((
                print::id(
                    state.ui_scale(),
                    format!("{} · {beat}", state.session.model().document().key()),
                )
                .prop(LineBreaking::Clip)
                .color(state.palette.muted)
                .flex(1.),
                print::meta(
                    state.ui_scale(),
                    format!("Beat {} of {}", position.ordinal(), position.total()),
                )
                .color(state.palette.muted),
            ))
            .gap(space::SIBLINGS.px()),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .gap(space::SIBLINGS.px())
        .padding(Padding {
            top: 0.px(),
            bottom: space::TEXT.px(),
            left: grid.pad.px(),
            right: grid.pad.px(),
        })
        .background_color(state.palette.surface),
    )
}

/// The running head: the scene and beat, shown over the top of the sheet once the scene title
/// has scrolled away.
pub(in crate::view) fn head(state: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let beat = state.reading.selected.as_ref()?;
    let key = state.session.model().document().key().as_str();
    let scene = recite_writer_session::choices::display_name(key.rsplit('/').next().unwrap_or(key));
    let grid = Layout::resolve(state).grid;
    Some(running_head(
        flex_col((
            print::meta(
                state.ui_scale(),
                format!(
                    "{scene} › {}",
                    recite_writer_session::choices::display_name(beat)
                ),
            )
            .weight(FontWeight::SEMI_BOLD)
            .color(state.palette.muted)
            .prop(LineBreaking::Clip),
            divider_h()
                .thickness(1.px())
                .prop(ContentColor::new(state.palette.rule)),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .gap(space::SIBLINGS.px())
        .padding(Padding {
            top: space::SIBLINGS.px(),
            bottom: 0.px(),
            left: (grid.pad + grid.text()).px(),
            right: grid.pad.px(),
        })
        .background_color(state.palette.surface),
    ))
}
