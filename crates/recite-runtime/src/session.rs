use recite_core::{
    BlockIndex, ChoiceId, CompiledAssetId, CompiledDivertTarget, StatementIndex, StatementRange,
};

use crate::{DialogueError, DialogueEvent};

/// Compact, asset-free runtime session state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DialogueSession {
    pub(crate) asset_id: CompiledAssetId,
    pub(crate) format_version: u16,
    pub(crate) compiler_compatibility_version: u16,
    pub(crate) current_block: BlockIndex,
    pub(crate) current_range: StatementRange,
    pub(crate) next_statement: StatementIndex,
    pub(crate) continuation_stack: Vec<StatementFrame>,
    pub(crate) pending_prompt: Option<PendingPrompt>,
    pub(crate) previous_prompt_choices: Vec<ChoiceId>,
    pub(crate) selected_choice_history: Vec<ChoiceId>,
    pub(crate) trace_counter: u64,
    pub(crate) ended: bool,
}

impl DialogueSession {
    pub(crate) fn new(
        asset_id: CompiledAssetId,
        format_version: u16,
        compiler_compatibility_version: u16,
        current_block: BlockIndex,
        current_range: StatementRange,
    ) -> Self {
        Self {
            asset_id,
            format_version,
            compiler_compatibility_version,
            current_block,
            current_range,
            next_statement: current_range.start,
            continuation_stack: Vec::new(),
            pending_prompt: None,
            previous_prompt_choices: Vec::new(),
            selected_choice_history: Vec::new(),
            trace_counter: 0,
            ended: false,
        }
    }

    pub(crate) fn emit(&mut self, event: DialogueEvent) -> Result<DialogueEvent, DialogueError> {
        self.trace_counter = self.trace_counter.checked_add(1).ok_or_else(|| {
            DialogueError::MalformedCompiledAsset {
                reason: "session trace counter overflowed".to_owned(),
            }
        })?;

        Ok(event)
    }

    #[must_use]
    pub fn selected_choice_history(&self) -> &[ChoiceId] {
        &self.selected_choice_history
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct StatementFrame {
    pub(crate) range: StatementRange,
    pub(crate) next_statement: StatementIndex,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PendingPrompt {
    pub(crate) choices: Vec<PendingPromptChoice>,
}

impl PendingPrompt {
    pub(crate) fn choice_ids(&self) -> Vec<ChoiceId> {
        self.choices
            .iter()
            .map(|choice| choice.id.clone())
            .collect()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PendingPromptChoice {
    pub(crate) id: ChoiceId,
    pub(crate) target: CompiledDivertTarget,
    pub(crate) is_available: bool,
    pub(crate) unavailable_reason: Option<String>,
}
