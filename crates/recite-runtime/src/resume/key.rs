//! Revision-independent keys for compiled statements.
//!
//! The borrowed key types serialize exactly like their owned snapshot
//! counterparts, so encoding a save does not copy compiled rows.

use recite_core::{
    LineId,
    compiled::{
        CompiledChoice, CompiledDivertTarget, CompiledEffectMode, CompiledStatementKind,
        StatementIndex,
    },
};
use serde::{Serialize, Serializer};

use crate::DialogueError;
use crate::session_snapshot::{
    DialogueEffectModeSnapshot, DialogueLandmarkSnapshot, DialogueStatementSignatureSnapshot,
};
use crate::traversal::AssetView;

use super::digest::{Digest, arguments_digest, call_digest, condition_digest};

/// How a statement is recognised in another revision of the same project.
pub(crate) enum StatementKey<'a> {
    /// Lines and prompts carry authored IDs that stay stable across edits.
    Landmark(Landmark<'a>),
    /// Other statements are recognised by what they do.
    Signature(Signature<'a>),
}

/// Mirrors [`DialogueLandmarkSnapshot`].
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Landmark<'a> {
    Line {
        #[serde(serialize_with = "line_id")]
        id: &'a LineId,
    },
    Prompt {
        #[serde(serialize_with = "optional_line_id")]
        line: Option<&'a LineId>,
        #[serde(serialize_with = "choice_ids")]
        choices: &'a [CompiledChoice],
    },
}

/// Mirrors [`DialogueStatementSignatureSnapshot`].
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Signature<'a> {
    Divert {
        target: Option<&'a str>,
    },
    End,
    If {
        #[serde(serialize_with = "digest")]
        condition: Digest,
    },
    Match {
        #[serde(serialize_with = "digest")]
        scrutinee: Digest,
    },
    Effect {
        mode: DialogueEffectModeSnapshot,
        function: &'a str,
        #[serde(serialize_with = "digest")]
        arguments: Digest,
    },
}

pub(crate) fn statement_key(
    asset: AssetView<'_>,
    index: StatementIndex,
) -> Result<StatementKey<'_>, DialogueError> {
    let statement = asset.statement_at(index)?;
    Ok(match &statement.kind {
        CompiledStatementKind::Line(line) => StatementKey::Landmark(Landmark::Line {
            id: &asset.line_at(*line)?.id,
        }),
        CompiledStatementKind::Prompt { line, choices } => {
            StatementKey::Landmark(Landmark::Prompt {
                line: line
                    .map(|line| asset.line_at(line).map(|line| &line.id))
                    .transpose()?,
                choices: asset.choices(*choices)?,
            })
        }
        CompiledStatementKind::Divert(target) => StatementKey::Signature(Signature::Divert {
            target: match target {
                CompiledDivertTarget::Block(block) => Some(asset.block_at(*block)?.id.as_str()),
                CompiledDivertTarget::End => None,
            },
        }),
        CompiledStatementKind::End => StatementKey::Signature(Signature::End),
        CompiledStatementKind::If { condition, .. } => StatementKey::Signature(Signature::If {
            condition: condition_digest(condition),
        }),
        CompiledStatementKind::Match { scrutinee, .. } => {
            StatementKey::Signature(Signature::Match {
                scrutinee: call_digest(&scrutinee.function, &scrutinee.args),
            })
        }
        CompiledStatementKind::Effect(effect) => {
            let effect = asset.effect_at(*effect)?;
            StatementKey::Signature(Signature::Effect {
                mode: effect_mode_snapshot(effect.mode),
                function: &effect.function,
                arguments: arguments_digest(&effect.args),
            })
        }
    })
}

impl Landmark<'_> {
    pub(crate) fn to_snapshot(self) -> DialogueLandmarkSnapshot {
        match self {
            Self::Line { id } => DialogueLandmarkSnapshot::Line {
                id: id.as_str().to_owned(),
            },
            Self::Prompt { line, choices } => DialogueLandmarkSnapshot::Prompt {
                line: line.map(|line| line.as_str().to_owned()),
                choices: choices
                    .iter()
                    .map(|choice| choice.id.as_str().to_owned())
                    .collect(),
            },
        }
    }

    /// Whether this statement is the saved landmark. A prompt matches when it
    /// keeps its saved prompt line or any saved choice, so adding, removing
    /// or rewording choices still finds it.
    pub(crate) fn matches(self, saved: &DialogueLandmarkSnapshot) -> bool {
        match (self, saved) {
            (Self::Line { id }, DialogueLandmarkSnapshot::Line { id: saved }) => {
                id.as_str() == saved
            }
            (
                Self::Prompt { line, choices },
                DialogueLandmarkSnapshot::Prompt {
                    line: saved_line,
                    choices: saved_choices,
                },
            ) => {
                let same_line = matches!(
                    (line, saved_line.as_deref()),
                    (Some(line), Some(saved)) if line.as_str() == saved
                );
                same_line
                    || choices.iter().any(|choice| {
                        saved_choices
                            .iter()
                            .any(|saved| saved == choice.id.as_str())
                    })
            }
            _ => false,
        }
    }
}

impl Signature<'_> {
    pub(crate) fn to_snapshot(&self) -> DialogueStatementSignatureSnapshot {
        match self {
            Self::Divert { target } => DialogueStatementSignatureSnapshot::Divert {
                target: target.map(str::to_owned),
            },
            Self::End => DialogueStatementSignatureSnapshot::End,
            Self::If { condition } => DialogueStatementSignatureSnapshot::If {
                condition: condition.to_vec(),
            },
            Self::Match { scrutinee } => DialogueStatementSignatureSnapshot::Match {
                scrutinee: scrutinee.to_vec(),
            },
            Self::Effect {
                mode,
                function,
                arguments,
            } => DialogueStatementSignatureSnapshot::Effect {
                mode: *mode,
                function: (*function).to_owned(),
                arguments: arguments.to_vec(),
            },
        }
    }

    pub(crate) fn matches(&self, saved: &DialogueStatementSignatureSnapshot) -> bool {
        match (self, saved) {
            (
                Self::Divert { target },
                DialogueStatementSignatureSnapshot::Divert { target: saved },
            ) => *target == saved.as_deref(),
            (Self::End, DialogueStatementSignatureSnapshot::End) => true,
            (
                Self::If { condition },
                DialogueStatementSignatureSnapshot::If { condition: saved },
            ) => condition.as_slice() == saved.as_slice(),
            (
                Self::Match { scrutinee },
                DialogueStatementSignatureSnapshot::Match { scrutinee: saved },
            ) => scrutinee.as_slice() == saved.as_slice(),
            (
                Self::Effect {
                    mode,
                    function,
                    arguments,
                },
                DialogueStatementSignatureSnapshot::Effect {
                    mode: saved_mode,
                    function: saved_function,
                    arguments: saved_arguments,
                },
            ) => {
                mode == saved_mode
                    && function == saved_function
                    && arguments.as_slice() == saved_arguments.as_slice()
            }
            _ => false,
        }
    }
}

pub(crate) fn effect_mode_snapshot(mode: CompiledEffectMode) -> DialogueEffectModeSnapshot {
    match mode {
        CompiledEffectMode::Deferred => DialogueEffectModeSnapshot::Deferred,
        CompiledEffectMode::Immediate => DialogueEffectModeSnapshot::Immediate,
        CompiledEffectMode::Blocking => DialogueEffectModeSnapshot::Blocking,
    }
}

fn line_id<S: Serializer>(id: &&LineId, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(id.as_str())
}

fn optional_line_id<S: Serializer>(id: &Option<&LineId>, serializer: S) -> Result<S::Ok, S::Error> {
    id.map(LineId::as_str).serialize(serializer)
}

fn choice_ids<S: Serializer>(
    choices: &&[CompiledChoice],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.collect_seq(choices.iter().map(|choice| choice.id.as_str()))
}

pub(crate) fn digest<S: Serializer>(digest: &Digest, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.collect_seq(digest.iter())
}
