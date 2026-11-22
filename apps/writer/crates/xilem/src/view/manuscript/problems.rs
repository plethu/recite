//! The first problems under the manuscript. A prose finding offers its replacements and, for a
//! spelling, adding the word to the project dictionary, as the language server's quick fixes do.
use super::*;
use recite_writer_app::prose::{Problem, ProseProblem};
use recite_writer_session::messages::{MsgId, UiArg, format};
use recite_writer_session::prose::ProseFindingKind;

/// Problems listed in full; the status line counts the rest and reveals the first.
const LISTED: usize = 3;
/// Replacements offered for one finding, in the checker's order.
const REPLACEMENTS: usize = 3;

pub(super) fn problems(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let problems = state.problems();
    flex_col(
        problems
            .iter()
            .take(LISTED)
            .map(|problem| row(state, &problem))
            .collect::<Vec<_>>(),
    )
    .gap(space::SIBLINGS.px())
}

/// A prose finding reads as plain wording, not the checker's message and rule name.
fn wording(problem: &Problem<'_>) -> String {
    let line = problem.diagnostic.span.start.line();
    match &problem.prose {
        None => format!("Line {line} · {}", problem.diagnostic.message),
        Some(prose) if prose.finding.kind == ProseFindingKind::Spelling => format(
            MsgId::WriterProseSpelling,
            [
                ("line", UiArg::from(line)),
                ("text", UiArg::from(prose.finding.text.as_str())),
            ],
        ),
        Some(prose) => format(
            MsgId::WriterProseNote,
            [
                ("line", UiArg::from(line)),
                ("text", UiArg::from(prose.finding.text.as_str())),
                ("detail", UiArg::from(prose.finding.message.as_str())),
            ],
        ),
    }
}

fn row(state: &Writer, problem: &Problem<'_>) -> impl WidgetView<Writer> + use<> {
    let line = paragraph(state.ui_scale(), wording(problem), state.script_width());
    match &problem.prose {
        None => Either::A(line),
        Some(prose) => Either::B(
            flex_col((line, fixes(state, prose)))
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .gap(space::SIBLINGS.px()),
        ),
    }
}

fn fixes(state: &Writer, prose: &ProseProblem<'_>) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let writing = state.phase() == Phase::Writing && !state.session.read_only();
    let editing = writing && !state.navigation.busy() && !state.work.busy();
    let (generation, index) = (prose.generation, prose.index);
    let replacements = prose
        .finding
        .fixes
        .iter()
        .take(REPLACEMENTS)
        .enumerate()
        .map(|(fix, edit)| {
            text_button(
                scale,
                format(
                    MsgId::WriterProseReplace,
                    [("replacement", edit.replacement.as_str())],
                ),
                move |state: &mut Writer| state.apply_prose_fix(generation, index, fix),
            )
            .disabled(!editing)
        })
        .collect::<Vec<_>>();
    let add = prose.can_add_word.then(|| {
        text_button(
            scale,
            format(
                MsgId::WriterProseAddWord,
                [("word", prose.finding.text.as_str())],
            ),
            move |state: &mut Writer| state.add_prose_word(generation, index),
        )
        .disabled(!writing)
    });
    flex_row((replacements, add))
        .direction(action_axis(state))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(space::SIBLINGS.px())
}
