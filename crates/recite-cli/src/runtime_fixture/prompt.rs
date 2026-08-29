use std::collections::BTreeMap;

use recite_core::{ChoiceId, CompiledDialogue, CompiledStatementKind, StatementRange};
use recite_runtime::{DialogueChoice, DialogueLine, DialogueTrace};
use recite_ui::UiArg;

use super::fixture::{FixtureChoice, RuntimeFixture};
use super::trace::{TracePrompt, TracePromptIdentity, trace_choice, trace_line};
use crate::error::CliError;
use crate::i18n::{Messages, MsgId};

#[derive(Clone, Debug)]
pub(super) struct PromptIdentity {
    pub(super) block: String,
    pub(super) line: Option<String>,
    pub(super) choice_ids: Vec<String>,
    pub(super) fixture_keys: Vec<String>,
}

pub(super) struct PromptCatalog {
    prompts: Vec<PromptIdentity>,
}

impl PromptCatalog {
    pub(super) fn new(asset: &CompiledDialogue) -> Result<Self, CliError> {
        let mut block_prompt_counts = BTreeMap::<String, usize>::new();
        let mut prompt_rows = Vec::<(String, Option<String>, Vec<String>)>::new();

        for block in &asset.blocks {
            let block_id = block.id.as_str().to_owned();
            collect_prompts(
                asset,
                block.statements,
                &block_id,
                &mut block_prompt_counts,
                &mut prompt_rows,
            )?;
        }

        let prompts = prompt_rows
            .into_iter()
            .map(|(block, line, choice_ids)| {
                let mut fixture_keys = Vec::new();
                if let Some(line) = &line {
                    fixture_keys.push(line.clone());
                }
                if block_prompt_counts.get(&block) == Some(&1) {
                    fixture_keys.push(block.clone());
                }

                PromptIdentity {
                    block,
                    line,
                    choice_ids,
                    fixture_keys,
                }
            })
            .collect();

        Ok(Self { prompts })
    }

    pub(super) fn identify(
        &self,
        line: Option<&DialogueLine>,
        choices: &[DialogueChoice],
    ) -> Result<PromptIdentity, CliError> {
        let line_id = line.map(|line| line.id.as_str());
        let choice_ids = choices
            .iter()
            .map(|choice| choice.id.as_str())
            .collect::<Vec<_>>();

        self.prompts
            .iter()
            .find(|prompt| {
                prompt.line.as_deref() == line_id
                    && prompt
                        .choice_ids
                        .iter()
                        .map(String::as_str)
                        .eq(choice_ids.iter().copied())
            })
            .cloned()
            .ok_or_else(|| CliError::UnknownPrompt {
                line: line_id.map(str::to_owned),
                choices: choice_ids.into_iter().map(str::to_owned).collect(),
            })
    }
}

fn collect_prompts(
    asset: &CompiledDialogue,
    range: StatementRange,
    block_id: &str,
    block_prompt_counts: &mut BTreeMap<String, usize>,
    prompt_rows: &mut Vec<(String, Option<String>, Vec<String>)>,
) -> Result<(), CliError> {
    let statement_start = range.start.as_u32();
    let statement_end = statement_start + range.len;

    for statement_index in statement_start..statement_end {
        let statement = asset
            .statements
            .get(statement_index as usize)
            .ok_or_else(|| CliError::MalformedCompiledAsset {
                reason: format!("statement index {statement_index} is out of bounds"),
            })?;
        match &statement.kind {
            CompiledStatementKind::Prompt { line, choices } => {
                let line = line
                    .map(|line| {
                        asset
                            .lines
                            .get(line.as_u32() as usize)
                            .map(|line| line.id.as_str().to_owned())
                            .ok_or_else(|| CliError::MalformedCompiledAsset {
                                reason: format!("line index {} is out of bounds", line.as_u32()),
                            })
                    })
                    .transpose()?;
                let choice_start = choices.start.as_u32();
                let choice_end = choice_start + choices.len;
                let mut choice_ids = Vec::new();
                for choice_index in choice_start..choice_end {
                    let choice = asset.choices.get(choice_index as usize).ok_or_else(|| {
                        CliError::MalformedCompiledAsset {
                            reason: format!("choice index {choice_index} is out of bounds"),
                        }
                    })?;
                    choice_ids.push(choice.id.as_str().to_owned());
                }

                *block_prompt_counts.entry(block_id.to_owned()).or_default() += 1;
                prompt_rows.push((block_id.to_owned(), line, choice_ids));
            }
            CompiledStatementKind::If {
                then_statements,
                else_statements,
                ..
            } => {
                collect_prompts(
                    asset,
                    *then_statements,
                    block_id,
                    block_prompt_counts,
                    prompt_rows,
                )?;
                collect_prompts(
                    asset,
                    *else_statements,
                    block_id,
                    block_prompt_counts,
                    prompt_rows,
                )?;
            }
            CompiledStatementKind::Match { arms, .. } => {
                let arm_start = arms.start.as_u32();
                let arm_end = arm_start + arms.len;
                for arm_index in arm_start..arm_end {
                    let arm = asset.match_arms.get(arm_index as usize).ok_or_else(|| {
                        CliError::MalformedCompiledAsset {
                            reason: format!("match arm index {arm_index} is out of bounds"),
                        }
                    })?;
                    collect_prompts(
                        asset,
                        arm.statements,
                        block_id,
                        block_prompt_counts,
                        prompt_rows,
                    )?;
                }
            }
            CompiledStatementKind::Line(_)
            | CompiledStatementKind::Divert(_)
            | CompiledStatementKind::Effect(_)
            | CompiledStatementKind::End => {}
        }
    }

    Ok(())
}

pub(super) fn select_fixture_choice(
    fixture: &RuntimeFixture,
    prompt: &PromptIdentity,
    choices: &[DialogueChoice],
) -> Result<ChoiceId, CliError> {
    let selection = prompt
        .fixture_keys
        .iter()
        .find_map(|key| fixture.choices.get(key))
        .ok_or_else(|| CliError::MissingFixtureChoice {
            prompt_keys: prompt.fixture_keys.clone(),
        })?;

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
    _prompt: &PromptIdentity,
    line: Option<&DialogueLine>,
    choices: &[DialogueChoice],
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

pub(super) fn trace_prompt(
    prompt: &PromptIdentity,
    line: Option<&DialogueLine>,
    choices: &[DialogueChoice],
    dialogue_trace: &DialogueTrace,
) -> TracePrompt {
    TracePrompt {
        identity: trace_prompt_identity(prompt),
        line: line.map(|line| trace_line(line, dialogue_trace)),
        choices: choices
            .iter()
            .map(|choice| trace_choice(choice, dialogue_trace))
            .collect(),
    }
}

pub(super) fn trace_prompt_identity(prompt: &PromptIdentity) -> TracePromptIdentity {
    TracePromptIdentity {
        block: prompt.block.clone(),
        line: prompt.line.clone(),
        fixture_keys: prompt.fixture_keys.clone(),
    }
}
