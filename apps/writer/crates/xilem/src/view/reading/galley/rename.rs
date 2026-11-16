//! The selected beat renamed in place: its heading becomes a field, and once the project rename
//! is reviewed a slip under the heading says what it changes, with Rename and Cancel.
use super::*;
use crate::app::keyboard::DefaultFocus;
use crate::presentation::slip::slip;
use crate::presentation::{Type, Typeset, material, paragraph, text_button};
pub(super) use recite_writer_app::workspaces::rename::inline::{Inline, Stage};
use xilem::masonry::parley::style::FontWeight;
use xilem::masonry::properties::TextAreaLabel;
use xilem::view::button;
use xilem::view::text_input;

/// The heading's field, holding the new id; Enter reviews the rename and Esc cancels it.
pub(super) fn field(state: &Writer, inline: &Inline) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    styled(
        ControlStyle::Field,
        text_input(inline.name.clone(), Writer::edit_inline_rename)
            .on_enter(|state: &mut Writer, _| state.commit_inline_rename())
            .typeset(Type::Section, scale)
            .prop(TextAreaLabel::new("New beat name"))
            .prop(DefaultFocus(inline.stage == Stage::Editing)),
    )
}

/// Under the heading: the review's progress, then its consequence slip.
pub(super) fn consequence(state: &Writer, grid: &Grid) -> Option<impl WidgetView<Writer> + use<>> {
    let inline = state.inline_rename()?;
    let scale = state.ui_scale();
    let palette = &state.palette;
    let measure = grid.row() - grid.text();
    let body = match inline.stage {
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
            let width = measure.min(size::SLIP_WIDTH * f64::from(scale));
            Either::B(slip(
                palette,
                sized_box(
                    flex_col((
                        paragraph(scale, sentence, width - 2. * space::KINDS),
                        flex_row((
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
                            text_button(scale, "Review all changes", Writer::review_inline_rename),
                            styled(
                                ControlStyle::Action,
                                button(
                                    Type::Control.label(scale, "Cancel"),
                                    Writer::cancel_inline_rename,
                                )
                                .accessible_label("Cancel rename"),
                            ),
                        ))
                        .gap(space::SIBLINGS.px()),
                    ))
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .gap(space::TEXT.px()),
                )
                .width(width.px())
                .padding(space::KINDS.px()),
            ))
        }
    };
    Some(sized_box(body).padding(Padding {
        left: grid.text().px(),
        top: space::TEXT.px(),
        ..Padding::all(0.px())
    }))
}
