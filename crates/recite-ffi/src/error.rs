use std::cell::RefCell;
use std::ffi::CString;

use recite_runtime::DialogueError;

/// Stable C error codes. Matches the category table in docs/c-abi-boundary-design.md.
///
/// Add a new variant only when a new contract §12 category is introduced.
/// Never renumber existing variants — that breaks compiled host bindings.
#[repr(i32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReciteStatus {
    Ok = 0,
    Validation = -1,
    AssetLoadOrDecode = -2,
    StaleOrIncompatible = -3,
    SchemaMismatch = -4,
    NoActiveSession = -5,
    SessionAlreadyActive = -6,
    UnknownStartBlock = -7,
    InvalidChoice = -8,
    UnavailableChoice = -9,
    StaleChoice = -10,
    MissingConditionHandler = -11,
    ConditionEvaluation = -12,
    InvalidConditionResult = -13,
    EffectAcknowledgement = -14,
    RejectedRefresh = -15,
    SaveLoadIncompatibility = -16,
    Localisation = -17,
    MissingProjectionHandler = -18,
    ProjectionEvaluation = -19,
    InvalidProjectionResult = -20,
    InvalidHandle = -21,
    DialogueFault = -22,
}

impl TryFrom<i32> for ReciteStatus {
    type Error = ();
    fn try_from(code: i32) -> Result<Self, ()> {
        match code {
            0 => Ok(Self::Ok),
            -1 => Ok(Self::Validation),
            -2 => Ok(Self::AssetLoadOrDecode),
            -3 => Ok(Self::StaleOrIncompatible),
            -4 => Ok(Self::SchemaMismatch),
            -5 => Ok(Self::NoActiveSession),
            -6 => Ok(Self::SessionAlreadyActive),
            -7 => Ok(Self::UnknownStartBlock),
            -8 => Ok(Self::InvalidChoice),
            -9 => Ok(Self::UnavailableChoice),
            -10 => Ok(Self::StaleChoice),
            -11 => Ok(Self::MissingConditionHandler),
            -12 => Ok(Self::ConditionEvaluation),
            -13 => Ok(Self::InvalidConditionResult),
            -14 => Ok(Self::EffectAcknowledgement),
            -15 => Ok(Self::RejectedRefresh),
            -16 => Ok(Self::SaveLoadIncompatibility),
            -17 => Ok(Self::Localisation),
            -18 => Ok(Self::MissingProjectionHandler),
            -19 => Ok(Self::ProjectionEvaluation),
            -20 => Ok(Self::InvalidProjectionResult),
            -21 => Ok(Self::InvalidHandle),
            -22 => Ok(Self::DialogueFault),
            _ => Err(()),
        }
    }
}

/// Maps `DialogueError` to a `ReciteStatus`.
///
/// Mirrors the `From<DialogueError> for AdapterErrorKind` impl in
/// `recite-godot/src/adapter_error.rs`. If a new `DialogueError` variant is
/// added, both that impl and this function must be updated together.
impl From<DialogueError> for ReciteStatus {
    fn from(error: DialogueError) -> Self {
        match error {
            DialogueError::UnknownBlock { .. } => Self::UnknownStartBlock,
            DialogueError::UnsupportedCompiledFormat { .. }
            | DialogueError::AssetMismatch { .. }
            | DialogueError::AssetContentMismatch { .. } => Self::StaleOrIncompatible,
            DialogueError::MalformedCompiledAsset { .. } => Self::AssetLoadOrDecode,
            DialogueError::EffectPending { .. }
            | DialogueError::NoEffectPending { .. }
            | DialogueError::WrongEffectAcknowledgement { .. } => Self::EffectAcknowledgement,
            DialogueError::PromptPending { .. } | DialogueError::NoPromptPending { .. } => {
                Self::StaleChoice
            }
            DialogueError::InvalidChoice { .. } => Self::InvalidChoice,
            DialogueError::UnavailableChoice { .. } => Self::UnavailableChoice,
            DialogueError::ConditionEvaluationFailed { ref reason, .. } => {
                decode_condition_status(reason).unwrap_or(Self::ConditionEvaluation)
            }
            DialogueError::ConditionResultTypeMismatch { .. } => Self::InvalidConditionResult,
            DialogueError::ConditionDepthLimitExceeded { .. } => Self::ConditionEvaluation,
            DialogueError::UnsupportedSessionSnapshotFormat { .. }
            | DialogueError::SessionSnapshotEncodeFailed { .. }
            | DialogueError::SessionSnapshotDecodeFailed { .. }
            | DialogueError::InvalidSessionSnapshot { .. } => Self::SaveLoadIncompatibility,
            DialogueError::SessionEnded => Self::NoActiveSession,
            DialogueError::TraversalLimitExceeded { .. } => Self::DialogueFault,
        }
    }
}

/// Encodes a condition error status into the reason string so `From<DialogueError>`
/// can recover it after the runtime wraps it in `ConditionEvaluationFailed`.
pub(crate) fn encode_condition_status(status: ReciteStatus, message: &str) -> String {
    format!("[{}] {}", status as i32, message)
}

pub(crate) fn decode_condition_status(reason: &str) -> Option<ReciteStatus> {
    let rest = reason.strip_prefix('[')?;
    let (code_str, _) = rest.split_once(']')?;
    let code: i32 = code_str.parse().ok()?;
    ReciteStatus::try_from(code).ok()
}

thread_local! {
    static LAST_ERROR: RefCell<Option<CString>> = const { RefCell::new(None) };
}

/// Sets the thread-local error message. The stored `CString` is valid until the
/// next `recite-ffi` call on the same thread.
pub(crate) fn set_last_error(message: &str) {
    let cstring = CString::new(message.replace('\0', "?")).unwrap_or_default();
    LAST_ERROR.with(|cell| *cell.borrow_mut() = Some(cstring));
}

/// Returns a pointer to the last error message set on the current thread.
/// The pointer is valid until the next `recite-ffi` call on this thread.
/// Returns a pointer to a NUL-terminated empty string (never null) if no error
/// has been set, so callers can always safely pass the result to C string APIs.
#[unsafe(no_mangle)]
pub extern "C" fn recite_last_error_message() -> *const std::ffi::c_char {
    // Static empty string so we never return null.
    static EMPTY: &[u8] = b"\0";
    LAST_ERROR.with(|cell| {
        cell.borrow()
            .as_ref()
            .map_or(EMPTY.as_ptr().cast(), |s| s.as_ptr())
    })
}
