use std::collections::BTreeMap;

use recite_core::{ChoiceId, CompiledDialogue, CompiledStatementKind};
use recite_runtime::{DialogueChoice, DialogueLine};

use super::fixture::{FixtureChoice, RuntimeFixture};
use super::trace::{TracePrompt, TracePromptIdentity, trace_choice, trace_line};
use crate::error::CliError;

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
            let statement_start = block.statements.start.as_u32();
            let statement_end = statement_start + block.statements.len;

            for statement_index in statement_start..statement_end {
                let statement =
                    asset
                        .statements
                        .get(statement_index as usize)
                        .ok_or_else(|| CliError::MalformedCompiledAsset {
                            reason: format!("statement index {statement_index} is out of bounds"),
                        })?;
                let CompiledStatementKind::Prompt { line, choices } = &statement.kind else {
                    continue;
                };

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

                *block_prompt_counts.entry(block_id.clone()).or_default() += 1;
                prompt_rows.push((block_id.clone(), line, choice_ids));
            }
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
    prompt: &PromptIdentity,
    line: Option<&DialogueLine>,
    choices: &[DialogueChoice],
) {
    match line {
        Some(line) => run_lines.push(format!("prompt {}: {}", line.id.as_str(), line.text)),
        None => run_lines.push(format!("prompt {}", prompt.fixture_keys.join("|"))),
    }

    for (index, choice) in choices.iter().enumerate() {
        let availability = if choice.is_available {
            ""
        } else {
            " (unavailable)"
        };
        run_lines.push(format!(
            "  [{}] {}: {}{}",
            index + 1,
            choice.id.as_str(),
            choice.text,
            availability
        ));
    }
}

pub(super) fn trace_prompt(
    prompt: &PromptIdentity,
    line: Option<&DialogueLine>,
    choices: &[DialogueChoice],
) -> TracePrompt {
    TracePrompt {
        identity: trace_prompt_identity(prompt),
        line: line.map(trace_line),
        choices: choices.iter().map(trace_choice).collect(),
    }
}

pub(super) fn trace_prompt_identity(prompt: &PromptIdentity) -> TracePromptIdentity {
    TracePromptIdentity {
        block: prompt.block.clone(),
        line: prompt.line.clone(),
        fixture_keys: prompt.fixture_keys.clone(),
    }
}
