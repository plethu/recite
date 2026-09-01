use std::collections::BTreeMap;

use recite_core::{BlockId, ChoiceId, CompiledDialogue, CompiledStatementKind, StatementRange};
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
    block: String,
    pub(super) fixture_keys: Vec<String>,
}

#[derive(Debug)]
pub(super) struct PromptCardinality {
    by_block: BTreeMap<String, usize>,
}

impl PromptCardinality {
    pub(super) fn new(asset: &CompiledDialogue) -> Result<Self, CliError> {
        let mut by_block = BTreeMap::new();
        for block in &asset.blocks {
            by_block.insert(
                block.id.as_str().to_owned(),
                count_prompts_in_range(asset, block.statements)?,
            );
        }
        Ok(Self { by_block })
    }

    pub(super) fn for_block(&self, block: &BlockId) -> usize {
        self.by_block.get(block.as_str()).copied().unwrap_or(0)
    }
}

impl PromptIdentity {
    pub(super) fn from_preview(prompt: &PreviewPrompt, block_prompt_count: usize) -> Self {
        let identity = prompt.identity();
        let mut fixture_keys = Vec::new();
        if let Some(line) = identity.line() {
            fixture_keys.push(line.as_str().to_owned());
        }
        // A block key is retained as a compatibility fallback. The driver verifies that the
        // active block has exactly one prompt before accepting it.
        if block_prompt_count == 1 {
            fixture_keys.push(identity.block().as_str().to_owned());
        }
        Self {
            has_line: identity.line().is_some(),
            block: identity.block().as_str().to_owned(),
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
            fixture
                .choices
                .get(&prompt.block)
                .ok_or_else(|| CliError::MissingFixtureChoice {
                    prompt_keys: prompt.fixture_keys.clone(),
                })?
        }
        None => {
            if fixture.choices.contains_key(&prompt.block) {
                return Err(CliError::AmbiguousFixtureChoice {
                    block: prompt.block.clone(),
                    prompt_count: block_prompt_count,
                });
            }
            return Err(CliError::MissingFixtureChoice {
                prompt_keys: prompt.fixture_keys.clone(),
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
