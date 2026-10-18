//! Roll back traversal without copying append-only session histories.
use super::{DialogueSession, SessionPhase, StatementFrame};
use crate::{DialogueContext, DialogueError, DialogueEvent, EffectAck, LocaleResolution};
use recite_core::{
    ChoiceId, EffectId, LocaleId,
    compiled::{BlockIndex, CompiledDialogue, StatementIndex, StatementRange},
};
use std::sync::Arc;

/// Traversal within a session transaction. Session inspection is immutable;
/// only the runtime's append-preserving operations can change its state.
#[non_exhaustive]
pub struct SessionTraversal<'session> {
    session: &'session mut DialogueSession,
}

impl SessionTraversal<'_> {
    #[must_use]
    pub fn session(&self) -> &DialogueSession {
        self.session
    }

    pub fn next_with(
        &mut self,
        asset: &CompiledDialogue,
        context: &dyn DialogueContext,
        resolution: LocaleResolution<'_>,
    ) -> Result<DialogueEvent, DialogueError> {
        crate::next_with(asset, self.session, context, resolution)
    }

    pub fn choose_with(
        &mut self,
        asset: &CompiledDialogue,
        choice: ChoiceId,
        context: &dyn DialogueContext,
        resolution: LocaleResolution<'_>,
    ) -> Result<DialogueEvent, DialogueError> {
        crate::choose_with(asset, self.session, choice, context, resolution)
    }

    pub fn acknowledge_effect(
        &mut self,
        effect: EffectId,
        acknowledgement: EffectAck,
    ) -> Result<(), DialogueError> {
        crate::acknowledge_effect(self.session, effect, acknowledgement)
    }
}

impl DialogueSession {
    /// Run a batch, committing when its callback returns `Ok`. Propagate
    /// traversal and output errors from the callback to roll back.
    /// Returning an error or unwinding restores the complete original session.
    /// Retained session clones remain independent through copy-on-write.
    pub fn try_traverse<T, E>(
        &mut self,
        operation: impl FnOnce(&mut SessionTraversal<'_>) -> Result<T, E>,
    ) -> Result<T, E> {
        let checkpoint = Checkpoint::capture(self);
        let mut rollback = Rollback {
            session: self,
            checkpoint: Some(checkpoint),
        };
        let result = operation(&mut SessionTraversal {
            session: rollback.session,
        });
        if result.is_ok() {
            rollback.checkpoint = None;
        }
        result
    }
}

struct Checkpoint {
    block: BlockIndex,
    range: StatementRange,
    next: StatementIndex,
    stack: Vec<StatementFrame>,
    phase: SessionPhase,
    prompt_choices: Vec<ChoiceId>,
    choice_count: usize,
    deferred_count: usize,
    locale: Option<LocaleId>,
    trace_counter: u64,
}

impl Checkpoint {
    fn capture(session: &DialogueSession) -> Self {
        // Classify every field explicitly: new session state must be considered
        // here. Traversal cannot change asset identity or existing history entries.
        let DialogueSession {
            asset_id: _,
            format_version: _,
            compiler_compatibility_version: _,
            compiler_version: _,
            source_map_id: _,
            schema_fingerprint: _,
            compiled_payload_fingerprint: _,
            sources: _,
            current_block,
            current_range,
            next_statement,
            continuation_stack,
            phase,
            previous_prompt_choices,
            selected_choice_history,
            deferred_effects,
            locale,
            trace_counter,
        } = session;
        Self {
            block: *current_block,
            range: *current_range,
            next: *next_statement,
            stack: continuation_stack.clone(),
            phase: phase.clone(),
            prompt_choices: previous_prompt_choices.clone(),
            choice_count: selected_choice_history.len(),
            deferred_count: deferred_effects.len(),
            locale: locale.clone(),
            trace_counter: *trace_counter,
        }
    }

    fn restore(self, session: &mut DialogueSession) {
        session.current_block = self.block;
        session.current_range = self.range;
        session.next_statement = self.next;
        session.continuation_stack = self.stack;
        session.phase = self.phase;
        session.previous_prompt_choices = self.prompt_choices;
        session.selected_choice_history.truncate(self.choice_count);
        if session.deferred_effects.len() > self.deferred_count {
            Arc::make_mut(&mut session.deferred_effects).truncate(self.deferred_count);
        }
        session.locale = self.locale;
        session.trace_counter = self.trace_counter;
    }
}

struct Rollback<'session> {
    session: &'session mut DialogueSession,
    checkpoint: Option<Checkpoint>,
}

impl Drop for Rollback<'_> {
    fn drop(&mut self) {
        if let Some(checkpoint) = self.checkpoint.take() {
            checkpoint.restore(self.session);
        }
    }
}
