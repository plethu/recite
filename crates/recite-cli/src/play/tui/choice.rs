use recite_core::ChoiceId;
use recite_runtime::PreviewPrompt;

use crate::error::CliError;
use crate::i18n::{Messages, MsgId};
use crate::play::choice_selection::ChoiceSelection;

pub(super) fn resolve_choice(
    messages: &Messages,
    selection: ChoiceSelection,
    prompt: &PreviewPrompt,
) -> Result<ChoiceId, CliError> {
    let choice = match selection {
        ChoiceSelection::Index(index) => {
            let numeric_id = index.to_string();
            if let Some(choice) = prompt
                .choices()
                .iter()
                .find(|choice| choice.id.as_str() == numeric_id)
            {
                choice
            } else if index == 0 || index > prompt.choices().len() {
                return Err(CliError::PlayInvalidInput(messages.format(
                    MsgId::PlayErrorChoiceIndexOutOfRange,
                    recite_ui::UiArgs::from([
                        ("index".to_owned(), recite_ui::UiArg::from(index)),
                        (
                            "count".to_owned(),
                            recite_ui::UiArg::from(prompt.choices().len()),
                        ),
                    ]),
                )));
            } else {
                &prompt.choices()[index - 1]
            }
        }
        ChoiceSelection::Id(id) => {
            let choice_id = ChoiceId::new(id.clone()).map_err(|error| {
                CliError::PlayInvalidInput(messages.format(
                    MsgId::PlayErrorChoiceIdInvalid,
                    [("id", id.clone()), ("error", error.to_string())],
                ))
            })?;
            prompt
                .choices()
                .iter()
                .find(|choice| choice.id == choice_id)
                .ok_or_else(|| {
                    CliError::PlayInvalidInput(
                        messages.format(MsgId::PlayErrorChoiceIdUnavailable, [("id", id)]),
                    )
                })?
        }
    };
    if choice.availability.is_available {
        return Ok(choice.id.clone());
    }

    let message = choice
        .availability
        .primary_reason
        .as_ref()
        .filter(|reason| !reason.source_text.is_empty())
        .map_or_else(
            || {
                messages.format(
                    MsgId::PlayErrorChoiceUnavailable,
                    [("id", choice.id.as_str().to_owned())],
                )
            },
            |reason| {
                messages.format(
                    MsgId::PlayErrorChoiceUnavailableReason,
                    [
                        ("id", choice.id.as_str().to_owned()),
                        ("reason", reason.source_text.clone()),
                    ],
                )
            },
        );
    Err(CliError::PlayInvalidInput(message))
}
