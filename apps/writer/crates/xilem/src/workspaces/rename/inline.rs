//! A beat renamed in place, wherever its name shows: the field that takes the name's place, and
//! under it the review's progress, then the consequence slip with Rename, Review all changes and
//! Cancel. The heading, the sidebar row and the map card share them.
use super::Writer;
use crate::app::keyboard::{DefaultFocus, ModalSurface};
use crate::presentation::control_style::{ControlStyle, styled};
use crate::presentation::group;
use crate::presentation::slip::slip;
use crate::presentation::tokens::{size, space};
use crate::presentation::{Type, Typeset, material, paragraph, print};
pub(crate) use recite_writer_app::workspaces::rename::inline::{Inline, Place, Stage, Target};
use xilem::WidgetView;
use xilem::core::one_of::Either;
use xilem::masonry::layout::{AsUnit, Dim};
use xilem::masonry::parley::style::FontWeight;
use xilem::masonry::properties::{LineBreaking, TextAreaLabel};
use xilem::style::{Padding, Style};
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

/// Whether the review's slip of a rename of `beat` hangs from its name at `place`.
pub(crate) fn hangs_from(state: &Writer, beat: &str, place: Place) -> bool {
    state
        .inline_rename()
        .is_some_and(|inline| inline.place == place && inline.beat == beat)
}

/// The field holding the new id, set in `role` at `scale` under its caption: a heading shows the
/// id as a title, so the caption says the field edits the id itself. Enter reviews the rename and
/// Esc cancels it.
pub(crate) fn field(
    state: &Writer,
    inline: &Inline,
    role: Type,
    scale: f32,
) -> impl WidgetView<Writer> + use<> {
    flex_col((
        caption(state),
        input(inline, (role, scale), ControlStyle::Field),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Stretch)
    .gap(space::UNIT.px())
}

/// The field on a map card, in the card's own units at `scale`: the card zooms with the map, so
/// the caption and the well's padding take the card's metrics rather than the interface's, and
/// the field never pushes the card's words past its foot.
pub(crate) fn card_field(
    state: &Writer,
    inline: &Inline,
    scale: f32,
) -> impl WidgetView<Writer> + use<> {
    flex_col((
        print::meta(1., "Beat id").color(state.palette.muted),
        styled(
            ControlStyle::Field,
            text_input(inline.name.clone(), Writer::edit_inline_rename)
                .on_enter(|state: &mut Writer, _| state.commit_inline_rename())
                .typeset(Type::Control, scale)
                .padding(Padding::from_vh(6.px(), size::CONTROL_PAD.px()))
                .prop(TextAreaLabel::new("New beat id"))
                .prop(DefaultFocus(inline.stage == Stage::Editing)),
        ),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Stretch)
    .gap(space::UNIT.px())
}

/// What the rename field edits, for a caption beside or above it.
pub(crate) fn caption(state: &Writer) -> impl WidgetView<Writer> + use<> {
    print::meta(state.ui_scale(), "Beat id").color(state.palette.muted)
}

/// The rename's text field alone, set in `role` at `scale` in `style`.
pub(crate) fn input(
    inline: &Inline,
    (role, scale): (Type, f32),
    style: ControlStyle,
) -> impl WidgetView<Writer> + use<> {
    styled(
        style,
        text_input(inline.name.clone(), Writer::edit_inline_rename)
            .on_enter(|state: &mut Writer, _| state.commit_inline_rename())
            .typeset(role, scale)
            .prop(TextAreaLabel::new("New beat id"))
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
                .padding(space::KINDS.px())
                // A decision: Tab stays on the slip until it is answered, and Esc cancels.
                .prop(ModalSurface(true)),
            ))
        }
    })
}

/// Whether a rename's review shows over the window, hanging from a measured heading or row.
pub(crate) fn floats(state: &Writer) -> bool {
    state
        .inline_rename()
        .is_some_and(|inline| inline.anchor.is_some() && inline.stage != Stage::Editing)
}

/// Whether a rename's review is a decision over the backdrop: a named dialog, with the window
/// behind it inert until it is answered.
pub(crate) fn modal(state: &Writer) -> bool {
    floats(state)
        && state
            .inline_rename()
            .is_some_and(|inline| inline.stage == Stage::Reviewed)
}

/// The review's progress or consequence slip of a rename at a heading or sidebar row, floating
/// over the window from the point the native host measured under that name, so nothing in the
/// document moves. Once the review is ready the slip is a decision over the backdrop. A card's
/// slip floats inside the map instead.
pub(crate) fn floating(state: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    if !floats(state) {
        return None;
    }
    let inline = state.inline_rename()?;
    let (x, y) = inline.anchor?;
    let scale = state.ui_scale();
    let (window, widest) = (state.viewport, slip_width(scale));
    let width = widest.min((window.width - 2. * space::GROUPS).max(0.));
    let actions = if width < widest {
        Actions::Stacked
    } else {
        Actions::Row
    };
    let slip = group::dialog_while(
        inline.stage == Stage::Reviewed,
        format!("Rename {}", inline.beat),
        consequence(state, inline.place, width, actions)?,
    );
    // A slip with a two-line sentence stays inside the window above a low name.
    let tall = 2. * space::KINDS
        + space::TEXT
        + (2. * f64::from(Type::Body.metrics().1) + size::ROW) * f64::from(scale);
    // A column keeps the slip its own height; the layer around it fills the window.
    let placed = flex_col((sized_box(slip).padding(Padding {
        left: f64::from(x)
            .min(window.width - width - space::GROUPS)
            .max(0.)
            .px(),
        top: (f64::from(y) + space::SIBLINGS)
            .min(window.height - tall)
            .max(0.)
            .px(),
        ..Padding::all(0.px())
    }),))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .dims(Dim::Stretch);
    Some(if inline.stage == Stage::Reviewed {
        Either::A(material::backdrop(
            state.palette.backdrop,
            material::screen(material::Texture::default(), state.palette.screen),
            placed,
        ))
    } else {
        Either::B(placed)
    })
}

/// The widest a consequence slip grows, at `scale`.
pub(crate) fn slip_width(scale: f32) -> f64 {
    size::SLIP_WIDTH * f64::from(scale)
}
