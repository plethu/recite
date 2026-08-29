use std::collections::BTreeMap;
use std::ffi::{CStr, c_char};
use std::sync::{Mutex, OnceLock};
use std::thread::{self, ThreadId};

use recite_core::CompiledDialogue;
use recite_runtime::{DialogueError, DialogueSession, DialogueSessionOptions, InterpolationValues};

use crate::condition::ConditionEntry;
use crate::error::{ReciteStatus, set_last_error};
use crate::locale::FfiLocaleProvider;

mod begin;
mod choice;
mod create;
mod drain;
mod effect;
mod locale;
mod restore;
mod snapshot;
mod start;
mod values;

pub use begin::{recite_session_begin, recite_session_register_condition};
pub use choice::recite_session_choose;
pub use create::{recite_session_create, recite_session_create_with_values};
pub(crate) use drain::{drain_after_event, drain_restored, drain_to_batch};
pub use effect::recite_session_acknowledge_effect;
pub(crate) use locale::set_locale_variant_value;
pub use locale::{
    recite_session_clear_locale_provider, recite_session_set_locale_provider,
    recite_session_set_locale_variant,
};
pub use restore::recite_session_restore_with_values_and_locale_provider;
pub use restore::recite_session_restore_with_values_and_locale_provider_and_variant;
pub use restore::{recite_session_restore, recite_session_restore_with_values};
pub use snapshot::recite_session_snapshot;
pub use start::{
    recite_session_start, recite_session_start_with_locale_provider,
    recite_session_start_with_locale_provider_and_variant, recite_session_start_with_values,
    recite_session_start_with_values_and_locale_provider,
    recite_session_start_with_values_and_locale_provider_and_variant,
};
pub use values::{recite_session_free, recite_session_set_interpolation_values};

type SessionMap = Mutex<BTreeMap<u64, FfiSession>>;

fn sessions() -> &'static SessionMap {
    static SESSIONS: OnceLock<SessionMap> = OnceLock::new();
    SESSIONS.get_or_init(|| Mutex::new(BTreeMap::new()))
}

#[allow(clippy::unwrap_used)]
pub(crate) fn lock_sessions() -> std::sync::MutexGuard<'static, BTreeMap<u64, FfiSession>> {
    sessions().lock().unwrap()
}

pub(crate) struct FfiSession {
    pub(crate) dialogue: std::sync::Arc<CompiledDialogue>,
    pub(crate) session: DialogueSession,
    pub(crate) handlers: BTreeMap<String, ConditionEntry>,
    pub(crate) interpolation_values: InterpolationValues,
    pub(crate) locale_provider: Option<FfiLocaleProvider>,
    pub(crate) locale_variant: Option<String>,
    pub(crate) owner_thread: ThreadId,
    /// False until `recite_session_begin` (or the `recite_session_start` shorthand) runs the
    /// initial drain. Guards against double-begin on a session created with
    /// `recite_session_create`.
    pub(crate) begun: bool,
}

/// Parses an optional UTF-8 NUL-terminated session string. Empty strings are
/// treated as an explicit clear, matching the existing locale and block
/// parameter convention.
///
/// # Safety
/// `value`, when non-null, must point to a valid NUL-terminated UTF-8 string
/// for the duration of the call.
pub(crate) unsafe fn parse_optional_session_string(
    value: *const c_char,
    label: &str,
) -> Result<Option<String>, ReciteStatus> {
    if value.is_null() {
        return Ok(None);
    }
    match unsafe { CStr::from_ptr(value) }.to_str() {
        Ok("") => Ok(None),
        Ok(value) => Ok(Some(value.to_owned())),
        Err(_) => {
            set_last_error(&format!("{label} is not valid UTF-8"));
            Err(ReciteStatus::Validation)
        }
    }
}

/// Parses `start_block` and `locale` C strings into Rust types.
///
/// # Safety
/// Both pointers, if non-null, must point to valid NUL-terminated UTF-8.
pub(crate) unsafe fn parse_session_params(
    start_block: *const c_char,
    locale: *const c_char,
) -> Result<(Option<String>, DialogueSessionOptions), (ReciteStatus, String)> {
    let block = if start_block.is_null() {
        None
    } else {
        match unsafe { CStr::from_ptr(start_block) }.to_str() {
            Ok("") => None,
            Ok(s) => Some(s.to_owned()),
            Err(_) => {
                return Err((
                    ReciteStatus::Validation,
                    "start_block is not valid UTF-8".to_owned(),
                ));
            }
        }
    };

    let options = if locale.is_null() {
        DialogueSessionOptions::new()
    } else {
        match unsafe { CStr::from_ptr(locale) }.to_str() {
            Ok("") => DialogueSessionOptions::new(),
            Ok(s) => match recite_core::LocaleId::new(s) {
                Ok(locale_id) => DialogueSessionOptions::new().with_locale(locale_id),
                Err(e) => return Err((ReciteStatus::Localisation, format!("invalid locale: {e}"))),
            },
            Err(_) => {
                return Err((
                    ReciteStatus::Validation,
                    "locale is not valid UTF-8".to_owned(),
                ));
            }
        }
    };

    Ok((block, options))
}

pub(crate) fn ensure_session_thread(ffi_session: &FfiSession) -> Result<(), ReciteStatus> {
    let current = thread::current().id();
    if current == ffi_session.owner_thread {
        return Ok(());
    }

    set_last_error("session handle used from a different thread than the one that created it");
    Err(ReciteStatus::Validation)
}

pub(crate) fn is_boundary_error(error: &DialogueError) -> bool {
    matches!(
        error,
        DialogueError::PromptPending { .. } | DialogueError::EffectPending { .. }
    )
}
