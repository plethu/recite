use recite_core::{
    BlockIndex, ChoiceId, CompiledDialogue, CompiledStatementKind, EffectId, LocaleId,
    SourcePosition, SourceSpan, StatementIndex, StatementRange,
};
use serde::{Deserialize, Serialize};

use crate::event::{DialogueEffectArgument, DialogueEffectMode, DialogueEffectRequest};
use crate::session::{PendingPrompt, PendingPromptChoice, StatementFrame};
use crate::traversal::AssetView;
use crate::{DialogueError, DialogueSession};

pub const SESSION_SNAPSHOT_FORMAT_VERSION_V0: u16 = 0;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DialogueSessionSnapshot {
    pub snapshot_format_version: u16,
    pub asset_id: String,
    pub asset_format_version: u16,
    pub asset_compiler_compatibility_version: u16,
    pub current_block: u32,
    pub current_range: DialogueSessionRangeSnapshot,
    pub next_statement: u32,
    pub continuation_stack: Vec<DialogueSessionFrameSnapshot>,
    pub pending_prompt: Option<DialogueSessionPendingPromptSnapshot>,
    pub previous_prompt_choices: Vec<String>,
    pub selected_choice_history: Vec<String>,
    pub deferred_effects: Vec<DialogueEffectRequestSnapshot>,
    pub locale: Option<String>,
    pub trace_counter: u64,
    pub ended: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DialogueSessionRangeSnapshot {
    pub start: u32,
    pub len: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DialogueSessionFrameSnapshot {
    pub range: DialogueSessionRangeSnapshot,
    pub next_statement: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DialogueSessionPendingPromptSnapshot {
    pub statement: u32,
    pub choices: Vec<DialogueSessionPendingChoiceSnapshot>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DialogueSessionPendingChoiceSnapshot {
    pub id: String,
    pub is_available: bool,
    pub unavailable_reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DialogueEffectRequestSnapshot {
    pub id: String,
    pub mode: DialogueEffectModeSnapshot,
    pub function: String,
    pub args: Vec<DialogueEffectArgumentSnapshot>,
    pub source_span: DialogueSourceSpanSnapshot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DialogueEffectModeSnapshot {
    Deferred,
    Immediate,
    Blocking,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DialogueEffectArgumentSnapshot {
    Identifier(String),
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DialogueSourceSpanSnapshot {
    pub file: String,
    pub start: DialogueSourcePositionSnapshot,
    pub end: Option<DialogueSourcePositionSnapshot>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DialogueSourcePositionSnapshot {
    pub line: u32,
    pub column: u32,
}

#[must_use]
pub fn snapshot_session(session: &DialogueSession) -> DialogueSessionSnapshot {
    DialogueSessionSnapshot {
        snapshot_format_version: SESSION_SNAPSHOT_FORMAT_VERSION_V0,
        asset_id: session.asset_id.as_str().to_owned(),
        asset_format_version: session.format_version,
        asset_compiler_compatibility_version: session.compiler_compatibility_version,
        current_block: session.current_block.as_u32(),
        current_range: range_snapshot(session.current_range),
        next_statement: session.next_statement.as_u32(),
        continuation_stack: session
            .continuation_stack
            .iter()
            .map(frame_snapshot)
            .collect(),
        pending_prompt: session.pending_prompt.as_ref().map(pending_prompt_snapshot),
        previous_prompt_choices: choice_ids_snapshot(&session.previous_prompt_choices),
        selected_choice_history: choice_ids_snapshot(&session.selected_choice_history),
        deferred_effects: session
            .deferred_effects
            .iter()
            .map(effect_request_snapshot)
            .collect(),
        locale: session
            .locale
            .as_ref()
            .map(|locale| locale.as_str().to_owned()),
        trace_counter: session.trace_counter,
        ended: session.ended,
    }
}

pub fn restore_session(
    asset: &CompiledDialogue,
    snapshot: DialogueSessionSnapshot,
) -> Result<DialogueSession, DialogueError> {
    if snapshot.snapshot_format_version != SESSION_SNAPSHOT_FORMAT_VERSION_V0 {
        return Err(DialogueError::UnsupportedSessionSnapshotFormat {
            snapshot_format_version: snapshot.snapshot_format_version,
        });
    }

    let asset_view = AssetView::new(asset)?;
    ensure_snapshot_matches_asset(asset, &snapshot)?;

    let current_block = BlockIndex::new(snapshot.current_block);
    asset_view.block_at(current_block)?;

    let current_range = statement_range(snapshot.current_range);
    asset_view.statement_range(current_range)?;

    let next_statement = StatementIndex::new(snapshot.next_statement);
    validate_statement_pointer("next statement", current_range, next_statement)?;

    let continuation_stack = restore_frames(asset_view, &snapshot.continuation_stack)?;
    let previous_prompt_choices = restore_choice_ids(
        asset_view,
        "previous prompt choices",
        &snapshot.previous_prompt_choices,
    )?;
    let selected_choice_history = restore_choice_ids(
        asset_view,
        "selected choice history",
        &snapshot.selected_choice_history,
    )?;
    let deferred_effects = restore_effects(&snapshot.deferred_effects)?;
    let locale = restore_locale(snapshot.locale.as_deref())?;
    let pending_prompt = restore_pending_prompt(
        asset_view,
        snapshot.pending_prompt.as_ref(),
        &previous_prompt_choices,
        snapshot.ended,
    )?;

    Ok(DialogueSession {
        asset_id: asset.header.asset_id.clone(),
        format_version: asset.header.format_version,
        compiler_compatibility_version: asset.header.compiler_compatibility_version,
        current_block,
        current_range,
        next_statement,
        continuation_stack,
        pending_prompt,
        previous_prompt_choices,
        selected_choice_history,
        deferred_effects,
        locale,
        trace_counter: snapshot.trace_counter,
        ended: snapshot.ended,
    })
}

pub fn encode_session_messagepack(session: &DialogueSession) -> Result<Vec<u8>, DialogueError> {
    rmp_serde::to_vec(&snapshot_session(session)).map_err(|error| {
        DialogueError::SessionSnapshotEncodeFailed {
            reason: error.to_string(),
        }
    })
}

pub fn decode_session_messagepack(
    asset: &CompiledDialogue,
    bytes: &[u8],
) -> Result<DialogueSession, DialogueError> {
    let snapshot: DialogueSessionSnapshot = rmp_serde::from_slice(bytes).map_err(|error| {
        DialogueError::SessionSnapshotDecodeFailed {
            reason: error.to_string(),
        }
    })?;

    restore_session(asset, snapshot)
}

fn ensure_snapshot_matches_asset(
    asset: &CompiledDialogue,
    snapshot: &DialogueSessionSnapshot,
) -> Result<(), DialogueError> {
    if snapshot.asset_id != asset.header.asset_id.as_str()
        || snapshot.asset_format_version != asset.header.format_version
        || snapshot.asset_compiler_compatibility_version
            != asset.header.compiler_compatibility_version
    {
        return Err(DialogueError::AssetMismatch {
            expected_asset_id: snapshot.asset_id.clone(),
            actual_asset_id: asset.header.asset_id.as_str().to_owned(),
            expected_format_version: snapshot.asset_format_version,
            actual_format_version: asset.header.format_version,
            expected_compiler_compatibility_version: snapshot.asset_compiler_compatibility_version,
            actual_compiler_compatibility_version: asset.header.compiler_compatibility_version,
        });
    }

    Ok(())
}

fn restore_frames(
    asset: AssetView<'_>,
    snapshots: &[DialogueSessionFrameSnapshot],
) -> Result<Vec<StatementFrame>, DialogueError> {
    snapshots
        .iter()
        .map(|snapshot| {
            let range = statement_range(snapshot.range);
            asset.statement_range(range)?;
            let next_statement = StatementIndex::new(snapshot.next_statement);
            validate_statement_pointer("continuation next statement", range, next_statement)?;

            Ok(StatementFrame {
                range,
                next_statement,
            })
        })
        .collect()
}

fn restore_pending_prompt(
    asset: AssetView<'_>,
    snapshot: Option<&DialogueSessionPendingPromptSnapshot>,
    previous_prompt_choices: &[ChoiceId],
    ended: bool,
) -> Result<Option<PendingPrompt>, DialogueError> {
    let Some(snapshot) = snapshot else {
        return Ok(None);
    };
    if ended {
        return Err(invalid_snapshot(
            "ended sessions cannot have a pending prompt",
        ));
    }
    if snapshot.choices.is_empty() {
        return Err(invalid_snapshot("pending prompt has no choices"));
    }

    let statement_index = StatementIndex::new(snapshot.statement);
    let statement = asset.statement_at(statement_index)?;
    let CompiledStatementKind::Prompt { choices, .. } = &statement.kind else {
        return Err(invalid_snapshot(format!(
            "pending prompt statement {} is not a prompt",
            statement_index.as_u32()
        )));
    };
    let compiled_choices = asset.choices(*choices)?;
    if compiled_choices.len() != snapshot.choices.len() {
        return Err(invalid_snapshot(format!(
            "pending prompt choice count {} does not match compiled prompt choice count {}",
            snapshot.choices.len(),
            compiled_choices.len()
        )));
    }

    let mut pending_choices = Vec::with_capacity(snapshot.choices.len());
    for (compiled_choice, snapshot_choice) in compiled_choices.iter().zip(&snapshot.choices) {
        let choice_id = choice_id(&snapshot_choice.id)?;
        if compiled_choice.id != choice_id {
            return Err(invalid_snapshot(format!(
                "pending prompt choice `{}` does not match compiled choice `{}`",
                snapshot_choice.id, compiled_choice.id
            )));
        }
        pending_choices.push(PendingPromptChoice {
            id: choice_id,
            target: compiled_choice.target.clone(),
            is_available: snapshot_choice.is_available,
            unavailable_reason: snapshot_choice.unavailable_reason.clone(),
        });
    }

    let pending_choice_ids = pending_choices
        .iter()
        .map(|choice| choice.id.clone())
        .collect::<Vec<_>>();
    if pending_choice_ids != previous_prompt_choices {
        return Err(invalid_snapshot(
            "pending prompt choices must match previous prompt choices",
        ));
    }

    Ok(Some(PendingPrompt {
        statement: statement_index,
        choices: pending_choices,
    }))
}

fn restore_choice_ids(
    asset: AssetView<'_>,
    field: &'static str,
    values: &[String],
) -> Result<Vec<ChoiceId>, DialogueError> {
    values
        .iter()
        .map(|value| {
            let choice_id = choice_id(value)?;
            asset.choice_by_id(&choice_id)?;
            Ok(choice_id)
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| match error {
            DialogueError::MalformedCompiledAsset { reason } => invalid_snapshot(format!(
                "{field} reference invalid compiled choice: {reason}"
            )),
            other => other,
        })
}

fn restore_effects(
    snapshots: &[DialogueEffectRequestSnapshot],
) -> Result<Vec<DialogueEffectRequest>, DialogueError> {
    snapshots.iter().map(restore_effect).collect()
}

fn restore_effect(
    snapshot: &DialogueEffectRequestSnapshot,
) -> Result<DialogueEffectRequest, DialogueError> {
    Ok(DialogueEffectRequest {
        id: effect_id(&snapshot.id)?,
        mode: restore_effect_mode(snapshot.mode),
        function: snapshot.function.clone(),
        args: snapshot.args.iter().map(restore_effect_argument).collect(),
        source_span: restore_source_span(&snapshot.source_span)?,
    })
}

fn restore_locale(value: Option<&str>) -> Result<Option<LocaleId>, DialogueError> {
    value.map(LocaleId::new).transpose().map_err(core_error)
}

fn frame_snapshot(frame: &StatementFrame) -> DialogueSessionFrameSnapshot {
    DialogueSessionFrameSnapshot {
        range: range_snapshot(frame.range),
        next_statement: frame.next_statement.as_u32(),
    }
}

fn pending_prompt_snapshot(prompt: &PendingPrompt) -> DialogueSessionPendingPromptSnapshot {
    DialogueSessionPendingPromptSnapshot {
        statement: prompt.statement.as_u32(),
        choices: prompt
            .choices
            .iter()
            .map(|choice| DialogueSessionPendingChoiceSnapshot {
                id: choice.id.as_str().to_owned(),
                is_available: choice.is_available,
                unavailable_reason: choice.unavailable_reason.clone(),
            })
            .collect(),
    }
}

fn choice_ids_snapshot(choice_ids: &[ChoiceId]) -> Vec<String> {
    choice_ids
        .iter()
        .map(|choice_id| choice_id.as_str().to_owned())
        .collect()
}

fn effect_request_snapshot(effect: &DialogueEffectRequest) -> DialogueEffectRequestSnapshot {
    DialogueEffectRequestSnapshot {
        id: effect.id.as_str().to_owned(),
        mode: effect_mode_snapshot(effect.mode),
        function: effect.function.clone(),
        args: effect.args.iter().map(effect_argument_snapshot).collect(),
        source_span: source_span_snapshot(&effect.source_span),
    }
}

fn range_snapshot(range: StatementRange) -> DialogueSessionRangeSnapshot {
    DialogueSessionRangeSnapshot {
        start: range.start.as_u32(),
        len: range.len,
    }
}

fn statement_range(snapshot: DialogueSessionRangeSnapshot) -> StatementRange {
    StatementRange::new(StatementIndex::new(snapshot.start), snapshot.len)
}

fn validate_statement_pointer(
    field: &'static str,
    range: StatementRange,
    pointer: StatementIndex,
) -> Result<(), DialogueError> {
    let start = range.start.as_u32();
    let end = start
        .checked_add(range.len)
        .ok_or_else(|| invalid_snapshot(format!("{field} range overflows u32")))?;
    let pointer = pointer.as_u32();

    if pointer < start || pointer > end {
        return Err(invalid_snapshot(format!(
            "{field} {pointer} is outside active range {start}..={end}"
        )));
    }

    Ok(())
}

fn effect_mode_snapshot(mode: DialogueEffectMode) -> DialogueEffectModeSnapshot {
    match mode {
        DialogueEffectMode::Deferred => DialogueEffectModeSnapshot::Deferred,
        DialogueEffectMode::Immediate => DialogueEffectModeSnapshot::Immediate,
        DialogueEffectMode::Blocking => DialogueEffectModeSnapshot::Blocking,
    }
}

fn restore_effect_mode(mode: DialogueEffectModeSnapshot) -> DialogueEffectMode {
    match mode {
        DialogueEffectModeSnapshot::Deferred => DialogueEffectMode::Deferred,
        DialogueEffectModeSnapshot::Immediate => DialogueEffectMode::Immediate,
        DialogueEffectModeSnapshot::Blocking => DialogueEffectMode::Blocking,
    }
}

fn effect_argument_snapshot(argument: &DialogueEffectArgument) -> DialogueEffectArgumentSnapshot {
    match argument {
        DialogueEffectArgument::Identifier(value) => {
            DialogueEffectArgumentSnapshot::Identifier(value.clone())
        }
        DialogueEffectArgument::String(value) => {
            DialogueEffectArgumentSnapshot::String(value.clone())
        }
        DialogueEffectArgument::Integer(value) => DialogueEffectArgumentSnapshot::Integer(*value),
        DialogueEffectArgument::Float(value) => DialogueEffectArgumentSnapshot::Float(*value),
        DialogueEffectArgument::Boolean(value) => DialogueEffectArgumentSnapshot::Boolean(*value),
    }
}

fn restore_effect_argument(argument: &DialogueEffectArgumentSnapshot) -> DialogueEffectArgument {
    match argument {
        DialogueEffectArgumentSnapshot::Identifier(value) => {
            DialogueEffectArgument::Identifier(value.clone())
        }
        DialogueEffectArgumentSnapshot::String(value) => {
            DialogueEffectArgument::String(value.clone())
        }
        DialogueEffectArgumentSnapshot::Integer(value) => DialogueEffectArgument::Integer(*value),
        DialogueEffectArgumentSnapshot::Float(value) => DialogueEffectArgument::Float(*value),
        DialogueEffectArgumentSnapshot::Boolean(value) => DialogueEffectArgument::Boolean(*value),
    }
}

fn source_span_snapshot(span: &SourceSpan) -> DialogueSourceSpanSnapshot {
    DialogueSourceSpanSnapshot {
        file: span.file.clone(),
        start: source_position_snapshot(span.start),
        end: span.end.map(source_position_snapshot),
    }
}

fn source_position_snapshot(position: SourcePosition) -> DialogueSourcePositionSnapshot {
    DialogueSourcePositionSnapshot {
        line: position.line(),
        column: position.column(),
    }
}

fn restore_source_span(snapshot: &DialogueSourceSpanSnapshot) -> Result<SourceSpan, DialogueError> {
    Ok(SourceSpan {
        file: snapshot.file.clone(),
        start: restore_source_position(snapshot.start)?,
        end: snapshot.end.map(restore_source_position).transpose()?,
    })
}

fn restore_source_position(
    snapshot: DialogueSourcePositionSnapshot,
) -> Result<SourcePosition, DialogueError> {
    SourcePosition::new(snapshot.line, snapshot.column).map_err(core_error)
}

fn choice_id(value: &str) -> Result<ChoiceId, DialogueError> {
    ChoiceId::new(value).map_err(core_error)
}

fn effect_id(value: &str) -> Result<EffectId, DialogueError> {
    EffectId::new(value).map_err(core_error)
}

fn core_error(error: impl std::fmt::Display) -> DialogueError {
    invalid_snapshot(error.to_string())
}

fn invalid_snapshot(reason: impl Into<String>) -> DialogueError {
    DialogueError::InvalidSessionSnapshot {
        reason: reason.into(),
    }
}
