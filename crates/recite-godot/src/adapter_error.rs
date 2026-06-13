use recite_core::CompiledAssetDecodeError;
use recite_runtime::DialogueError;

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
        }
    }
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
            | DialogueError::AssetContentMismatch { .. }
            | DialogueError::TraversalLimitExceeded { .. } => {
                AdapterErrorKind::StaleOrIncompatibleAsset
            }
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
                if reason.contains(AdapterErrorKind::MissingConditionHandler.code()) {
                    AdapterErrorKind::MissingConditionHandler
                } else {
                    AdapterErrorKind::ConditionEvaluationFailed
                }
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
