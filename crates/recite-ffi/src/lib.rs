//! C ABI surface for non-Rust engine adapters.
//!
//! Design decisions are documented in `docs/c-abi-boundary-design.md`.
//! Normative adapter semantics are in `docs/engine-adapter-contract.md`.

mod condition;
mod error;
mod output;

use std::collections::BTreeMap;
use std::ffi::{CStr, c_char};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use recite_core::{CompiledDialogue, decode_compiled_dialogue_messagepack};
use recite_runtime::{
    DialogueEvent, DialogueSession, DialogueSessionOptions, EffectAck, LocaleResolution,
    acknowledge_effect, choose_with, decode_session_messagepack, encode_session_messagepack,
    next_with, start_scene_with_options,
};

pub use condition::{ReciteConditionFn, ReciteConditionQuery, ReciteConditionResult};
pub use error::{ReciteStatus, recite_last_error_message};

use condition::{ConditionEntry, FfiContext, SendPtr};
use error::set_last_error;
use output::{encode_batch, should_continue};

// ---------------------------------------------------------------------------
// Handle registry
// ---------------------------------------------------------------------------

static NEXT_HANDLE: AtomicU64 = AtomicU64::new(1);

fn alloc_handle() -> u64 {
    NEXT_HANDLE.fetch_add(1, Ordering::Relaxed)
}

type AssetMap = Mutex<BTreeMap<u64, std::sync::Arc<CompiledDialogue>>>;
type SessionMap = Mutex<BTreeMap<u64, FfiSession>>;

fn assets() -> &'static AssetMap {
    static ASSETS: OnceLock<AssetMap> = OnceLock::new();
    ASSETS.get_or_init(|| Mutex::new(BTreeMap::new()))
}

fn sessions() -> &'static SessionMap {
    static SESSIONS: OnceLock<SessionMap> = OnceLock::new();
    SESSIONS.get_or_init(|| Mutex::new(BTreeMap::new()))
}

// Mutex poison is unrecoverable in a cdylib boundary; unwrap is intentional.
#[allow(clippy::unwrap_used)]
fn lock_assets() -> std::sync::MutexGuard<'static, BTreeMap<u64, std::sync::Arc<CompiledDialogue>>>
{
    assets().lock().unwrap()
}

#[allow(clippy::unwrap_used)]
fn lock_sessions() -> std::sync::MutexGuard<'static, BTreeMap<u64, FfiSession>> {
    sessions().lock().unwrap()
}

struct FfiSession {
    dialogue: std::sync::Arc<CompiledDialogue>,
    session: DialogueSession,
    handlers: BTreeMap<String, ConditionEntry>,
}

// ---------------------------------------------------------------------------
// Output buffer
// ---------------------------------------------------------------------------

/// Heap-allocated byte buffer returned by `recite-ffi`.
///
/// The host must call `recite_buffer_free` exactly once after consuming the
/// data. The buffer is allocated by Rust's global allocator; do not free it
/// with a different allocator.
///
/// # Safety
/// Freeing with the wrong allocator (e.g. the Unity C runtime's `free`) is
/// undefined behaviour. Always link against `recite-ffi` as a pre-built
/// `.dll`/`.so`; never recompile it per backend or link it statically into
/// a Unity player separately.
#[repr(C)]
pub struct ReciteBuffer {
    pub data: *mut u8,
    pub len: usize,
}

impl ReciteBuffer {
    pub fn null() -> Self {
        Self {
            data: std::ptr::null_mut(),
            len: 0,
        }
    }

    fn from_bytes(mut bytes: Vec<u8>) -> Self {
        bytes.shrink_to_fit();
        let len = bytes.len();
        let data = bytes.as_mut_ptr();
        std::mem::forget(bytes);
        Self { data, len }
    }
}

/// Frees a buffer allocated by `recite-ffi`.
///
/// # Safety
/// `buf` must have been produced by a `recite-ffi` function and must not have
/// been freed already.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn recite_buffer_free(buf: *mut ReciteBuffer) {
    if buf.is_null() {
        return;
    }
    let buf = unsafe { &mut *buf };
    if !buf.data.is_null() && buf.len > 0 {
        // SAFETY: data was allocated by Vec::from_bytes above.
        unsafe {
            let _ = Vec::from_raw_parts(buf.data, buf.len, buf.len);
        }
        buf.data = std::ptr::null_mut();
        buf.len = 0;
    }
}

// ---------------------------------------------------------------------------
// Asset lifecycle
// ---------------------------------------------------------------------------

/// Loads and decodes a compiled Recite asset from a byte slice.
///
/// On success writes a non-zero handle to `*asset_handle_out` and returns
/// `RECITE_OK`. On failure returns a negative status code and sets the
/// thread-local error message.
///
/// The asset handle is valid until `recite_asset_free` is called.
///
/// # Safety
/// `bytes` must be valid for `len` bytes. `asset_handle_out` must be a valid
/// non-null pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn recite_asset_load(
    bytes: *const u8,
    len: usize,
    asset_handle_out: *mut u64,
) -> ReciteStatus {
    if bytes.is_null() || asset_handle_out.is_null() {
        set_last_error("null pointer argument");
        return ReciteStatus::Validation;
    }
    let bytes_slice = unsafe { std::slice::from_raw_parts(bytes, len) };
    match decode_compiled_dialogue_messagepack(bytes_slice) {
        Ok(dialogue) => {
            let handle = alloc_handle();
            lock_assets().insert(handle, std::sync::Arc::new(dialogue));
            unsafe { *asset_handle_out = handle };
            ReciteStatus::Ok
        }
        Err(e) => {
            set_last_error(&e.to_string());
            ReciteStatus::AssetLoadOrDecode
        }
    }
}

/// Frees an asset handle.
///
/// If sessions that reference this asset are still alive, their internal
/// `Arc` reference keeps the underlying data alive until those sessions are
/// also freed.
#[unsafe(no_mangle)]
pub extern "C" fn recite_asset_free(asset_handle: u64) {
    lock_assets().remove(&asset_handle);
}

// ---------------------------------------------------------------------------
// Session lifecycle
// ---------------------------------------------------------------------------

/// Starts a new dialogue session from an asset handle.
///
/// `start_block` and `locale` are nullable UTF-8 NUL-terminated strings.
/// On success writes a session handle to `*session_handle_out`, writes the
/// initial output batch to `*batch_out`, and returns `RECITE_OK`.
///
/// Condition handlers must be registered on the returned session handle before
/// traversal will succeed for assets that use conditions. `start` triggers the
/// first traversal call after session creation.
///
/// # Safety
/// All non-null pointer arguments must be valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn recite_session_start(
    asset_handle: u64,
    start_block: *const c_char,
    locale: *const c_char,
    session_handle_out: *mut u64,
    batch_out: *mut ReciteBuffer,
) -> ReciteStatus {
    if session_handle_out.is_null() || batch_out.is_null() {
        set_last_error("null pointer argument");
        return ReciteStatus::Validation;
    }

    let dialogue = {
        let guard = lock_assets();
        match guard.get(&asset_handle).cloned() {
            Some(d) => d,
            None => {
                set_last_error("unknown asset handle");
                return ReciteStatus::InvalidHandle;
            }
        }
    };

    let block = if start_block.is_null() {
        None
    } else {
        // SAFETY: caller guarantees valid NUL-terminated UTF-8.
        match unsafe { CStr::from_ptr(start_block) }.to_str() {
            Ok("") => None,
            Ok(s) => Some(s.to_owned()),
            Err(_) => {
                set_last_error("start_block is not valid UTF-8");
                return ReciteStatus::Validation;
            }
        }
    };

    let options = if locale.is_null() {
        DialogueSessionOptions::new()
    } else {
        // SAFETY: caller guarantees valid NUL-terminated UTF-8.
        match unsafe { CStr::from_ptr(locale) }.to_str() {
            Ok("") => DialogueSessionOptions::new(),
            Ok(s) => match recite_core::LocaleId::new(s) {
                Ok(locale_id) => DialogueSessionOptions::new().with_locale(locale_id),
                Err(e) => {
                    set_last_error(&format!("invalid locale: {e}"));
                    return ReciteStatus::Localisation;
                }
            },
            Err(_) => {
                set_last_error("locale is not valid UTF-8");
                return ReciteStatus::Validation;
            }
        }
    };

    let mut session = match start_scene_with_options(&dialogue, block.as_deref(), options) {
        Ok(s) => s,
        Err(e) => {
            set_last_error(&e.to_string());
            return ReciteStatus::from(e);
        }
    };

    // Drain initial output with an empty handler map (conditions registered after start).
    let context = FfiContext {
        handlers: &BTreeMap::new(),
    };
    let batch = match drain_to_batch(&dialogue, &mut session, &context) {
        Ok(b) => b,
        Err((status, msg)) => {
            set_last_error(&msg);
            return status;
        }
    };

    let handle = alloc_handle();
    lock_sessions().insert(
        handle,
        FfiSession {
            dialogue,
            session,
            handlers: BTreeMap::new(),
        },
    );
    unsafe { *session_handle_out = handle };
    unsafe { *batch_out = batch };
    ReciteStatus::Ok
}

/// Registers a condition handler on an existing session handle.
///
/// Must be called before the session's first traversal call for conditions
/// that appear in the dialogue. `name` is a UTF-8 NUL-terminated condition
/// function name. `userdata` is passed back to the handler on each invocation.
///
/// # Safety
/// `name` must be valid NUL-terminated UTF-8 for the duration of the call.
/// `userdata` must remain valid and accessible from the calling thread for the
/// session's lifetime.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn recite_session_register_condition(
    session_handle: u64,
    name: *const c_char,
    handler: ReciteConditionFn,
    userdata: *mut std::ffi::c_void,
) -> ReciteStatus {
    if name.is_null() {
        set_last_error("null name argument");
        return ReciteStatus::Validation;
    }
    let name_str = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(s) => s.to_owned(),
        Err(_) => {
            set_last_error("name is not valid UTF-8");
            return ReciteStatus::Validation;
        }
    };
    let mut guard = lock_sessions();
    match guard.get_mut(&session_handle) {
        Some(ffi_session) => {
            ffi_session.handlers.insert(
                name_str,
                ConditionEntry {
                    handler,
                    userdata: SendPtr(userdata),
                },
            );
            ReciteStatus::Ok
        }
        None => {
            set_last_error("unknown session handle");
            ReciteStatus::InvalidHandle
        }
    }
}

/// Selects a pending prompt choice.
///
/// `choice_id` is a UTF-8 NUL-terminated string. On success writes the
/// subsequent output batch to `*batch_out`.
///
/// # Safety
/// All non-null pointer arguments must be valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn recite_session_choose(
    session_handle: u64,
    choice_id: *const c_char,
    batch_out: *mut ReciteBuffer,
) -> ReciteStatus {
    if choice_id.is_null() || batch_out.is_null() {
        set_last_error("null pointer argument");
        return ReciteStatus::Validation;
    }
    let choice_str = match unsafe { CStr::from_ptr(choice_id) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            set_last_error("choice_id is not valid UTF-8");
            return ReciteStatus::Validation;
        }
    };
    let choice_id = match recite_core::ChoiceId::new(choice_str) {
        Ok(id) => id,
        Err(e) => {
            set_last_error(&e.to_string());
            return ReciteStatus::InvalidChoice;
        }
    };

    let mut guard = lock_sessions();
    let ffi_session = match guard.get_mut(&session_handle) {
        Some(s) => s,
        None => {
            set_last_error("unknown session handle");
            return ReciteStatus::InvalidHandle;
        }
    };

    let context = FfiContext {
        handlers: &ffi_session.handlers,
    };
    let session_checkpoint = ffi_session.session.clone();
    let result = match choose_with(
        &ffi_session.dialogue,
        &mut ffi_session.session,
        choice_id,
        &context,
        LocaleResolution::new(),
    ) {
        Ok(first_event) => drain_after_event(
            &ffi_session.dialogue,
            &mut ffi_session.session,
            &context,
            first_event,
        ),
        Err(e) => {
            set_last_error(&e.to_string());
            Err((ReciteStatus::from(e), String::new()))
        }
    };

    match result {
        Ok(batch) => {
            unsafe { *batch_out = batch };
            ReciteStatus::Ok
        }
        Err((status, msg)) => {
            ffi_session.session = session_checkpoint;
            if !msg.is_empty() {
                set_last_error(&msg);
            }
            status
        }
    }
}

/// Acknowledges the currently pending blocking effect.
///
/// `effect_request_id` is a UTF-8 NUL-terminated string matching the effect ID
/// emitted in the output batch. `ack_completed` is 1 for `EffectAck::Completed`
/// or 0 for `EffectAck::Failed`. `failure_reason` is a nullable UTF-8
/// NUL-terminated string used when `ack_completed == 0`.
///
/// # Safety
/// All non-null pointer arguments must be valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn recite_session_acknowledge_effect(
    session_handle: u64,
    effect_request_id: *const c_char,
    ack_completed: u8,
    failure_reason: *const c_char,
    batch_out: *mut ReciteBuffer,
) -> ReciteStatus {
    if effect_request_id.is_null() || batch_out.is_null() {
        set_last_error("null pointer argument");
        return ReciteStatus::Validation;
    }
    let id_str = match unsafe { CStr::from_ptr(effect_request_id) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            set_last_error("effect_request_id is not valid UTF-8");
            return ReciteStatus::Validation;
        }
    };
    let effect_id = match recite_core::EffectId::new(id_str) {
        Ok(id) => id,
        Err(e) => {
            set_last_error(&e.to_string());
            return ReciteStatus::EffectAcknowledgement;
        }
    };

    let ack = if ack_completed != 0 {
        EffectAck::Completed
    } else {
        let reason = if failure_reason.is_null() {
            String::new()
        } else {
            unsafe { CStr::from_ptr(failure_reason) }
                .to_string_lossy()
                .into_owned()
        };
        EffectAck::Failed { reason }
    };

    let mut guard = lock_sessions();
    let ffi_session = match guard.get_mut(&session_handle) {
        Some(s) => s,
        None => {
            set_last_error("unknown session handle");
            return ReciteStatus::InvalidHandle;
        }
    };

    let context = FfiContext {
        handlers: &ffi_session.handlers,
    };
    let session_checkpoint = ffi_session.session.clone();
    let result = match acknowledge_effect(&mut ffi_session.session, effect_id, ack) {
        Ok(()) => drain_to_batch(&ffi_session.dialogue, &mut ffi_session.session, &context),
        Err(e) => {
            set_last_error(&e.to_string());
            Err((ReciteStatus::from(e), String::new()))
        }
    };

    match result {
        Ok(batch) => {
            unsafe { *batch_out = batch };
            ReciteStatus::Ok
        }
        Err((status, msg)) => {
            ffi_session.session = session_checkpoint;
            if !msg.is_empty() {
                set_last_error(&msg);
            }
            status
        }
    }
}

/// Encodes the current session state as an opaque msgpack snapshot.
///
/// The snapshot bytes are written to a freshly allocated `ReciteBuffer`; the
/// host must call `recite_buffer_free` after consuming them.
///
/// # Safety
/// `snapshot_out` must be a valid non-null pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn recite_session_snapshot(
    session_handle: u64,
    snapshot_out: *mut ReciteBuffer,
) -> ReciteStatus {
    if snapshot_out.is_null() {
        set_last_error("null pointer argument");
        return ReciteStatus::Validation;
    }
    let guard = lock_sessions();
    let ffi_session = match guard.get(&session_handle) {
        Some(s) => s,
        None => {
            set_last_error("unknown session handle");
            return ReciteStatus::InvalidHandle;
        }
    };
    match encode_session_messagepack(&ffi_session.session) {
        Ok(bytes) => {
            unsafe { *snapshot_out = ReciteBuffer::from_bytes(bytes) };
            ReciteStatus::Ok
        }
        Err(e) => {
            set_last_error(&e.to_string());
            ReciteStatus::from(e)
        }
    }
}

/// Restores a session from a snapshot previously produced by
/// `recite_session_snapshot`.
///
/// The snapshot must have been produced against the same compiled asset
/// identified by `asset_handle`. On success writes a new session handle to
/// `*session_handle_out` and the resumption output batch to `*batch_out`.
///
/// # Safety
/// All non-null pointer arguments must be valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn recite_session_restore(
    asset_handle: u64,
    snapshot_bytes: *const u8,
    snapshot_len: usize,
    session_handle_out: *mut u64,
    batch_out: *mut ReciteBuffer,
) -> ReciteStatus {
    if snapshot_bytes.is_null() || session_handle_out.is_null() || batch_out.is_null() {
        set_last_error("null pointer argument");
        return ReciteStatus::Validation;
    }
    let dialogue = {
        let guard = lock_assets();
        match guard.get(&asset_handle).cloned() {
            Some(d) => d,
            None => {
                set_last_error("unknown asset handle");
                return ReciteStatus::InvalidHandle;
            }
        }
    };
    let bytes = unsafe { std::slice::from_raw_parts(snapshot_bytes, snapshot_len) };
    let mut session = match decode_session_messagepack(&dialogue, bytes) {
        Ok(s) => s,
        Err(e) => {
            set_last_error(&e.to_string());
            return ReciteStatus::from(e);
        }
    };

    let context = FfiContext {
        handlers: &BTreeMap::new(),
    };
    let batch = match drain_restored(&dialogue, &mut session, &context) {
        Ok(b) => b,
        Err((status, msg)) => {
            set_last_error(&msg);
            return status;
        }
    };

    let handle = alloc_handle();
    lock_sessions().insert(
        handle,
        FfiSession {
            dialogue,
            session,
            handlers: BTreeMap::new(),
        },
    );
    unsafe { *session_handle_out = handle };
    unsafe { *batch_out = batch };
    ReciteStatus::Ok
}

/// Frees a session handle. Does nothing if the handle is unknown.
#[unsafe(no_mangle)]
pub extern "C" fn recite_session_free(session_handle: u64) {
    lock_sessions().remove(&session_handle);
}

// ---------------------------------------------------------------------------
// Traversal helpers (mirrors the Godot adapter's drain pattern)
// ---------------------------------------------------------------------------

fn drain_to_batch(
    dialogue: &CompiledDialogue,
    session: &mut DialogueSession,
    context: &FfiContext<'_>,
) -> Result<ReciteBuffer, (ReciteStatus, String)> {
    match next_with(dialogue, session, context, LocaleResolution::new()) {
        Ok(first_event) => drain_after_event(dialogue, session, context, first_event),
        Err(e) if is_boundary_error(&e) => Ok(empty_batch()),
        Err(e) => Err((ReciteStatus::from(e.clone()), e.to_string())),
    }
}

fn drain_restored(
    dialogue: &CompiledDialogue,
    session: &mut DialogueSession,
    context: &FfiContext<'_>,
) -> Result<ReciteBuffer, (ReciteStatus, String)> {
    // After restore, a pending prompt or blocking effect is valid — return an
    // empty batch rather than an error so the host can re-display state.
    drain_to_batch(dialogue, session, context)
}

fn drain_after_event(
    dialogue: &CompiledDialogue,
    session: &mut DialogueSession,
    context: &FfiContext<'_>,
    first_event: DialogueEvent,
) -> Result<ReciteBuffer, (ReciteStatus, String)> {
    let mut events = Vec::new();
    let mut current = first_event;
    loop {
        let continues = should_continue(&current);
        events.push(current);
        if !continues {
            break;
        }
        match next_with(dialogue, session, context, LocaleResolution::new()) {
            Ok(next_event) => current = next_event,
            Err(e) if is_boundary_error(&e) => break,
            Err(e) => return Err((ReciteStatus::from(e.clone()), e.to_string())),
        }
    }
    encode_batch(events)
        .map(ReciteBuffer::from_bytes)
        .map_err(|msg| (ReciteStatus::Validation, msg))
}

fn empty_batch() -> ReciteBuffer {
    let batch = output::FfiOutputBatch {
        batch_format_version: output::BATCH_FORMAT_VERSION,
        events: Vec::new(),
    };
    let bytes = rmp_serde::to_vec_named(&batch).unwrap_or_default();
    ReciteBuffer::from_bytes(bytes)
}

fn is_boundary_error(e: &recite_runtime::DialogueError) -> bool {
    matches!(
        e,
        recite_runtime::DialogueError::PromptPending { .. }
            | recite_runtime::DialogueError::EffectPending { .. }
    )
}
