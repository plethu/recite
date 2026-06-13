use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use recite_core::{
    ChoiceId, CompiledDialogue, EffectId, LocaleId, decode_compiled_dialogue_messagepack,
};
use recite_runtime::{
    ConditionArgument, ConditionEvaluationError, ConditionExpectedType, ConditionQuery,
    ConditionValue, DialogueChoice, DialogueContext, DialogueEffectMode, DialogueEffectRequest,
    DialogueError, DialogueEvent, DialogueLine, DialogueSession, DialogueSessionOptions, EffectAck,
    LocaleResolution, acknowledge_effect, choose_with, decode_session_messagepack,
    encode_session_messagepack, next_with, start_scene_with_options,
};

pub(crate) use crate::adapter_error::{AdapterError, AdapterErrorKind, AdapterResult};

pub type ConditionHandlerResult = Result<ConditionValue, AdapterError>;
type ConditionHandler = dyn Fn(ConditionCall<'_>) -> ConditionHandlerResult;

#[derive(Clone, Debug)]
pub struct ReciteDialogueAsset {
    dialogue: Arc<CompiledDialogue>,
}

impl ReciteDialogueAsset {
    pub fn load_from_bytes(bytes: &[u8]) -> AdapterResult<Self> {
        let dialogue = decode_compiled_dialogue_messagepack(bytes).map_err(AdapterError::from)?;
        Ok(Self {
            dialogue: Arc::new(dialogue),
        })
    }

    pub fn load_from_path(path: impl AsRef<Path>) -> AdapterResult<Self> {
        let path = path.as_ref();
        let bytes = std::fs::read(path).map_err(|error| {
            AdapterError::with_detail(
                AdapterErrorKind::AssetLoadOrDecode,
                format!("failed to read `{}`: {error}", path.display()),
            )
        })?;
        Self::load_from_bytes(&bytes)
    }

    #[must_use]
    pub fn dialogue(&self) -> &CompiledDialogue {
        &self.dialogue
    }

    #[must_use]
    pub fn shared_dialogue(&self) -> Arc<CompiledDialogue> {
        Arc::clone(&self.dialogue)
    }

    #[must_use]
    pub fn asset_id(&self) -> &str {
        self.dialogue.header.asset_id.as_str()
    }
}

#[derive(Default)]
pub struct ReciteDialogueDriver {
    session: Option<ActiveSession>,
    conditions: BTreeMap<String, Box<ConditionHandler>>,
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

    pub fn start(
        &mut self,
        asset: &ReciteDialogueAsset,
        block_id: Option<&str>,
        locale: Option<&str>,
    ) -> AdapterResult<Vec<ReciteOutput>> {
        if self.session.is_some() {
            return Err(AdapterError::new(AdapterErrorKind::SessionAlreadyActive));
        }

        let options = session_options(locale)?;
        let mut session = start_scene_with_options(asset.dialogue(), block_id, options)?;
        let dialogue = asset.shared_dialogue();
        let outputs = self.drain_from_next(&dialogue, &mut session)?;
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
            LocaleResolution::new(),
        ) {
            Ok(first_event) => {
                self.drain_after_event(&active.dialogue, &mut active.session, first_event)
            }
            Err(error) => Err(AdapterError::from(error)),
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
        if self.session.is_some() {
            return Err(AdapterError::new(AdapterErrorKind::SessionAlreadyActive));
        }

        let mut session = decode_session_messagepack(asset.dialogue(), snapshot_bytes)?;
        let dialogue = asset.shared_dialogue();
        let outputs = self.drain_restored(&dialogue, &mut session)?;
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
        match next_with(dialogue, session, self, LocaleResolution::new()) {
            Ok(event) => self.drain_after_event(dialogue, session, event),
            Err(DialogueError::PromptPending { .. } | DialogueError::EffectPending { .. }) => {
                Ok(Vec::new())
            }
            Err(error) => Err(AdapterError::from(error)),
        }
    }

    fn drain_from_next(
        &self,
        dialogue: &CompiledDialogue,
        session: &mut DialogueSession,
    ) -> AdapterResult<Vec<ReciteOutput>> {
        let first_event = next_with(dialogue, session, self, LocaleResolution::new())?;
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

            match next_with(dialogue, session, self, LocaleResolution::new()) {
                Ok(next_event) => event = next_event,
                Err(DialogueError::PromptPending { .. } | DialogueError::EffectPending { .. }) => {
                    return Ok(outputs);
                }
                Err(error) => return Err(AdapterError::from(error)),
            }
        }
    }
}

impl DialogueContext for ReciteDialogueDriver {
    fn evaluate_condition(
        &self,
        query: ConditionQuery<'_>,
    ) -> Result<ConditionValue, ConditionEvaluationError> {
        let Some(handler) = self.conditions.get(query.function()) else {
            return Err(ConditionEvaluationError::new(
                AdapterError::new(AdapterErrorKind::MissingConditionHandler).to_string(),
            ));
        };

        handler(ConditionCall { query })
            .map_err(|error| ConditionEvaluationError::new(error.to_string()))
    }
}

#[derive(Debug)]
struct ActiveSession {
    dialogue: Arc<CompiledDialogue>,
    session: DialogueSession,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConditionCall<'a> {
    query: ConditionQuery<'a>,
}

impl<'a> ConditionCall<'a> {
    #[must_use]
    pub fn function(self) -> &'a str {
        self.query.function()
    }

    #[must_use]
    pub fn expected_type(self) -> ConditionExpectedType {
        self.query.expected_type()
    }

    pub fn arguments(self) -> impl Iterator<Item = AdapterValue> + 'a {
        self.query.arguments().into_iter().map(AdapterValue::from)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum AdapterValue {
    Identifier(String),
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
}

impl From<ConditionArgument<'_>> for AdapterValue {
    fn from(argument: ConditionArgument<'_>) -> Self {
        match argument {
            ConditionArgument::Identifier(value) => Self::Identifier(value.to_owned()),
            ConditionArgument::String(value) => Self::String(value.to_owned()),
            ConditionArgument::Integer(value) => Self::Integer(value),
            ConditionArgument::Float(value) => Self::Float(value),
            ConditionArgument::Boolean(value) => Self::Boolean(value),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ReciteOutput {
    Line(DialogueLine),
    Prompt {
        line: Option<DialogueLine>,
        choices: Vec<DialogueChoice>,
    },
    Effect(DialogueEffectRequest),
    End {
        deferred_effects: Vec<DialogueEffectRequest>,
    },
}

impl From<DialogueEvent> for ReciteOutput {
    fn from(event: DialogueEvent) -> Self {
        match event {
            DialogueEvent::Line(line) => Self::Line(line),
            DialogueEvent::Prompt { line, choices } => Self::Prompt { line, choices },
            DialogueEvent::Effect(effect) => Self::Effect(effect),
            DialogueEvent::End { deferred_effects } => Self::End { deferred_effects },
        }
    }
}

fn session_options(locale: Option<&str>) -> AdapterResult<DialogueSessionOptions> {
    let Some(locale) = locale.filter(|locale| !locale.is_empty()) else {
        return Ok(DialogueSessionOptions::new());
    };
    let locale = LocaleId::new(locale).map_err(|error| {
        AdapterError::with_detail(AdapterErrorKind::Localisation, error.to_string())
    })?;
    Ok(DialogueSessionOptions::new().with_locale(locale))
}

fn should_continue_after_event(event: &DialogueEvent) -> bool {
    matches!(
        event,
        DialogueEvent::Line(_)
            | DialogueEvent::Effect(DialogueEffectRequest {
                mode: DialogueEffectMode::Immediate,
                ..
            })
    )
}
