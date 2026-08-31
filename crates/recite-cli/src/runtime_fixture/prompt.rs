use recite_core::{ChoiceId, CompiledDialogue, CompiledStatementKind, StatementRange};
use recite_runtime::PreviewPrompt;
use recite_ui::UiArg;

use super::fixture::{FixtureChoice, RuntimeFixture};
use crate::error::CliError;
use crate::i18n::{Messages, MsgId};

/// The CLI-only identity used to preserve fixture key compatibility. Runtime identity remains
/// authoritative; this projection never scans the compiled asset.
#[derive(Clone, Debug)]
pub(super) struct PromptIdentity {
    has_line: bool,
    pub(super) fixture_keys: Vec<String>,
}

impl PromptIdentity {
    pub(super) fn from_preview(prompt: &PreviewPrompt) -> Self {
        let identity = prompt.identity();
        let mut fixture_keys = Vec::new();
        if let Some(line) = identity.line() {
            fixture_keys.push(line.as_str().to_owned());
        }
        // A block key is retained as a compatibility fallback. The driver verifies that the
        // active block has exactly one prompt before accepting it.
        fixture_keys.push(identity.block().as_str().to_owned());
        Self {
            has_line: identity.line().is_some(),
            fixture_keys,
        }
    }
}

pub(super) fn select_fixture_choice(
    fixture: &RuntimeFixture,
    prompt: &PromptIdentity,
    choices: &[recite_runtime::DialogueChoice],
    block_prompt_count: usize,
) -> Result<ChoiceId, CliError> {
    let selection = prompt
        .has_line
        .then(|| prompt.fixture_keys.first())
        .flatten()
        .and_then(|key| fixture.choices.get(key));
    let selection = match selection {
        Some(selection) => selection,
        None if block_prompt_count == 1 => {
            let Some(block_key) = prompt.fixture_keys.last() else {
                return Err(CliError::MalformedCompiledAsset {
                    reason: "preview prompt has no block identity".to_owned(),
                });
            };
            fixture
                .choices
                .get(block_key)
                .ok_or_else(|| CliError::MissingFixtureChoice {
                    prompt_keys: prompt.fixture_keys.clone(),
                })?
        }
        None if prompt.has_line => {
            return Err(CliError::MalformedCompiledAsset {
                reason: format!(
                    "fixture block choice key `{}` is ambiguous because the block contains multiple prompts; use a line ID",
                    prompt.fixture_keys.last().cloned().unwrap_or_default()
                ),
            });
        }
        None => {
            return Err(CliError::MalformedCompiledAsset {
                reason: format!(
                    "fixture block choice key `{}` is ambiguous because the block contains multiple prompts; use a line ID",
                    prompt.fixture_keys.first().cloned().unwrap_or_default()
                ),
            });
        }
    };

    match selection {
        FixtureChoice::Id(choice_id) => {
            let choice = ChoiceId::new(choice_id.clone())?;
            if !choices.iter().any(|candidate| candidate.id == choice) {
                return Err(CliError::FixtureChoiceNotInPrompt {
                    choice: choice_id.clone(),
                    prompt_keys: prompt.fixture_keys.clone(),
                });
            }
            Ok(choice)
        }
        FixtureChoice::Index(index) => {
            if *index == 0 || *index > choices.len() {
                return Err(CliError::FixtureChoiceIndexOutOfRange {
                    index: *index,
                    choice_count: choices.len(),
                    prompt_keys: prompt.fixture_keys.clone(),
                });
            }
            Ok(choices[*index - 1].id.clone())
        }
    }
}

pub(super) fn write_prompt_run_lines(
    run_lines: &mut Vec<String>,
    line: Option<&recite_runtime::DialogueLine>,
    choices: &[recite_runtime::DialogueChoice],
    messages: &Messages,
) {
    match line {
        Some(line) => run_lines.push(messages.format(
            MsgId::PlayPromptLine,
            [
                ("id", UiArg::from(line.id.as_str())),
                ("text", UiArg::from(line.text.as_str())),
            ],
        )),
        None => run_lines.push(messages.text(MsgId::PlayPrompt)),
    }
    for (index, choice) in choices.iter().enumerate() {
        run_lines.push(messages.format(
            MsgId::PlayChoiceRow,
            [
                ("index", UiArg::from(index + 1)),
                ("id", UiArg::from(choice.id.as_str())),
                ("text", UiArg::from(choice.text.as_str())),
                ("available", UiArg::from(choice.availability.is_available)),
            ],
        ));
    }
}

pub(super) fn count_prompts_in_block(
    asset: &CompiledDialogue,
    block: &str,
) -> Result<usize, CliError> {
    let block = asset
        .blocks
        .iter()
        .find(|candidate| candidate.id.as_str() == block)
        .ok_or_else(|| {
            CliError::Runtime(recite_runtime::DialogueError::UnknownBlock {
                block: block.to_owned(),
            })
        })?;
    count_prompts_in_range(asset, block.statements)
}

fn count_prompts_in_range(
    asset: &CompiledDialogue,
    range: StatementRange,
) -> Result<usize, CliError> {
    let start = range.start.as_u32();
    let end = start
        .checked_add(range.len)
        .ok_or_else(|| CliError::MalformedCompiledAsset {
            reason: "statement range overflows".to_owned(),
        })?;
    let mut count = 0;
    for index in start..end {
        let statement = asset.statements.get(index as usize).ok_or_else(|| {
            CliError::MalformedCompiledAsset {
                reason: format!("statement index {index} is out of bounds"),
            }
        })?;
        count += match &statement.kind {
            CompiledStatementKind::Prompt { .. } => 1,
            CompiledStatementKind::If {
                then_statements,
                else_statements,
                ..
            } => {
                count_prompts_in_range(asset, *then_statements)?
                    + count_prompts_in_range(asset, *else_statements)?
            }
            CompiledStatementKind::Match { arms, .. } => {
                let arm_start = arms.start.as_u32();
                let arm_end = arm_start.checked_add(arms.len).ok_or_else(|| {
                    CliError::MalformedCompiledAsset {
                        reason: "match arm range overflows".to_owned(),
                    }
                })?;
                let mut nested = 0;
                for arm_index in arm_start..arm_end {
                    let arm = asset.match_arms.get(arm_index as usize).ok_or_else(|| {
                        CliError::MalformedCompiledAsset {
                            reason: format!("match arm index {arm_index} is out of bounds"),
                        }
                    })?;
                    nested += count_prompts_in_range(asset, arm.statements)?;
                }
                nested
            }
            CompiledStatementKind::Line(_)
            | CompiledStatementKind::Divert(_)
            | CompiledStatementKind::Effect(_)
            | CompiledStatementKind::End => 0,
        };
    }
    Ok(count)
}
