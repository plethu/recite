use recite_core::ChoiceId;

use crate::DialogueEffectMode;

/// Runtime error for deterministic traversal over compiled dialogue assets.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DialogueError {
    UnknownBlock {
        block: String,
    },
    UnsupportedCompiledFormat {
        format_version: u16,
        compiler_compatibility_version: u16,
    },
    AssetMismatch {
        expected_asset_id: String,
        actual_asset_id: String,
        expected_format_version: u16,
        actual_format_version: u16,
        expected_compiler_compatibility_version: u16,
        actual_compiler_compatibility_version: u16,
    },
    AssetContentMismatch {
        asset_id: String,
        reason: String,
    },
    MalformedCompiledAsset {
        reason: String,
    },
    UnsupportedStatement {
        kind: UnsupportedStatementKind,
    },
    UnsupportedEffectMode {
        mode: DialogueEffectMode,
    },
    PromptPending {
        choices: Vec<ChoiceId>,
    },
    NoPromptPending {
        choice: ChoiceId,
    },
    InvalidChoice {
        choice: ChoiceId,
        prompt_choices: Vec<ChoiceId>,
    },
    UnavailableChoice {
        choice: ChoiceId,
        reason: Option<String>,
    },
    ConditionEvaluationFailed {
        function: String,
        reason: String,
    },
    ConditionDepthLimitExceeded {
        limit: usize,
    },
    UnsupportedSessionSnapshotFormat {
        snapshot_format_version: u16,
    },
    SessionSnapshotEncodeFailed {
        reason: String,
    },
    SessionSnapshotDecodeFailed {
        reason: String,
    },
    InvalidSessionSnapshot {
        reason: String,
    },
    SessionEnded,
    TraversalLimitExceeded {
        limit: usize,
    },
}

impl std::fmt::Display for DialogueError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownBlock { block } => write!(formatter, "unknown block `{block}`"),
            Self::UnsupportedCompiledFormat {
                format_version,
                compiler_compatibility_version,
            } => write!(
                formatter,
                "unsupported compiled asset format {format_version} with compatibility version {compiler_compatibility_version}"
            ),
            Self::AssetMismatch {
                expected_asset_id,
                actual_asset_id,
                expected_format_version,
                actual_format_version,
                expected_compiler_compatibility_version,
                actual_compiler_compatibility_version,
            } => write!(
                formatter,
                "session is for asset `{expected_asset_id}` ({expected_format_version}/{expected_compiler_compatibility_version}) but got `{actual_asset_id}` ({actual_format_version}/{actual_compiler_compatibility_version})"
            ),
            Self::AssetContentMismatch { asset_id, reason } => {
                write!(
                    formatter,
                    "session is for a different compiled asset payload `{asset_id}`: {reason}"
                )
            }
            Self::MalformedCompiledAsset { reason } => {
                write!(formatter, "malformed compiled asset: {reason}")
            }
            Self::UnsupportedStatement { kind } => {
                write!(formatter, "runtime traversal does not support {kind} yet")
            }
            Self::UnsupportedEffectMode { mode } => {
                write!(
                    formatter,
                    "runtime traversal does not support {mode} effects yet"
                )
            }
            Self::PromptPending { .. } => {
                formatter.write_str("session is waiting for a choice selection")
            }
            Self::NoPromptPending { choice } => {
                write!(
                    formatter,
                    "choice `{choice}` was selected with no pending prompt"
                )
            }
            Self::InvalidChoice {
                choice,
                prompt_choices: _,
            } => write!(
                formatter,
                "choice `{choice}` is not available in the pending prompt"
            ),
            Self::UnavailableChoice { choice, reason } => {
                write!(formatter, "choice `{choice}` is unavailable")?;
                if let Some(reason) = reason {
                    write!(formatter, ": {reason}")?;
                }
                Ok(())
            }
            Self::ConditionEvaluationFailed { function, reason } => {
                write!(formatter, "condition `{function}` failed: {reason}")
            }
            Self::ConditionDepthLimitExceeded { limit } => {
                write!(
                    formatter,
                    "condition expression exceeded maximum evaluation depth {limit}"
                )
            }
            Self::UnsupportedSessionSnapshotFormat {
                snapshot_format_version,
            } => write!(
                formatter,
                "unsupported session snapshot format {snapshot_format_version}"
            ),
            Self::SessionSnapshotEncodeFailed { reason } => {
                write!(formatter, "failed to encode session snapshot: {reason}")
            }
            Self::SessionSnapshotDecodeFailed { reason } => {
                write!(formatter, "failed to decode session snapshot: {reason}")
            }
            Self::InvalidSessionSnapshot { reason } => {
                write!(formatter, "invalid session snapshot: {reason}")
            }
            Self::SessionEnded => formatter.write_str("session has already ended"),
            Self::TraversalLimitExceeded { limit } => {
                write!(
                    formatter,
                    "runtime traversal exceeded {limit} internal steps"
                )
            }
        }
    }
}

impl std::fmt::Display for DialogueEffectMode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Deferred => formatter.write_str("deferred"),
            Self::Immediate => formatter.write_str("immediate"),
            Self::Blocking => formatter.write_str("blocking"),
        }
    }
}

impl std::error::Error for DialogueError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnsupportedStatementKind {
    Match,
}

impl std::fmt::Display for UnsupportedStatementKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Match => formatter.write_str("match branches"),
        }
    }
}
