//! Readable beat cards compose a native selection button and a contextual move grip.
use super::*;
use crate::presentation::marks::{GLYPH, Mark, THICKNESS, marked, wedge_glyph};
use crate::presentation::space;
use crate::presentation::{Icon, SMALL, excerpt, mark, print};
use recite_writer_app::map::actions::Press;
use recite_writer_session::typography::FONT_UI;
use xilem::core::one_of::Either;
use xilem::masonry::{
    parley::style::FontWeight,
    properties::{self, BoxShadow},
};
use xilem::view::PointerButton;
use xilem::{peniko::Color, style::Padding};

pub(super) fn card(state: &Writer, scene: &Scene, index: usize) -> impl WidgetView<Writer> + use<> {
    let block = &scene.projection.graph.blocks[index];
    let synopsis = &scene.projection.synopsis[index];
    let id = block.id.clone();
    let mut title = recite_writer_session::choices::display_name(&block.id);
    if scene.projection.graph.endings.contains(&block.id) {
        title.push_str(" · end");
    }
    let selected = scene.selected.as_deref() == Some(&block.id);
    // The galley's margin marks repeat here: the reading beat's bar, and a wedge on a beat with a
    // problem, which the card's name also says.
    let reading = state.reading.selected.as_deref() == Some(block.id.as_str());
    let problem = state
        .beat_problems()
        .iter()
        .any(|(beat, _)| beat == &block.id);
    let name = if problem {
        format!("{title} · Problem")
    } else {
        title.clone()
    };
    let disabled = state.phase() != crate::Phase::Writing
        || !scene
            .projection
            .is_current(state.session.model().document());
    let zoom = scene.camera.zoom;
    let width = f64::from(scene.node(index).width);
    let contents = flex_col((
        {
            let heading = label(1., title.clone())
                .font(print::font(FONT_UI))
                .weight(FontWeight::SEMI_BOLD)
                .text_size(16_f32.max(14. / zoom));
            if problem {
                Either::A(
                    flex_row((heading, wedge_glyph(GLYPH, state.palette.error)))
                        .cross_axis_alignment(CrossAxisAlignment::Center)
                        .gap(space::UNIT.px()),
                )
            } else {
                Either::B(heading)
            }
        },
        flex_col((
            label(1., synopsis.speaker.clone())
                .text_size(13_f32.max(11. / zoom))
                .color(state.palette.muted),
            excerpt(synopsis.prose.clone(), zoom).width((width - 24.).px()),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(space::UNIT.px()),
        (!synopsis.annotations.is_empty()).then(|| {
            label(1., synopsis.annotations.join(" · "))
                .text_size(13_f32.max(11. / zoom))
                .color(state.palette.muted)
        }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::SIBLINGS.px())
    .dims(((width - 24.).px(), (HEIGHT - 24.).px()));
    let focus_id = id.clone();
    let move_id = id.clone();
    let body = button_any_pointer(contents, move |state: &mut Writer, pointer| {
        let press = match pointer {
            None => Press::Keyboard,
            Some(PointerButton::Primary) => Press::Primary,
            Some(_) => Press::Secondary,
        };
        state.press_map_beat(id.clone(), press);
    })
    .disabled(disabled)
    .prop(properties::Dimensions::fixed(
        scene.node(index).width.px(),
        HEIGHT.px(),
    ))
    .padding(Padding::all(space::TEXT.px()))
    .background_color(Color::TRANSPARENT)
    .border_color(Color::TRANSPARENT)
    .border_width(0.px());
    let focus = input::input(
        input::Kind::Card {
            title: name,
            selected,
        },
        false,
        body,
        move |state: &mut Writer, action| state.map_input(Source::Card(focus_id.clone()), action),
    )
    .disabled(disabled)
    .focus(
        scene
            .focus
            .as_ref()
            .filter(|focus| focus.beat == block.id)
            .and_then(|_| canvas::focus_request(scene)),
    )
    .background_color(if selected {
        state
            .palette
            .surface
            .lerp_rect(state.palette.selection, 0.35)
    } else {
        state.palette.surface
    })
    .border_color(if selected {
        state.palette.accent
    } else {
        state.palette.rule
    })
    .border_width(0.px())
    .corner_radius(4.px())
    .box_shadow(BoxShadow::new(
        if selected {
            state.palette.accent
        } else {
            state.palette.teal
        },
        (4., 4.),
    ))
    .prop(properties::Dimensions::fixed(
        scene.node(index).width.px(),
        HEIGHT.px(),
    ))
    // Selection is the plum offset and border; no dot screen sits behind the title.
    .prop(input::CardFocus(state.palette.accent));
    let grip = (selected || scene.hovered.as_deref() == Some(&block.id)).then(|| {
        placed::placed(
            Affine::translate((width - 30., 6.)),
            input::input(
                input::Kind::Grip(title),
                state.config.ui.keymap == recite_config::Keymap::Vim,
                // A painted grip; the image keeps the glyph's name inside the named control.
                mark(SMALL, "⠿", Icon::Grip, state.palette.muted),
                move |state: &mut Writer, action| {
                    state.map_input(Source::Grip(move_id.clone()), action)
                },
            )
            .disabled(
                state.phase() != crate::Phase::Writing
                    || state.navigation.busy()
                    || state.work.busy(),
            )
            .prop(properties::Dimensions::fixed(24.px(), 24.px()))
            .prop(properties::BorderColor {
                color: state.palette.accent,
            }),
        )
    });
    // Only the reading beat's card carries a mark, so the others mount nothing for it.
    let mark = reading.then(|| {
        marked(
            &state.palette,
            Some(if state.session.model().has_draft() {
                Mark::Unsaved
            } else {
                Mark::Current
            }),
            flex_col(()).dims((THICKNESS.px(), HEIGHT.px())),
        )
    });
    zstack((focus, grip, mark))
        .alignment(UnitPoint::TOP_LEFT)
        .dims((scene.node(index).width.px(), HEIGHT.px()))
}
