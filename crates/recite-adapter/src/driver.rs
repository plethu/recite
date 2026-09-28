use std::collections::BTreeSet;

use recite_core::{ChoiceId, EffectId};
use recite_runtime::{
    DialogueContext, DialogueError, DialogueEvent, DialogueSession, DialogueSessionOptions,
    EffectAck, LocaleResolution, acknowledge_effect, choose_with,
    snapshot::{decode_session_messagepack, encode_session_messagepack},
    start_scene_with_options,
};

use crate::{AdapterError, AdapterErrorKind, AdapterResult, LoadedDialogue};

mod drain;
use drain::{drain_after_event, drain_from_next};

/// Distinguishes traversal errors from a host's fallible output encoding.
#[non_exhaustive]
#[derive(Debug)]
pub enum DriverError<E> {
    Adapter(AdapterError),
    Output(E),
}

impl<E> From<AdapterError> for DriverError<E> {
    fn from(error: AdapterError) -> Self {
        Self::Adapter(error)
    }
}

#[derive(Debug)]
struct ActiveSession {
    asset: LoadedDialogue,
    session: DialogueSession,
}

#[derive(Debug)]
enum SessionState {
    Prepared(ActiveSession),
    Begun(ActiveSession),
}

impl SessionState {
    fn active(&self) -> &ActiveSession {
        match self {
            Self::Prepared(active) | Self::Begun(active) => active,
        }
    }
}

/// Immutable asset and options for one start request.
pub struct StartRequest<'a> {
    pub asset: &'a LoadedDialogue,
    pub block_id: Option<&'a str>,
    pub options: DialogueSessionOptions,
}

/// One session owner. Prepared sessions occupy the owner until ended.
#[derive(Debug, Default)]
pub struct SessionDriver {
    active: Option<SessionState>,
    /// Host observation metadata; never enters runtime snapshots. Restore can
    /// reseed only latest-prompt IDs and selected IDs retained by the snapshot,
    /// not unselected IDs from older prompts.
    observed_choices: BTreeSet<ChoiceId>,
}

impl SessionDriver {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn has_active_session(&self) -> bool {
        self.active.is_some()
    }

    #[must_use]
    pub fn active_asset(&self) -> Option<&LoadedDialogue> {
        self.active.as_ref().map(|state| &state.active().asset)
    }

    #[must_use]
    pub fn is_prepared(&self) -> bool {
        matches!(self.active, Some(SessionState::Prepared(_)))
    }

    pub fn prepare(&mut self, request: StartRequest<'_>) -> AdapterResult<()> {
        if self.has_active_session() {
            return Err(AdapterError::new(AdapterErrorKind::SessionAlreadyActive));
        }
        let session =
            start_scene_with_options(request.asset.dialogue(), request.block_id, request.options)?;
        self.observed_choices.clear();
        self.active = Some(SessionState::Prepared(ActiveSession {
            asset: request.asset.clone(),
            session,
        }));
        Ok(())
    }

    pub fn begin(
        &mut self,
        context: &dyn DialogueContext,
        resolution: LocaleResolution<'_>,
    ) -> AdapterResult<Vec<DialogueEvent>> {
        collapse(self.begin_with(context, resolution, Ok::<_, std::convert::Infallible>))
    }

    /// Commits the initial traversal only after `encode` succeeds.
    pub fn begin_with<T, E>(
        &mut self,
        context: &dyn DialogueContext,
        resolution: LocaleResolution<'_>,
        encode: impl FnOnce(Vec<DialogueEvent>) -> Result<T, E>,
    ) -> Result<T, DriverError<E>> {
        if !self.is_prepared() {
            return Err(AdapterError::new(if self.has_active_session() {
                AdapterErrorKind::SessionAlreadyActive
            } else {
                AdapterErrorKind::NoActiveSession
            })
            .into());
        }
        self.run_active(context, resolution, encode, |_, _, _| Ok(None))
    }

    pub fn start(
        &mut self,
        request: StartRequest<'_>,
        context: &dyn DialogueContext,
        resolution: LocaleResolution<'_>,
    ) -> AdapterResult<Vec<DialogueEvent>> {
        collapse(self.start_with(
            request,
            context,
            resolution,
            Ok::<_, std::convert::Infallible>,
        ))
    }

    pub fn start_with<T, E>(
        &mut self,
        request: StartRequest<'_>,
        context: &dyn DialogueContext,
        resolution: LocaleResolution<'_>,
        encode: impl FnOnce(Vec<DialogueEvent>) -> Result<T, E>,
    ) -> Result<T, DriverError<E>> {
        self.prepare(request)?;
        let result = self.begin_with(context, resolution, encode);
        if result.is_err() {
            self.active = None;
            self.observed_choices.clear();
        }
        result
    }

    pub fn select_choice(
        &mut self,
        choice: ChoiceId,
        context: &dyn DialogueContext,
        resolution: LocaleResolution<'_>,
    ) -> AdapterResult<Vec<DialogueEvent>> {
        collapse(self.select_choice_with(
            choice,
            context,
            resolution,
            Ok::<_, std::convert::Infallible>,
        ))
    }

    pub fn select_choice_with<T, E>(
        &mut self,
        choice: ChoiceId,
        context: &dyn DialogueContext,
        resolution: LocaleResolution<'_>,
        encode: impl FnOnce(Vec<DialogueEvent>) -> Result<T, E>,
    ) -> Result<T, DriverError<E>> {
        let previously_observed = self.observed_choices.contains(&choice);
        self.run_active(context, resolution, encode, |asset, session, resolution| {
            choose_with(asset, session, choice, context, resolution)
                .map(Some)
                .map_err(|error| classify_choice_error(error, previously_observed))
        })
    }

    pub fn acknowledge_effect(
        &mut self,
        effect: EffectId,
        ack: EffectAck,
        context: &dyn DialogueContext,
        resolution: LocaleResolution<'_>,
    ) -> AdapterResult<Vec<DialogueEvent>> {
        collapse(self.acknowledge_effect_with(
            effect,
            ack,
            context,
            resolution,
            Ok::<_, std::convert::Infallible>,
        ))
    }

    pub fn acknowledge_effect_with<T, E>(
        &mut self,
        effect: EffectId,
        ack: EffectAck,
        context: &dyn DialogueContext,
        resolution: LocaleResolution<'_>,
        encode: impl FnOnce(Vec<DialogueEvent>) -> Result<T, E>,
    ) -> Result<T, DriverError<E>> {
        self.run_active(context, resolution, encode, |_, session, _| {
            acknowledge_effect(session, effect, ack)?;
            Ok(None)
        })
    }

    pub fn restore(
        &mut self,
        asset: &LoadedDialogue,
        bytes: &[u8],
        context: &dyn DialogueContext,
        resolution: LocaleResolution<'_>,
    ) -> AdapterResult<Vec<DialogueEvent>> {
        collapse(self.restore_with(
            asset,
            bytes,
            context,
            resolution,
            Ok::<_, std::convert::Infallible>,
        ))
    }

    pub fn restore_with<T, E>(
        &mut self,
        asset: &LoadedDialogue,
        bytes: &[u8],
        context: &dyn DialogueContext,
        resolution: LocaleResolution<'_>,
        encode: impl FnOnce(Vec<DialogueEvent>) -> Result<T, E>,
    ) -> Result<T, DriverError<E>> {
        if self.has_active_session() {
            return Err(AdapterError::new(AdapterErrorKind::SessionAlreadyActive).into());
        }
        let session = decode_session_messagepack(asset.dialogue(), bytes)
            .map_err(AdapterError::from_restore_error)?;
        self.active = Some(SessionState::Prepared(ActiveSession {
            asset: asset.clone(),
            session,
        }));
        if let Some(active) = self.active.as_ref() {
            self.observed_choices = active
                .active()
                .session
                .previous_prompt_choices()
                .iter()
                .chain(active.active().session.selected_choice_history())
                .cloned()
                .collect();
        }
        let result = self.begin_with(context, resolution, encode);
        if result.is_err() {
            self.active = None;
            self.observed_choices.clear();
        }
        result
    }

    pub fn snapshot(&self) -> AdapterResult<Vec<u8>> {
        let active = self.active.as_ref().ok_or_else(no_active)?.active();
        encode_session_messagepack(&active.session).map_err(AdapterError::from)
    }

    pub fn end_session(&mut self) -> AdapterResult<()> {
        let result = self.active.take().map(|_| ()).ok_or_else(no_active);
        if result.is_ok() {
            self.observed_choices.clear();
        }
        result
    }

    fn run_active<T, E>(
        &mut self,
        context: &dyn DialogueContext,
        resolution: LocaleResolution<'_>,
        encode: impl FnOnce(Vec<DialogueEvent>) -> Result<T, E>,
        first: impl FnOnce(
            &recite_core::compiled::CompiledDialogue,
            &mut DialogueSession,
            LocaleResolution<'_>,
        ) -> AdapterResult<Option<DialogueEvent>>,
    ) -> Result<T, DriverError<E>> {
        let active = self.active.as_ref().ok_or_else(no_active)?.active();
        let mut trial = active.session.clone();
        let asset = active.asset.dialogue();
        let first_event = first(asset, &mut trial, resolution)?;
        let events = match first_event {
            Some(event) => drain_after_event(asset, &mut trial, context, resolution, event)?,
            None => drain_from_next(asset, &mut trial, context, resolution)?,
        };
        let observed = events
            .iter()
            .filter_map(|event| match event {
                DialogueEvent::Prompt { choices, .. } => {
                    Some(choices.iter().map(|choice| choice.id.clone()))
                }
                _ => None,
            })
            .flatten()
            .collect::<BTreeSet<_>>();
        let output = encode(events).map_err(DriverError::Output)?;
        let old = self.active.take().ok_or_else(no_active)?;
        self.active = Some(SessionState::Begun(ActiveSession {
            asset: old.active().asset.clone(),
            session: trial,
        }));
        self.observed_choices.extend(observed);
        Ok(output)
    }
}

fn classify_choice_error(error: DialogueError, previously_observed: bool) -> AdapterError {
    match error {
        DialogueError::InvalidChoice { .. } | DialogueError::NoPromptPending { .. } => {
            AdapterError::with_detail(
                if previously_observed {
                    AdapterErrorKind::StaleChoice
                } else {
                    AdapterErrorKind::InvalidChoice
                },
                error.to_string(),
            )
        }
        other => other.into(),
    }
}

fn no_active() -> AdapterError {
    AdapterError::new(AdapterErrorKind::NoActiveSession)
}

impl<E: std::fmt::Display> std::fmt::Display for DriverError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Adapter(error) => error.fmt(f),
            Self::Output(error) => error.fmt(f),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for DriverError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Adapter(error) => Some(error),
            Self::Output(error) => Some(error),
        }
    }
}

fn collapse<T>(result: Result<T, DriverError<std::convert::Infallible>>) -> AdapterResult<T> {
    match result {
        Ok(value) => Ok(value),
        Err(DriverError::Adapter(error)) => Err(error),
        Err(DriverError::Output(never)) => match never {},
    }
}
