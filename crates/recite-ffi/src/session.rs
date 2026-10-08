use std::collections::BTreeMap;
use std::ffi::{CStr, c_char};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::{self, ThreadId};

use recite_runtime::{
    DialogueSessionOptions, LocaleResolution,
    localisation::{InterpolationValues, LocaleProvider},
};

use crate::condition::ConditionEntry;
use crate::error::{ReciteStatus, set_last_error};
use crate::locale::FfiLocaleProvider;
use recite_adapter::{DriverError, ReciteDialogueCatalog, SessionDriver};

mod begin;
mod choice;
mod create;
mod effect;
mod locale;
mod prepare_restore;
mod restore;
mod snapshot;
mod start;
mod values;

pub use begin::{recite_session_begin, recite_session_register_condition};
pub use choice::recite_session_choose;
pub use create::recite_session_create;
pub use effect::recite_session_acknowledge_effect;
pub use locale::{
    recite_session_clear_locale_provider, recite_session_set_locale_provider,
    recite_session_set_locale_variant,
};
pub use prepare_restore::recite_session_prepare_restore;
pub use restore::recite_session_restore;
pub use snapshot::recite_session_snapshot;
pub use start::recite_session_start;
pub use values::{recite_session_free, recite_session_set_interpolation_values};

type SessionMap = Mutex<BTreeMap<u64, FfiSession>>;

fn sessions() -> &'static SessionMap {
    static SESSIONS: OnceLock<SessionMap> = OnceLock::new();
    SESSIONS.get_or_init(|| Mutex::new(BTreeMap::new()))
}

#[allow(
    clippy::unwrap_used,
    reason = "ffi: the process-global session registry cannot recover a poisoned mutex"
)]
pub(crate) fn lock_sessions() -> std::sync::MutexGuard<'static, BTreeMap<u64, FfiSession>> {
    sessions().lock().unwrap()
}

pub(crate) struct FfiSession {
    pub(crate) driver: SessionDriver,
    pub(crate) handlers: BTreeMap<String, ConditionEntry>,
    pub(crate) interpolation_values: InterpolationValues,
    pub(crate) locale_source: FfiLocaleSource,
    pub(crate) locale_variant: Option<String>,
    pub(crate) owner_thread: ThreadId,
}

impl FfiSession {
    fn prepared(driver: SessionDriver) -> Self {
        Self {
            driver,
            handlers: BTreeMap::new(),
            interpolation_values: InterpolationValues::new(),
            locale_source: FfiLocaleSource::None,
            locale_variant: None,
            owner_thread: thread::current().id(),
        }
    }
}

// Convenience entrypoints validate outputs before allocating. Publish the
// handle only after begin succeeds; failure frees the prepared session.
unsafe fn begin_and_publish(
    handle: u64,
    session_handle_out: *mut u64,
    batch_out: *mut crate::buffer::ReciteBuffer,
) -> ReciteStatus {
    let status = unsafe { recite_session_begin(handle, batch_out) };
    if status == ReciteStatus::Ok {
        unsafe { *session_handle_out = handle };
    } else {
        lock_sessions().remove(&handle);
    }
    status
}

pub(crate) enum FfiLocaleSource {
    None,
    Callback(FfiLocaleProvider),
    Catalog(Arc<ReciteDialogueCatalog>),
}

impl FfiLocaleSource {
    pub(crate) fn provider(&self) -> Option<&dyn LocaleProvider> {
        match self {
            Self::None => None,
            Self::Callback(provider) => Some(provider),
            Self::Catalog(catalog) => Some(catalog.as_ref()),
        }
    }
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

pub(crate) fn locale_resolution<'a>(
    values: &'a InterpolationValues,
    provider: Option<&'a dyn LocaleProvider>,
    variant: Option<&'a str>,
) -> LocaleResolution<'a> {
    let resolution = LocaleResolution::new().with_values(values);
    let resolution = provider.map_or(resolution, |provider| resolution.with_provider(provider));
    variant.map_or(resolution, |variant| resolution.with_variant(variant))
}

pub(crate) fn driver_failure(error: DriverError<(ReciteStatus, String)>) -> (ReciteStatus, String) {
    match error {
        DriverError::Adapter(error) => (ReciteStatus::from(error.kind()), error.to_string()),
        DriverError::Output(error) => error,
        _ => (
            ReciteStatus::DialogueFault,
            "unknown adapter driver error".to_owned(),
        ),
    }
}
