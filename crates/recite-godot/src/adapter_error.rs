use recite_core::CompiledAssetDecodeError;
use recite_runtime::DialogueError;

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdapterErrorKind {
    AssetLoadOrDecode,
    StaleOrIncompatibleAsset,
    NoActiveSession,
    SessionAlreadyActive,
    UnknownStartBlock,
    InvalidChoice,
    StaleChoice,
    UnavailableChoice,
    MissingConditionHandler,
    ConditionEvaluationFailed,
    InvalidConditionResult,
    EffectAcknowledgement,
    SaveLoadIncompatibility,
    Localisation,
    /// Dialogue content caused a runtime fault (e.g. an infinite divert loop
    /// that exceeded the traversal safety cap). This is a dialogue authoring
    /// bug, not an asset compatibility problem.
    DialogueFault,
}

impl AdapterErrorKind {
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::AssetLoadOrDecode => "asset_load_or_decode_error",
            Self::StaleOrIncompatibleAsset => "stale_or_incompatible_asset_error",
            Self::NoActiveSession => "no_active_session_error",
            Self::SessionAlreadyActive => "session_already_active_error",
            Self::UnknownStartBlock => "unknown_start_block_error",
            Self::InvalidChoice => "invalid_choice_error",
            Self::StaleChoice => "stale_choice_error",
            Self::UnavailableChoice => "unavailable_choice_error",
            Self::MissingConditionHandler => "missing_condition_handler_error",
            Self::ConditionEvaluationFailed => "condition_evaluation_error",
            Self::InvalidConditionResult => "invalid_condition_result_error",
            Self::EffectAcknowledgement => "effect_acknowledgement_error",
            Self::SaveLoadIncompatibility => "save_load_incompatibility_error",
            Self::Localisation => "localisation_error",
            Self::DialogueFault => "dialogue_fault_error",
        }
    }

    pub(crate) fn from_code(code: &str) -> Option<Self> {
        match code {
            "asset_load_or_decode_error" => Some(Self::AssetLoadOrDecode),
            "stale_or_incompatible_asset_error" => Some(Self::StaleOrIncompatibleAsset),
            "no_active_session_error" => Some(Self::NoActiveSession),
            "session_already_active_error" => Some(Self::SessionAlreadyActive),
            "unknown_start_block_error" => Some(Self::UnknownStartBlock),
            "invalid_choice_error" => Some(Self::InvalidChoice),
            "stale_choice_error" => Some(Self::StaleChoice),
            "unavailable_choice_error" => Some(Self::UnavailableChoice),
            "missing_condition_handler_error" => Some(Self::MissingConditionHandler),
            "condition_evaluation_error" => Some(Self::ConditionEvaluationFailed),
            "invalid_condition_result_error" => Some(Self::InvalidConditionResult),
            "effect_acknowledgement_error" => Some(Self::EffectAcknowledgement),
            "save_load_incompatibility_error" => Some(Self::SaveLoadIncompatibility),
            "localisation_error" => Some(Self::Localisation),
            "dialogue_fault_error" => Some(Self::DialogueFault),
            _ => None,
        }
    }
}

/// Encodes a condition error for propagation through [`recite_runtime::ConditionEvaluationError`].
///
/// `ConditionEvaluationError` carries only a `String`, so we embed the kind
/// code as a `[code]` prefix. [`decode_condition_error_kind`] recovers it on
/// the other side in `From<DialogueError>`.
pub(crate) fn encode_condition_error(error: &AdapterError) -> String {
    format!("[{}] {}", error.kind().code(), error)
}

/// Recovers an [`AdapterErrorKind`] from a reason string written by
/// [`encode_condition_error`].
pub(crate) fn decode_condition_error_kind(reason: &str) -> Option<AdapterErrorKind> {
    let rest = reason.strip_prefix('[')?;
    let (code, _) = rest.split_once(']')?;
    AdapterErrorKind::from_code(code)
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("{code}: {message}")]
pub struct AdapterError {
    kind: AdapterErrorKind,
    code: &'static str,
    message: String,
}

impl AdapterError {
    #[must_use]
    pub fn new(kind: AdapterErrorKind) -> Self {
        let code = kind.code();
        Self {
            kind,
            code,
            message: code.to_owned(),
        }
    }

    #[must_use]
    pub fn with_detail(kind: AdapterErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            code: kind.code(),
            message: message.into(),
        }
    }

    #[must_use]
    pub fn kind(&self) -> AdapterErrorKind {
        self.kind
    }

    #[must_use]
    pub fn code(&self) -> &'static str {
        self.code
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

pub type AdapterResult<T> = Result<T, AdapterError>;

impl From<CompiledAssetDecodeError> for AdapterError {
    fn from(error: CompiledAssetDecodeError) -> Self {
        AdapterError::with_detail(AdapterErrorKind::AssetLoadOrDecode, error.to_string())
    }
}

impl From<DialogueError> for AdapterError {
    fn from(error: DialogueError) -> Self {
        let kind = match error {
            DialogueError::UnknownBlock { .. } => AdapterErrorKind::UnknownStartBlock,
            DialogueError::UnsupportedCompiledFormat { .. }
            | DialogueError::AssetMismatch { .. }
            | DialogueError::AssetContentMismatch { .. } => {
                AdapterErrorKind::StaleOrIncompatibleAsset
            }
            DialogueError::TraversalLimitExceeded { .. } => AdapterErrorKind::DialogueFault,
            DialogueError::MalformedCompiledAsset { .. } => AdapterErrorKind::AssetLoadOrDecode,
            DialogueError::InvalidChoice { .. } => AdapterErrorKind::InvalidChoice,
            DialogueError::NoPromptPending { .. } | DialogueError::PromptPending { .. } => {
                AdapterErrorKind::StaleChoice
            }
            DialogueError::UnavailableChoice { .. } => AdapterErrorKind::UnavailableChoice,
            DialogueError::WrongEffectAcknowledgement { .. }
            | DialogueError::NoEffectPending { .. }
            | DialogueError::EffectPending { .. } => AdapterErrorKind::EffectAcknowledgement,
            DialogueError::ConditionEvaluationFailed { ref reason, .. } => {
                decode_condition_error_kind(reason)
                    .unwrap_or(AdapterErrorKind::ConditionEvaluationFailed)
            }
            DialogueError::ConditionResultTypeMismatch { .. } => {
                AdapterErrorKind::InvalidConditionResult
            }
            DialogueError::ConditionDepthLimitExceeded { .. } => {
                AdapterErrorKind::ConditionEvaluationFailed
            }
            DialogueError::UnsupportedSessionSnapshotFormat { .. }
            | DialogueError::SessionSnapshotEncodeFailed { .. }
            | DialogueError::SessionSnapshotDecodeFailed { .. }
            | DialogueError::InvalidSessionSnapshot { .. } => {
                AdapterErrorKind::SaveLoadIncompatibility
            }
            DialogueError::SessionEnded => AdapterErrorKind::NoActiveSession,
        };
        AdapterError::with_detail(kind, error.to_string())
    }
}
