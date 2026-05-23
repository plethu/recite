use recite_core::{
    BlockIndex, ChoiceId, CompiledAssetHeader, CompiledAssetId, CompiledDivertTarget,
    CompiledSourceFile, CompilerVersion, LocaleId, SchemaFingerprint, SourceMapId, StatementIndex,
    StatementRange,
};

use crate::{DialogueEffectRequest, DialogueError, DialogueEvent};

/// Compact, asset-free runtime session state.
#[derive(Clone, Debug, PartialEq)]
pub struct DialogueSession {
    pub(crate) asset_id: CompiledAssetId,
    pub(crate) format_version: u16,
    pub(crate) compiler_compatibility_version: u16,
    pub(crate) compiler_version: CompilerVersion,
    pub(crate) source_map_id: SourceMapId,
    pub(crate) schema_fingerprint: SchemaFingerprint,
    pub(crate) sources: Vec<CompiledSourceFile>,
    pub(crate) current_block: BlockIndex,
    pub(crate) current_range: StatementRange,
    pub(crate) next_statement: StatementIndex,
    pub(crate) continuation_stack: Vec<StatementFrame>,
    pub(crate) pending_prompt: Option<PendingPrompt>,
    pub(crate) pending_effect: Option<DialogueEffectRequest>,
    pub(crate) previous_prompt_choices: Vec<ChoiceId>,
    pub(crate) selected_choice_history: Vec<ChoiceId>,
    pub(crate) deferred_effects: Vec<DialogueEffectRequest>,
    pub(crate) locale: Option<LocaleId>,
    pub(crate) trace_counter: u64,
    pub(crate) ended: bool,
}

impl DialogueSession {
    pub(crate) fn new(
        header: &CompiledAssetHeader,
        sources: Vec<CompiledSourceFile>,
        current_block: BlockIndex,
        current_range: StatementRange,
        options: DialogueSessionOptions,
    ) -> Self {
        Self {
            asset_id: header.asset_id.clone(),
            format_version: header.format_version,
            compiler_compatibility_version: header.compiler_compatibility_version,
            compiler_version: header.compiler_version.clone(),
            source_map_id: header.source_map_id.clone(),
            schema_fingerprint: header.schema_fingerprint.clone(),
            sources,
            current_block,
            current_range,
            next_statement: current_range.start,
            continuation_stack: Vec::new(),
            pending_prompt: None,
            pending_effect: None,
            previous_prompt_choices: Vec::new(),
            selected_choice_history: Vec::new(),
            deferred_effects: Vec::new(),
            locale: options.locale,
            trace_counter: 0,
            ended: false,
        }
    }

    pub(crate) fn emit(&mut self, event: DialogueEvent) -> Result<DialogueEvent, DialogueError> {
        self.trace_counter = self.next_trace_counter()?;

        Ok(event)
    }

    pub(crate) fn next_trace_counter(&self) -> Result<u64, DialogueError> {
        self.trace_counter
            .checked_add(1)
            .ok_or_else(|| DialogueError::MalformedCompiledAsset {
                reason: "session trace counter overflowed".to_owned(),
            })
    }

    #[must_use]
    pub fn selected_choice_history(&self) -> &[ChoiceId] {
        &self.selected_choice_history
    }

    #[must_use]
    pub fn deferred_effects(&self) -> &[DialogueEffectRequest] {
        &self.deferred_effects
    }

    #[must_use]
    pub fn pending_effect(&self) -> Option<&DialogueEffectRequest> {
        self.pending_effect.as_ref()
    }

    #[must_use]
    pub fn locale(&self) -> Option<&LocaleId> {
        self.locale.as_ref()
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DialogueSessionOptions {
    locale: Option<LocaleId>,
}

impl DialogueSessionOptions {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_locale(mut self, locale: LocaleId) -> Self {
        self.locale = Some(locale);
        self
    }

    #[must_use]
    pub fn locale(&self) -> Option<&LocaleId> {
        self.locale.as_ref()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct StatementFrame {
    pub(crate) range: StatementRange,
    pub(crate) next_statement: StatementIndex,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PendingPrompt {
    pub(crate) statement: StatementIndex,
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
