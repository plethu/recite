//! Compile generated programs and drive the runtime the way the reference
//! interpreter is driven.

use proptest::prelude::*;
use recite_compiler::compile::{CompileInput, CompileOptions, compile_inputs_with_schema};
use recite_core::{
    ChoiceId, ScalarValue,
    compiled::{CompiledAssetId, CompiledDialogue, CompilerVersion, SourceMapId},
    schema::{ProjectSchema, load_schema_manifest_str},
};
use recite_runtime::{
    ConditionArgument, ConditionEvaluationError, ConditionQuery, ConditionValue,
    DialogueEffectArgument, DialogueEffectMode, DialogueEffectRequest, DialogueError,
    DialogueEvent, DialogueLine, DialogueSession, EffectAck, LocaleResolution, acknowledge_effect,
    choose_with,
    localisation::InterpolationValues,
    next_with,
    snapshot::{
        DialogueSessionSnapshot, decode_session_messagepack, encode_session_messagepack,
        snapshot_session,
    },
};
use recite_testkit::{
    ASSET_ID, Answer, Answers, ChoiceObservation, Dialogue, EffectMode, EffectObservation,
    LineObservation, Observation, Outcome, Program, RangePosition, Rejection, SCHEMA_JSON,
    SCHEMA_PATH,
};

/// Operations a driven run may take, which bounds runs that loop through
/// choices.
pub(super) const MAX_STEPS: usize = 64;

pub(super) fn failure(error: impl std::fmt::Debug) -> TestCaseError {
    TestCaseError::fail(format!("{error:?}"))
}

fn schema() -> ProjectSchema {
    load_schema_manifest_str(SCHEMA_PATH, SCHEMA_JSON)
        .schema
        .unwrap_or_else(|| panic!("generated schema is valid"))
}

pub(super) fn compile(program: &Program) -> Result<CompiledDialogue, TestCaseError> {
    let schema = schema();
    let sources = program.sources();
    let report = compile_inputs_with_schema(
        sources
            .iter()
            .map(|(path, source)| CompileInput::new(path.as_str(), source.as_str())),
        CompileOptions::new(
            CompilerVersion::new("0.0.1").map_err(failure)?,
            CompiledAssetId::new(ASSET_ID).map_err(failure)?,
            SourceMapId::new("generated.recitec.map").map_err(failure)?,
            schema.canonical_fingerprint(),
        ),
        &schema,
    )
    .map_err(failure)?;
    prop_assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    report
        .asset
        .map(|asset| asset.dialogue)
        .ok_or_else(|| TestCaseError::fail("a valid program compiles to an asset"))
}

fn evaluate(
    answers: &Answers,
    query: ConditionQuery<'_>,
) -> Result<ConditionValue, ConditionEvaluationError> {
    let args: Vec<String> = query
        .arguments()
        .iter()
        .map(|argument| match argument {
            ConditionArgument::Identifier(value) | ConditionArgument::String(value) => {
                value.to_owned()
            }
            other => format!("{other:?}"),
        })
        .collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match answers.answer(query.function(), &args) {
        Some(Answer::Bool(value)) => Ok(ConditionValue::Bool(value)),
        Some(Answer::Variant(variant)) => Ok(ConditionValue::EnumVariant(variant.to_owned())),
        None => Err(ConditionEvaluationError::new("not a generated condition")),
    }
}

/// The runtime as a [`Dialogue`]. Before an operation whose checkpoint flag
/// is set, the session is saved, encoded, decoded and restored, and play
/// continues with the restored session.
pub(super) struct RuntimeDialogue<'a> {
    asset: &'a CompiledDialogue,
    pub(super) session: DialogueSession,
    answers: &'a Answers,
    values: InterpolationValues,
    checkpoints: Vec<bool>,
    operations: usize,
    last_event: Option<DialogueEvent>,
    pub(super) events: Vec<DialogueEvent>,
    pub(super) failure: Option<String>,
    /// The next advance presents a prompt with its saved availability.
    saved_prompt: bool,
    /// Saves after every observation, when enabled.
    pub(super) saves: Option<Vec<Save>>,
}

/// A save taken after an observation, or before the first one.
pub(super) struct Save {
    pub(super) snapshot: DialogueSessionSnapshot,
    pub(super) after: Option<Observation>,
}

impl<'a> RuntimeDialogue<'a> {
    pub(super) fn new(
        asset: &'a CompiledDialogue,
        session: DialogueSession,
        answers: &'a Answers,
        checkpoints: Vec<bool>,
    ) -> Self {
        let mut values = InterpolationValues::new();
        values.insert("count".to_owned(), ScalarValue::Integer(answers.count));
        Self {
            asset,
            session,
            answers,
            values,
            checkpoints,
            operations: 0,
            last_event: None,
            events: Vec::new(),
            failure: None,
            saved_prompt: false,
            saves: None,
        }
    }

    /// Expects the next advance to present a resumed prompt with its saved
    /// availability, without querying conditions.
    pub(super) fn presenting_saved_prompt(mut self) -> Self {
        self.saved_prompt = true;
        self
    }

    /// Records a save before the first operation and after every observation.
    pub(super) fn saving(mut self) -> Self {
        let mut saves = Vec::new();
        match snapshot_session(self.asset, &self.session) {
            Ok(snapshot) => saves.push(Save {
                snapshot,
                after: None,
            }),
            Err(error) => self.failure = Some(format!("{error:?}")),
        }
        self.saves = Some(saves);
        self
    }

    fn checkpoint(&mut self) -> Result<(), Rejection> {
        let due = !self.checkpoints.is_empty()
            && self.checkpoints[self.operations % self.checkpoints.len()];
        self.operations += 1;
        if !due {
            return Ok(());
        }
        self.restore().map_err(|reason| {
            self.failure = Some(reason.clone());
            Rejection::Other(reason)
        })
    }

    fn restore(&mut self) -> Result<(), String> {
        let saved =
            snapshot_session(self.asset, &self.session).map_err(|error| format!("{error:?}"))?;
        let bytes = encode_session_messagepack(self.asset, &self.session)
            .map_err(|error| format!("{error:?}"))?;
        let mut restored =
            decode_session_messagepack(self.asset, &bytes).map_err(|error| format!("{error:?}"))?;
        let round_trip =
            snapshot_session(self.asset, &restored).map_err(|error| format!("{error:?}"))?;
        if round_trip != saved {
            return Err(format!(
                "snapshot round trip changed the session: {saved:?} -> {round_trip:?}"
            ));
        }
        if saved.pending_prompt.is_some() || saved.pending_effect.is_some() {
            // A restored prompt or blocking effect is presented once more.
            self.saved_prompt = saved.pending_prompt.is_some();
            let presented = self.advance(&mut restored);
            self.saved_prompt = false;
            if presented.as_ref().ok() != self.last_event.as_ref() {
                return Err(format!(
                    "restored session presented {presented:?}, expected {:?}",
                    self.last_event
                ));
            }
        }
        self.session = restored;
        Ok(())
    }

    /// Advances `session`. A restored prompt is presented with its saved
    /// availability, so while `saved_prompt` holds the host refuses every
    /// condition query.
    fn advance(&self, session: &mut DialogueSession) -> Result<DialogueEvent, DialogueError> {
        let answers = self.answers;
        let saved_prompt = self.saved_prompt;
        let context = move |query: ConditionQuery<'_>| {
            if saved_prompt {
                return Err(ConditionEvaluationError::new(
                    "a restored prompt is presented without querying conditions",
                ));
            }
            evaluate(answers, query)
        };
        next_with(
            self.asset,
            session,
            &context,
            LocaleResolution::new().with_values(&self.values),
        )
    }

    fn record(&mut self, result: Result<DialogueEvent, DialogueError>) -> Outcome {
        match result {
            Ok(event) => {
                self.events.push(event.clone());
                let observation = observe(&event);
                if let Some(saves) = &mut self.saves {
                    match snapshot_session(self.asset, &self.session) {
                        Ok(snapshot) => saves.push(Save {
                            snapshot,
                            after: Some(observation.clone()),
                        }),
                        Err(error) => self.failure = Some(format!("{error:?}")),
                    }
                }
                self.last_event = Some(event);
                Ok(observation)
            }
            Err(error) => Err(rejection(error)),
        }
    }
}

impl Dialogue for RuntimeDialogue<'_> {
    fn next(&mut self) -> Outcome {
        self.checkpoint()?;
        let mut session = self.session.clone();
        let result = self.advance(&mut session);
        self.saved_prompt = false;
        self.session = session;
        self.record(result)
    }

    fn choose(&mut self, id: &str) -> Outcome {
        self.checkpoint()?;
        let id = ChoiceId::new(id).map_err(|error| Rejection::Other(format!("{error:?}")))?;
        let answers = self.answers;
        let context = move |query: ConditionQuery<'_>| evaluate(answers, query);
        let result = choose_with(
            self.asset,
            &mut self.session,
            id,
            &context,
            LocaleResolution::new().with_values(&self.values),
        );
        self.record(result)
    }

    fn acknowledge(&mut self) -> Result<(), Rejection> {
        self.checkpoint()?;
        let Some(DialogueEvent::Effect(request)) = &self.last_event else {
            return Err(Rejection::NoEffectPending);
        };
        acknowledge_effect(&mut self.session, request.id.clone(), EffectAck::Completed)
            .map_err(rejection)
    }
}

fn rejection(error: DialogueError) -> Rejection {
    match error {
        DialogueError::SessionEnded => Rejection::SessionEnded,
        DialogueError::PromptPending { .. } => Rejection::PromptPending,
        DialogueError::EffectPending { .. } => Rejection::EffectPending,
        DialogueError::NoPromptPending { .. } => Rejection::NoPromptPending,
        DialogueError::InvalidChoice { .. } => Rejection::InvalidChoice,
        DialogueError::UnavailableChoice { .. } => Rejection::UnavailableChoice,
        DialogueError::NoEffectPending { .. } => Rejection::NoEffectPending,
        other => Rejection::Other(format!("{other:?}")),
    }
}

fn line(line: &DialogueLine) -> LineObservation {
    LineObservation {
        id: line.id.as_str().to_owned(),
        speaker: line
            .speaker
            .as_ref()
            .map(|speaker| speaker.as_str().to_owned()),
        text: line.text.clone(),
    }
}

pub(super) fn effect(request: &DialogueEffectRequest) -> EffectObservation {
    EffectObservation {
        mode: match request.mode {
            DialogueEffectMode::Immediate => EffectMode::Immediate,
            DialogueEffectMode::Deferred => EffectMode::Deferred,
            DialogueEffectMode::Blocking => EffectMode::Blocking,
        },
        function: request.function.clone(),
        args: request
            .args
            .iter()
            .map(|argument| match argument {
                DialogueEffectArgument::Identifier(value) => value.clone(),
                other => format!("{other:?}"),
            })
            .collect(),
    }
}

pub(super) fn observe(event: &DialogueEvent) -> Observation {
    match event {
        DialogueEvent::Line(observed) => Observation::Line(line(observed)),
        DialogueEvent::Prompt {
            line: text,
            choices,
        } => Observation::Prompt {
            line: text.as_ref().map(line),
            choices: choices
                .iter()
                .map(|choice| ChoiceObservation {
                    id: choice.id.as_str().to_owned(),
                    text: choice.text.clone(),
                    available: choice.availability.is_available,
                })
                .collect(),
        },
        DialogueEvent::Effect(request) => Observation::Effect(effect(request)),
        DialogueEvent::End { deferred_effects } => Observation::End {
            deferred: deferred_effects.iter().map(effect).collect(),
        },
    }
}

/// A saved position as the spans of each compiled range, outermost first,
/// for locating it in the source AST.
pub(super) fn levels(
    asset: &CompiledDialogue,
    snapshot: &DialogueSessionSnapshot,
) -> Vec<RangePosition> {
    let level = |start: u32, len: u32, next: u32| RangePosition {
        spans: (start..start + len)
            .map(|index| {
                let statement = &asset.statements[index as usize];
                asset.source_maps[statement.source_map.as_u32() as usize]
                    .span
                    .clone()
            })
            .collect(),
        next: next.saturating_sub(start) as usize,
    };
    snapshot
        .continuation_stack
        .iter()
        .map(|frame| level(frame.range.start, frame.range.len, frame.next_statement))
        .chain(std::iter::once(level(
            snapshot.current_range.start,
            snapshot.current_range.len,
            snapshot.next_statement,
        )))
        .collect()
}
