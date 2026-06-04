use recite_core::{
    AvailabilityReasonId, ChoiceId, CompiledSourceFile, ContentFingerprint, LocaleId,
    SchemaFingerprint, StatementIndex, StatementRange,
};
use serde::{Deserialize, Serialize};

use crate::DialogueSession;
use crate::event::{
    ChoiceAvailability, ChoiceAvailabilityReason, ChoiceAvailabilityReasonArg,
    ChoiceAvailabilityReasonOrigin, ChoiceAvailabilityReasonTree, DialogueEffectRequest,
};
use crate::session::{PendingEffect, PendingPrompt, StatementFrame};

pub const SESSION_SNAPSHOT_FORMAT_VERSION_V0: u16 = 0;
pub const SESSION_SNAPSHOT_FORMAT_VERSION_V1: u16 = 1;
pub const CURRENT_SESSION_SNAPSHOT_FORMAT_VERSION: u16 = SESSION_SNAPSHOT_FORMAT_VERSION_V1;

/// Versioned structural save data for a dialogue session.
///
/// Snapshots contain only compact runtime state and asset identity references.
/// They are not a tamper-proof proof that the state was produced by a previous
/// honest traversal. Hosts that treat save data as untrusted should
/// authenticate or encrypt encoded snapshots before restoring them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DialogueSessionSnapshot {
    pub snapshot_format_version: u16,
    pub asset_id: String,
    pub asset_format_version: u16,
    pub asset_compiler_compatibility_version: u16,
    pub compiler_version: String,
    pub source_map_id: String,
    pub schema_fingerprint: DialogueSchemaFingerprintSnapshot,
    pub sources: Vec<DialogueSessionSourceSnapshot>,
    pub current_block: u32,
    pub current_range: DialogueSessionRangeSnapshot,
    pub next_statement: u32,
    pub continuation_stack: Vec<DialogueSessionFrameSnapshot>,
    pub pending_prompt: Option<DialogueSessionPendingPromptSnapshot>,
    pub pending_effect: Option<DialogueSessionPendingEffectSnapshot>,
    pub previous_prompt_choices: Vec<String>,
    pub selected_choice_history: Vec<String>,
    pub deferred_effects: Vec<DialogueDeferredEffectSnapshot>,
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
    pub availability: DialogueChoiceAvailabilitySnapshot,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DialogueChoiceAvailabilitySnapshot {
    pub is_available: bool,
    pub primary_reason: Option<DialogueChoiceAvailabilityReasonSnapshot>,
    pub reason_tree: Option<DialogueChoiceAvailabilityReasonTreeSnapshot>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DialogueChoiceAvailabilityReasonSnapshot {
    pub id: String,
    pub source_text: String,
    pub origin: Option<DialogueChoiceAvailabilityReasonOriginSnapshot>,
    pub args: Vec<DialogueChoiceAvailabilityReasonArgSnapshot>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DialogueChoiceAvailabilityReasonOriginSnapshot {
    ConditionCall { function: String, args: Vec<String> },
    RequirementExpression { source_text: String },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DialogueChoiceAvailabilityReasonArgSnapshot {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DialogueChoiceAvailabilityReasonTreeSnapshot {
    All(Vec<DialogueChoiceAvailabilityReasonTreeSnapshot>),
    Any(Vec<DialogueChoiceAvailabilityReasonTreeSnapshot>),
    Reason(DialogueChoiceAvailabilityReasonSnapshot),
    RequirementSourceText(String),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DialogueSessionPendingEffectSnapshot {
    pub statement: u32,
    pub id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DialogueDeferredEffectSnapshot {
    pub id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DialogueSessionSourceSnapshot {
    pub path: String,
    pub fingerprint: DialogueContentFingerprintSnapshot,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DialogueContentFingerprintSnapshot {
    pub algorithm: String,
    pub digest: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DialogueSchemaFingerprintSnapshot {
    Fingerprint(DialogueContentFingerprintSnapshot),
    NoSchema,
}

/// Captures the current session as trusted structural save data.
///
/// The returned snapshot is intended to be stored by the host save system. It
/// should be authenticated by that host if users or external systems can modify
/// saves before restore.
#[must_use]
pub fn snapshot_session(session: &DialogueSession) -> DialogueSessionSnapshot {
    DialogueSessionSnapshot {
        snapshot_format_version: CURRENT_SESSION_SNAPSHOT_FORMAT_VERSION,
        asset_id: session.asset_id.as_str().to_owned(),
        asset_format_version: session.format_version,
        asset_compiler_compatibility_version: session.compiler_compatibility_version,
        compiler_version: session.compiler_version.as_str().to_owned(),
        source_map_id: session.source_map_id.as_str().to_owned(),
        schema_fingerprint: schema_fingerprint_snapshot(&session.schema_fingerprint),
        sources: session.sources.iter().map(source_snapshot).collect(),
        current_block: session.current_block.as_u32(),
        current_range: range_snapshot(session.current_range),
        next_statement: session.next_statement.as_u32(),
        continuation_stack: session
            .continuation_stack
            .iter()
            .map(frame_snapshot)
            .collect(),
        pending_prompt: session.pending_prompt.as_ref().map(pending_prompt_snapshot),
        pending_effect: session.pending_effect.as_ref().map(pending_effect_snapshot),
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
            .map(LocaleId::as_str)
            .map(str::to_owned),
        trace_counter: session.trace_counter,
        ended: session.ended,
    }
}

pub(crate) fn statement_range(snapshot: DialogueSessionRangeSnapshot) -> StatementRange {
    StatementRange::new(StatementIndex::new(snapshot.start), snapshot.len)
}

pub(crate) fn source_snapshot(source: &CompiledSourceFile) -> DialogueSessionSourceSnapshot {
    DialogueSessionSourceSnapshot {
        path: source.path.clone(),
        fingerprint: content_fingerprint_snapshot(&source.fingerprint),
    }
}

pub(crate) fn schema_fingerprint_snapshot(
    fingerprint: &SchemaFingerprint,
) -> DialogueSchemaFingerprintSnapshot {
    match fingerprint {
        SchemaFingerprint::Fingerprint(fingerprint) => {
            DialogueSchemaFingerprintSnapshot::Fingerprint(content_fingerprint_snapshot(
                fingerprint,
            ))
        }
        SchemaFingerprint::NoSchema => DialogueSchemaFingerprintSnapshot::NoSchema,
    }
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
                availability: availability_snapshot(&choice.availability),
            })
            .collect(),
    }
}

pub(crate) fn availability_snapshot(
    availability: &ChoiceAvailability,
) -> DialogueChoiceAvailabilitySnapshot {
    DialogueChoiceAvailabilitySnapshot {
        is_available: availability.is_available,
        primary_reason: availability.primary_reason.as_ref().map(reason_snapshot),
        reason_tree: availability.reason_tree.as_ref().map(reason_tree_snapshot),
    }
}

pub(crate) fn availability_from_snapshot(
    snapshot: DialogueChoiceAvailabilitySnapshot,
) -> Result<ChoiceAvailability, String> {
    Ok(ChoiceAvailability {
        is_available: snapshot.is_available,
        primary_reason: snapshot
            .primary_reason
            .map(reason_from_snapshot)
            .transpose()?,
        reason_tree: snapshot
            .reason_tree
            .map(reason_tree_from_snapshot)
            .transpose()?,
    })
}

fn reason_snapshot(reason: &ChoiceAvailabilityReason) -> DialogueChoiceAvailabilityReasonSnapshot {
    DialogueChoiceAvailabilityReasonSnapshot {
        id: reason.id.as_str().to_owned(),
        source_text: reason.source_text.clone(),
        origin: reason.origin.as_ref().map(reason_origin_snapshot),
        args: reason
            .args
            .iter()
            .map(|arg| DialogueChoiceAvailabilityReasonArgSnapshot {
                name: arg.name.clone(),
                value: arg.value.clone(),
            })
            .collect(),
    }
}

fn reason_from_snapshot(
    snapshot: DialogueChoiceAvailabilityReasonSnapshot,
) -> Result<ChoiceAvailabilityReason, String> {
    Ok(ChoiceAvailabilityReason {
        id: AvailabilityReasonId::new(snapshot.id).map_err(|error| error.to_string())?,
        source_text: snapshot.source_text,
        origin: snapshot.origin.map(reason_origin_from_snapshot),
        args: snapshot
            .args
            .into_iter()
            .map(|arg| ChoiceAvailabilityReasonArg {
                name: arg.name,
                value: arg.value,
            })
            .collect(),
    })
}

fn reason_origin_snapshot(
    origin: &ChoiceAvailabilityReasonOrigin,
) -> DialogueChoiceAvailabilityReasonOriginSnapshot {
    match origin {
        ChoiceAvailabilityReasonOrigin::ConditionCall { function, args } => {
            DialogueChoiceAvailabilityReasonOriginSnapshot::ConditionCall {
                function: function.clone(),
                args: args.clone(),
            }
        }
        ChoiceAvailabilityReasonOrigin::RequirementExpression { source_text } => {
            DialogueChoiceAvailabilityReasonOriginSnapshot::RequirementExpression {
                source_text: source_text.clone(),
            }
        }
    }
}

fn reason_origin_from_snapshot(
    snapshot: DialogueChoiceAvailabilityReasonOriginSnapshot,
) -> ChoiceAvailabilityReasonOrigin {
    match snapshot {
        DialogueChoiceAvailabilityReasonOriginSnapshot::ConditionCall { function, args } => {
            ChoiceAvailabilityReasonOrigin::ConditionCall { function, args }
        }
        DialogueChoiceAvailabilityReasonOriginSnapshot::RequirementExpression { source_text } => {
            ChoiceAvailabilityReasonOrigin::RequirementExpression { source_text }
        }
    }
}

fn reason_tree_snapshot(
    tree: &ChoiceAvailabilityReasonTree,
) -> DialogueChoiceAvailabilityReasonTreeSnapshot {
    match tree {
        ChoiceAvailabilityReasonTree::All(children) => {
            DialogueChoiceAvailabilityReasonTreeSnapshot::All(
                children.iter().map(reason_tree_snapshot).collect(),
            )
        }
        ChoiceAvailabilityReasonTree::Any(children) => {
            DialogueChoiceAvailabilityReasonTreeSnapshot::Any(
                children.iter().map(reason_tree_snapshot).collect(),
            )
        }
        ChoiceAvailabilityReasonTree::Reason(reason) => {
            DialogueChoiceAvailabilityReasonTreeSnapshot::Reason(reason_snapshot(reason))
        }
        ChoiceAvailabilityReasonTree::RequirementSourceText(text) => {
            DialogueChoiceAvailabilityReasonTreeSnapshot::RequirementSourceText(text.clone())
        }
    }
}

fn reason_tree_from_snapshot(
    snapshot: DialogueChoiceAvailabilityReasonTreeSnapshot,
) -> Result<ChoiceAvailabilityReasonTree, String> {
    match snapshot {
        DialogueChoiceAvailabilityReasonTreeSnapshot::All(children) => {
            Ok(ChoiceAvailabilityReasonTree::All(
                children
                    .into_iter()
                    .map(reason_tree_from_snapshot)
                    .collect::<Result<Vec<_>, _>>()?,
            ))
        }
        DialogueChoiceAvailabilityReasonTreeSnapshot::Any(children) => {
            Ok(ChoiceAvailabilityReasonTree::Any(
                children
                    .into_iter()
                    .map(reason_tree_from_snapshot)
                    .collect::<Result<Vec<_>, _>>()?,
            ))
        }
        DialogueChoiceAvailabilityReasonTreeSnapshot::Reason(reason) => Ok(
            ChoiceAvailabilityReasonTree::Reason(reason_from_snapshot(reason)?),
        ),
        DialogueChoiceAvailabilityReasonTreeSnapshot::RequirementSourceText(text) => {
            Ok(ChoiceAvailabilityReasonTree::RequirementSourceText(text))
        }
    }
}

fn pending_effect_snapshot(effect: &PendingEffect) -> DialogueSessionPendingEffectSnapshot {
    DialogueSessionPendingEffectSnapshot {
        statement: effect.statement.as_u32(),
        id: effect.request.id.as_str().to_owned(),
    }
}

fn choice_ids_snapshot(choice_ids: &[ChoiceId]) -> Vec<String> {
    choice_ids
        .iter()
        .map(|choice_id| choice_id.as_str().to_owned())
        .collect()
}

fn effect_request_snapshot(effect: &DialogueEffectRequest) -> DialogueDeferredEffectSnapshot {
    DialogueDeferredEffectSnapshot {
        id: effect.id.as_str().to_owned(),
    }
}

fn range_snapshot(range: StatementRange) -> DialogueSessionRangeSnapshot {
    DialogueSessionRangeSnapshot {
        start: range.start.as_u32(),
        len: range.len,
    }
}

fn content_fingerprint_snapshot(
    fingerprint: &ContentFingerprint,
) -> DialogueContentFingerprintSnapshot {
    DialogueContentFingerprintSnapshot {
        algorithm: fingerprint.algorithm().as_str().to_owned(),
        digest: fingerprint.digest().as_bytes().to_vec(),
    }
}
