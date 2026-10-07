use std::cell::RefCell;
use std::ffi::CString;

use recite_adapter::{AdapterError, AdapterErrorKind};

/// Stable C error codes. The generated header defines the public names.
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

impl From<AdapterErrorKind> for ReciteStatus {
    fn from(kind: AdapterErrorKind) -> Self {
        match kind {
            AdapterErrorKind::Validation => Self::Validation,
            AdapterErrorKind::AssetLoadOrDecode => Self::AssetLoadOrDecode,
            AdapterErrorKind::StaleOrIncompatibleAsset => Self::StaleOrIncompatible,
            AdapterErrorKind::SchemaMismatch => Self::SchemaMismatch,
            AdapterErrorKind::NoActiveSession => Self::NoActiveSession,
            AdapterErrorKind::SessionAlreadyActive => Self::SessionAlreadyActive,
            AdapterErrorKind::UnknownStartBlock => Self::UnknownStartBlock,
            AdapterErrorKind::InvalidChoice => Self::InvalidChoice,
            AdapterErrorKind::StaleChoice => Self::StaleChoice,
            AdapterErrorKind::UnavailableChoice => Self::UnavailableChoice,
            AdapterErrorKind::MissingConditionHandler => Self::MissingConditionHandler,
            AdapterErrorKind::ConditionEvaluationFailed => Self::ConditionEvaluation,
            AdapterErrorKind::InvalidConditionResult => Self::InvalidConditionResult,
            AdapterErrorKind::EffectAcknowledgement => Self::EffectAcknowledgement,
            AdapterErrorKind::SaveLoadIncompatibility => Self::SaveLoadIncompatibility,
            AdapterErrorKind::RejectedChangedAssetRefresh => Self::RejectedRefresh,
            AdapterErrorKind::Localisation => Self::Localisation,
            AdapterErrorKind::DialogueFault => Self::DialogueFault,
            _ => Self::DialogueFault,
        }
    }
}

impl From<AdapterError> for ReciteStatus {
    fn from(error: AdapterError) -> Self {
        Self::from(error.kind())
    }
}

thread_local! {
    static LAST_ERROR: RefCell<Option<CString>> = const { RefCell::new(None) };
}

/// Sets the thread-local error message. The stored `CString` is valid until the
/// next `recite-ffi` call on the same thread.
pub(crate) fn set_last_error(message: &str) {
    let message = message.replace('\0', "?");
    debug_assert!(
        !message.as_bytes().contains(&0),
        "interior NULs are replaced before constructing CString"
    );
    let cstring = match CString::new(message) {
        Ok(cstring) => cstring,
        Err(error) => unreachable!("interior NULs were not replaced: {error}"),
    };
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
