//! Composed trial settings; only one bounded page of values and languages is mounted.
use super::*;
use crate::app::keyboard::ModalSurface;
use crate::presentation::control_style::{ControlStyle, styled};
use crate::presentation::fit_content;
use crate::presentation::group::dialog;
use crate::presentation::slip::OFFSET;
use crate::presentation::space;
use crate::presentation::tokens::size;
use crate::presentation::{self, Icon};
use crate::presentation::{Type, Typeset};
use recite_writer_session::messages::{MsgId, text};
use xilem::{masonry::properties, view::checkbox};

mod language;
mod values;

/// The Configure preview slip: the next run's inputs on floating paper, anchored under the
/// preview's header and over its transcript, which stays in place beneath. The language query takes
/// focus as it opens, Tab stays inside, and Esc or its close returns to the preview.
pub(super) fn slip(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let width = (size::SLIP_WIDTH * f64::from(scale)).min(state.content_width() - OFFSET);
    let inner = width - 2. * space::KINDS - 2.;
    let disabled = state.phase() != Phase::Writing || state.session.read_only();
    // Room for the slip's stock offset inside the transcript's area.
    // The slip hugs its fields up to the transcript's height, and scrolls inside past that.
    sized_box(presentation::slip::slip(
        &state.palette,
        fit_content(
            sized_box(dialog(
                "Configure preview",
                flex_col((
                    flex_row((
                        label(scale, "Configure preview")
                            .typeset(Type::Section, scale)
                            .flex(1.),
                        print::icon_action(
                            scale,
                            "Done with preview inputs",
                            Icon::Close,
                            &state.palette,
                            false,
                            Writer::close_trial_inputs,
                        ),
                    ))
                    .cross_axis_alignment(CrossAxisAlignment::Center)
                    .gap(space::SIBLINGS.px()),
                    pane("Preview inputs", editor(state, disabled, inner))
                        .constrain_horizontal(true)
                        .flex(1.),
                ))
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .gap(space::TEXT.px()),
            ))
            .width(width.px())
            .padding(space::KINDS.px())
            .prop(ModalSurface(true)),
        ),
    ))
    .padding(Padding {
        right: OFFSET.px(),
        bottom: OFFSET.px(),
        ..Padding::all(0.px())
    })
}

fn editor(state: &Writer, disabled: bool, width: f64) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    // One line: what a change here applies to.
    let help = paragraph(
        scale,
        if matches!(state.trial.settings.review, Review::Diagnostic) {
            "Values were valid when checked. Changes apply to the next run."
        } else {
            "Changes apply to the next run; this one keeps its inputs."
        },
        width,
    );
    let drafts = styled(
        ControlStyle::Flush,
        checkbox(
            text(MsgId::WriterIncludeDrafts),
            state.trial.settings.include_drafts,
            |state: &mut Writer, value| state.trial.settings.include_drafts = value,
        )
        .text_size(Type::Body.size(scale))
        .disabled(disabled),
    );
    // Groups sit one group gap apart; a label sits one unit above its field.
    flex_col((
        help,
        language::selector(state, disabled, width),
        flex_col((
            label(scale, "Preview wording (empty for default)"),
            wording(state, disabled, width),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(space::UNIT.px()),
        drafts,
        // Without interpolations there is no values group, nor its gap.
        (!state
            .session
            .model()
            .document()
            .preview_bindings()
            .is_empty())
        .then(|| values::values(state, disabled, width)),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::GROUPS.px())
}

fn wording(state: &Writer, disabled: bool, width: f64) -> impl WidgetView<Writer> + use<> {
    sized_box(
        text_input(
            state.trial.settings.variant.clone(),
            |state: &mut Writer, value| {
                state.trial.settings.variant = value;
            },
        )
        .typeset(Type::Body, state.ui_scale())
        .disabled(disabled)
        .prop(properties::TextAreaLabel::new("Preview wording")),
    )
    .width(width.px())
}

#[cfg(test)]
mod tests;
