//! A beat renamed in place, wherever its name shows: the field that takes the name's place, and
//! under it the review's progress, then the consequence slip with Rename, Review all changes and
//! Cancel. The heading, the sidebar row and the map card share them.
use super::Writer;
use crate::app::keyboard::DefaultFocus;
use crate::presentation::control_style::{ControlStyle, styled};
use crate::presentation::slip::slip;
use crate::presentation::tokens::{size, space};
use crate::presentation::{Type, Typeset, material, paragraph, print};
pub(crate) use recite_writer_app::workspaces::rename::inline::{Inline, Place, Stage, Target};
use xilem::WidgetView;
use xilem::core::one_of::Either;
use xilem::masonry::layout::AsUnit;
use xilem::masonry::parley::style::FontWeight;
use xilem::masonry::properties::{LineBreaking, TextAreaLabel};
use xilem::style::Style;
use xilem::view::{CrossAxisAlignment, button, flex_col, flex_row, sized_box, text_input};

/// How the slip sets its actions: in one row under a heading or card, stacked in the narrow
/// sidebar.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Actions {
    Row,
    Stacked,
}

/// The rename of the beat `beat` at `place`, while its name is a field.
pub(crate) fn editing<'a>(state: &'a Writer, beat: &str, place: Place) -> Option<&'a Inline> {
    state.inline_rename().filter(|inline| {
        inline.place == place
            && inline.beat == beat
            && matches!(inline.stage, Stage::Editing | Stage::Reviewing)
    })
}

/// The field holding the new id, set in `role` at `scale`; Enter reviews the rename and Esc
/// cancels it.
pub(crate) fn field(inline: &Inline, role: Type, scale: f32) -> impl WidgetView<Writer> + use<> {
    styled(
        ControlStyle::Field,
        text_input(inline.name.clone(), Writer::edit_inline_rename)
            .on_enter(|state: &mut Writer, _| state.commit_inline_rename())
            .typeset(role, scale)
            .prop(TextAreaLabel::new("New beat name"))
            .prop(DefaultFocus(inline.stage == Stage::Editing)),
    )
}

/// Under the field at `place`: the review's progress, then its consequence slip, `width` wide.
pub(crate) fn consequence(
    state: &Writer,
    place: Place,
    width: f64,
    actions: Actions,
) -> Option<impl WidgetView<Writer> + use<>> {
    let inline = state
        .inline_rename()
        .filter(|inline| inline.place == place)?;
    let scale = state.ui_scale();
    let palette = &state.palette;
    Some(match inline.stage {
        Stage::Editing => return None,
        Stage::Reviewing | Stage::Applying => Either::A(
            print::meta(
                scale,
                if inline.stage == Stage::Reviewing {
                    "Checking every reference…"
                } else {
                    "Renaming…"
                },
            )
            .color(palette.muted),
        ),
        Stage::Reviewed => {
            let sentence = state
                .rename_consequence()
                .map_or_else(String::new, |consequence| {
                    consequence.sentence(&inline.beat, inline.name.trim())
                });
            let buttons = (
                // The slip's one Primary, focused as the slip opens.
                material::focus_ring(
                    Some(palette.on_accent),
                    styled(
                        ControlStyle::Primary,
                        button(
                            Type::Control
                                .label(scale, "Rename")
                                .weight(FontWeight::SEMI_BOLD)
                                .color(palette.on_accent),
                            Writer::confirm_inline_rename,
                        )
                        .prop(DefaultFocus(true)),
                    ),
                ),
                // Its words wrap rather than overflow a slip as narrow as the sidebar.
                button(
                    Type::Control
                        .label(scale, "Review all changes")
                        .prop(LineBreaking::WordWrap),
                    Writer::review_inline_rename,
                ),
                styled(
                    ControlStyle::Action,
                    button(
                        Type::Control.label(scale, "Cancel"),
                        Writer::cancel_inline_rename,
                    )
                    .accessible_label("Cancel rename"),
                ),
            );
            let buttons = match actions {
                Actions::Row => Either::A(flex_row(buttons).gap(space::SIBLINGS.px())),
                Actions::Stacked => Either::B(
                    flex_col(buttons)
                        .cross_axis_alignment(CrossAxisAlignment::Start)
                        .gap(space::UNIT.px()),
                ),
            };
            Either::B(slip(
                palette,
                sized_box(
                    flex_col((
                        paragraph(scale, sentence, width - 2. * space::KINDS),
                        buttons,
                    ))
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .gap(space::TEXT.px()),
                )
                .width(width.px())
                .padding(space::KINDS.px()),
            ))
        }
    })
}

/// The widest a consequence slip grows, at `scale`.
pub(crate) fn slip_width(scale: f32) -> f64 {
    size::SLIP_WIDTH * f64::from(scale)
}
