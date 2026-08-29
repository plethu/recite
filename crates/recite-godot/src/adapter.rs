use std::cell::Cell;
use std::collections::BTreeMap;
use std::sync::Arc;

use recite_core::{ChoiceId, CompiledDialogue, EffectId};
use recite_runtime::{
    ConditionEvaluationError, ConditionQuery, ConditionValue, DialogueContext, DialogueError,
    DialogueEvent, DialogueSession, EffectAck, InterpolationValues, LocaleResolution,
    acknowledge_effect, choose_with, decode_session_messagepack, encode_session_messagepack,
    next_with, start_scene_with_options,
};

use crate::adapter_policy::{session_options, should_continue_after_event};
pub(crate) use crate::adapter_surface::{
    AdapterValue, ConditionCall, ReciteDialogueAsset, ReciteOutput,
};
use crate::catalog::ReciteDialogueCatalog;

pub(crate) use crate::adapter_error::{AdapterError, AdapterErrorKind, AdapterResult};

pub type ConditionHandlerResult = Result<ConditionValue, AdapterError>;
type ConditionHandler = dyn Fn(ConditionCall<'_>) -> ConditionHandlerResult;

#[derive(Default)]
pub struct ReciteDialogueDriver {
    session: Option<ActiveSession>,
    conditions: BTreeMap<String, Box<ConditionHandler>>,
    interpolation_values: InterpolationValues,
    locale_catalog: Option<ReciteDialogueCatalog>,
    locale_variant: Option<String>,
    // Runtime errors carry display text only, so this state keeps the adapter
    // category separate while a traversal call is in progress.
    // It is consumed when the runtime returns that condition failure.
    condition_error: Cell<Option<AdapterErrorKind>>,
}

impl ReciteDialogueDriver {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_condition<F>(&mut self, name: impl Into<String>, handler: F)
    where
        F: Fn(ConditionCall<'_>) -> ConditionHandlerResult + 'static,
    {
        self.conditions.insert(name.into(), Box::new(handler));
    }

    pub fn unregister_condition(&mut self, name: &str) {
        self.conditions.remove(name);
    }

    /// Replaces the caller-owned typed values used for line and choice
    /// interpolation. Values are copied into the driver and are not part of a
    /// serialised runtime session snapshot.
    pub fn set_interpolation_values(&mut self, values: InterpolationValues) {
        self.interpolation_values = values;
    }

    /// Replaces the owned catalogue used by subsequent traversal calls. The
    /// session locale remains explicit: an absent locale bypasses the
    /// catalogue and emits authored source text.
    pub fn set_locale_catalog(&mut self, catalog: ReciteDialogueCatalog) {
        self.locale_catalog = Some(catalog);
    }

    pub fn clear_locale_catalog(&mut self) {
        self.locale_catalog = None;
    }

    /// Sets the grammatical variant used by subsequent locale lookups. The
    /// value is adapter-owned and is deliberately not part of runtime save
    /// state; restore callers must supply it again when needed.
    pub fn set_locale_variant(&mut self, variant: Option<&str>) -> AdapterResult<()> {
        self.locale_variant = validate_variant(variant)?;
        Ok(())
    }

    pub fn start(
        &mut self,
        asset: &ReciteDialogueAsset,
        block_id: Option<&str>,
        locale: Option<&str>,
    ) -> AdapterResult<Vec<ReciteOutput>> {
        self.start_with_variant(asset, block_id, locale, None)
    }

    pub fn start_with_variant(
        &mut self,
        asset: &ReciteDialogueAsset,
        block_id: Option<&str>,
        locale: Option<&str>,
        variant: Option<&str>,
    ) -> AdapterResult<Vec<ReciteOutput>> {
        if self.session.is_some() {
            return Err(AdapterError::new(AdapterErrorKind::SessionAlreadyActive));
        }

        let variant = validate_variant(variant)?;
        let previous_variant = self.locale_variant.clone();
        let options = session_options(locale)?;
        self.locale_variant = variant;
        let mut session = match start_scene_with_options(asset.dialogue(), block_id, options) {
            Ok(session) => session,
            Err(error) => {
                self.locale_variant = previous_variant;
                return Err(self.map_dialogue_error(error));
            }
        };
        let dialogue = asset.shared_dialogue();
        let outputs = match self.drain_from_next(&dialogue, &mut session) {
            Ok(outputs) => outputs,
            Err(error) => {
                self.locale_variant = previous_variant;
                return Err(error);
            }
        };
        self.session = Some(ActiveSession { dialogue, session });
        Ok(outputs)
    }

    pub fn select_choice(&mut self, choice_id: &str) -> AdapterResult<Vec<ReciteOutput>> {
        let choice_id = ChoiceId::new(choice_id).map_err(|error| {
            AdapterError::with_detail(AdapterErrorKind::InvalidChoice, error.to_string())
        })?;
        let mut active = self.take_active_session()?;
        let session_checkpoint = active.session.clone();
        let result = match choose_with(
            &active.dialogue,
            &mut active.session,
            choice_id,
            self,
            self.locale_resolution(),
        ) {
            Ok(first_event) => {
                self.drain_after_event(&active.dialogue, &mut active.session, first_event)
            }
            Err(error) => Err(self.map_dialogue_error(error)),
        };
        if result.is_err() {
            active.session = session_checkpoint;
        }
        self.session = Some(active);
        result
    }

    pub fn acknowledge_effect(
        &mut self,
        effect_request_id: &str,
        succeeded: bool,
        failure_reason: Option<&str>,
    ) -> AdapterResult<Vec<ReciteOutput>> {
        let effect_id = EffectId::new(effect_request_id).map_err(|error| {
            AdapterError::with_detail(AdapterErrorKind::EffectAcknowledgement, error.to_string())
        })?;
        let ack = if succeeded {
            EffectAck::Completed
        } else {
            EffectAck::Failed {
                reason: failure_reason.unwrap_or("").to_owned(),
            }
        };
        let mut active = self.take_active_session()?;
        let session_checkpoint = active.session.clone();
        let result = match acknowledge_effect(&mut active.session, effect_id, ack) {
            Ok(()) => self.drain_from_next(&active.dialogue, &mut active.session),
            Err(error) => Err(AdapterError::from(error)),
        };
        if result.is_err() {
            active.session = session_checkpoint;
        }
        self.session = Some(active);
        result
    }

    pub fn snapshot(&self) -> AdapterResult<Vec<u8>> {
        let active = self.active_session()?;
        encode_session_messagepack(&active.session).map_err(AdapterError::from)
    }

    pub fn restore(
        &mut self,
        asset: &ReciteDialogueAsset,
        snapshot_bytes: &[u8],
    ) -> AdapterResult<Vec<ReciteOutput>> {
        self.restore_with_variant(asset, snapshot_bytes, None)
    }

    pub fn restore_with_variant(
        &mut self,
        asset: &ReciteDialogueAsset,
        snapshot_bytes: &[u8],
        variant: Option<&str>,
    ) -> AdapterResult<Vec<ReciteOutput>> {
        if self.session.is_some() {
            return Err(AdapterError::new(AdapterErrorKind::SessionAlreadyActive));
        }

        let variant = validate_variant(variant)?;
        let previous_variant = self.locale_variant.clone();
        self.locale_variant = variant;
        let mut session = match decode_session_messagepack(asset.dialogue(), snapshot_bytes) {
            Ok(session) => session,
            Err(error) => {
                self.locale_variant = previous_variant;
                return Err(AdapterError::from_restore_error(error));
            }
        };
        let dialogue = asset.shared_dialogue();
        let outputs = match self.drain_restored(&dialogue, &mut session) {
            Ok(outputs) => outputs,
            Err(error) => {
                self.locale_variant = previous_variant;
                return Err(error);
            }
        };
        self.session = Some(ActiveSession { dialogue, session });
        Ok(outputs)
    }

    pub fn end_session(&mut self) -> AdapterResult<()> {
        if self.session.take().is_none() {
            return Err(AdapterError::new(AdapterErrorKind::NoActiveSession));
        }
        Ok(())
    }

    #[must_use]
    pub fn has_active_session(&self) -> bool {
        self.session.is_some()
    }

    fn take_active_session(&mut self) -> AdapterResult<ActiveSession> {
        self.session
            .take()
            .ok_or_else(|| AdapterError::new(AdapterErrorKind::NoActiveSession))
    }

    fn record_condition_error(&self, kind: AdapterErrorKind) {
        self.condition_error.set(Some(kind));
    }

    fn map_dialogue_error(&self, error: DialogueError) -> AdapterError {
        if matches!(&error, DialogueError::ConditionEvaluationFailed { .. }) {
            if let Some(kind) = self.condition_error.take() {
                return AdapterError::with_detail(kind, error.to_string());
            }
        } else {
            self.condition_error.take();
        }
        AdapterError::from(error)
    }

    fn active_session(&self) -> AdapterResult<&ActiveSession> {
        self.session
            .as_ref()
            .ok_or_else(|| AdapterError::new(AdapterErrorKind::NoActiveSession))
    }

    fn drain_restored(
        &self,
        dialogue: &CompiledDialogue,
        session: &mut DialogueSession,
    ) -> AdapterResult<Vec<ReciteOutput>> {
        match next_with(dialogue, session, self, self.locale_resolution()) {
            Ok(event) => self.drain_after_event(dialogue, session, event),
            Err(DialogueError::PromptPending { .. } | DialogueError::EffectPending { .. }) => {
                Ok(Vec::new())
            }
            Err(error) => Err(self.map_dialogue_error(error)),
        }
    }

    fn drain_from_next(
        &self,
        dialogue: &CompiledDialogue,
        session: &mut DialogueSession,
    ) -> AdapterResult<Vec<ReciteOutput>> {
        let first_event = next_with(dialogue, session, self, self.locale_resolution())
            .map_err(|error| self.map_dialogue_error(error))?;
        self.drain_after_event(dialogue, session, first_event)
    }

    fn drain_after_event(
        &self,
        dialogue: &CompiledDialogue,
        session: &mut DialogueSession,
        first_event: DialogueEvent,
    ) -> AdapterResult<Vec<ReciteOutput>> {
        let mut outputs = Vec::new();
        let mut event = first_event;

        loop {
            let should_continue = should_continue_after_event(&event);
            outputs.push(ReciteOutput::from(event));
            if !should_continue {
                return Ok(outputs);
            }

            match next_with(dialogue, session, self, self.locale_resolution()) {
                Ok(next_event) => event = next_event,
                Err(DialogueError::PromptPending { .. } | DialogueError::EffectPending { .. }) => {
                    return Ok(outputs);
                }
                Err(error) => return Err(self.map_dialogue_error(error)),
            }
        }
    }

    fn locale_resolution(&self) -> LocaleResolution<'_> {
        let resolution = LocaleResolution::new().with_values(&self.interpolation_values);
        let resolution = self
            .locale_variant
            .as_deref()
            .map_or(resolution, |variant| resolution.with_variant(variant));
        self.locale_catalog
            .as_ref()
            .map_or(resolution, |catalog| resolution.with_provider(catalog))
    }
}

fn validate_variant(variant: Option<&str>) -> AdapterResult<Option<String>> {
    let Some(variant) = variant else {
        return Ok(None);
    };
    if variant.is_empty() {
        return Ok(None);
    }
    if variant.contains('\0') {
        return Err(AdapterError::with_detail(
            AdapterErrorKind::Localisation,
            "locale variant must not contain NUL",
        ));
    }
    Ok(Some(variant.to_owned()))
}

impl DialogueContext for ReciteDialogueDriver {
    fn evaluate_condition(
        &self,
        query: ConditionQuery<'_>,
    ) -> Result<ConditionValue, ConditionEvaluationError> {
        let Some(handler) = self.conditions.get(query.function()) else {
            let error = AdapterError::new(AdapterErrorKind::MissingConditionHandler);
            self.record_condition_error(error.kind());
            return Err(ConditionEvaluationError::new(error.message()));
        };

        handler(ConditionCall { query }).map_err(|error| {
            self.record_condition_error(error.kind());
            ConditionEvaluationError::new(error.message())
        })
    }
}

#[derive(Debug)]
struct ActiveSession {
    dialogue: Arc<CompiledDialogue>,
    session: DialogueSession,
}
