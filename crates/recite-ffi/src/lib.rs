//! C ABI surface for non-Rust engine adapters.
//!
//! Design decisions are documented in `docs/c-abi-boundary-design.md`.
//! Normative adapter semantics are in `docs/engine-adapter-contract.md`.

mod condition;
mod condition_codec;
mod error;
mod output;

use std::collections::BTreeMap;
use std::ffi::{CStr, c_char};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread::{self, ThreadId};

use recite_core::{CompiledDialogue, decode_compiled_dialogue_messagepack};
use recite_runtime::{
    DialogueEvent, DialogueSession, DialogueSessionOptions, EffectAck, LocaleResolution,
    acknowledge_effect, choose_with, decode_session_messagepack, encode_session_messagepack,
    next_with, start_scene_with_options,
};

pub use condition::{ReciteConditionFn, ReciteConditionQuery, ReciteConditionResult};
pub use error::{ReciteStatus, recite_last_error_message};

use condition::{ConditionEntry, FfiContext, SendPtr};
use error::{clear_condition_status, restore_status, set_last_error};
use output::{FfiOutputEncodeError, encode_batch, should_continue};

/// ABI major version for the generated C header.
///
/// Increment this for breaking C ABI changes.
pub const RECITE_FFI_VERSION_MAJOR: u32 = 0;
/// ABI minor version for additive, backwards-compatible C ABI changes.
pub const RECITE_FFI_VERSION_MINOR: u32 = 0;
/// ABI patch version for documentation-only or implementation-only releases.
pub const RECITE_FFI_VERSION_PATCH: u32 = 1;

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
    owner_thread: ThreadId,
    /// False until `recite_session_begin` (or the `recite_session_start` shorthand) runs the
    /// initial drain. Guards against double-begin on a session created with
    /// `recite_session_create`.
    begun: bool,
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

    fn from_bytes(bytes: Vec<u8>) -> Self {
        // Use Box<[u8]> so the allocator sees the exact allocation size on free,
        // avoiding the capacity-vs-len mismatch that Vec::from_raw_parts can cause.
        let boxed = bytes.into_boxed_slice();
        let len = boxed.len();
        let data = Box::into_raw(boxed) as *mut u8;
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
        // SAFETY: data+len were produced by Box<[u8]>::into_raw in from_bytes.
        unsafe {
            let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(buf.data, buf.len));
        }
    }
    // Always zero both fields so the host sees a clean null state after free,
    // even when len was 0 and no deallocation ran.
    buf.data = std::ptr::null_mut();
    buf.len = 0;
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

/// Parses `start_block` and `locale` C strings into Rust types.
///
/// # Safety
/// Both pointers, if non-null, must point to valid NUL-terminated UTF-8.
unsafe fn parse_session_params(
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

/// Creates a session handle without running any traversal.
///
/// Use this instead of `recite_session_start` when conditions appear in the
/// opening block of the scene: register handlers with
/// `recite_session_register_condition` after this call, then call
/// `recite_session_begin` to run the first traversal drain.
///
/// `start_block` and `locale` are nullable UTF-8 NUL-terminated strings.
/// On success writes a non-zero handle to `*session_handle_out`.
///
/// # Safety
/// All non-null pointer arguments must be valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn recite_session_create(
    asset_handle: u64,
    start_block: *const c_char,
    locale: *const c_char,
    session_handle_out: *mut u64,
) -> ReciteStatus {
    if session_handle_out.is_null() {
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

    let (block, options) = match unsafe { parse_session_params(start_block, locale) } {
        Ok(v) => v,
        Err((status, msg)) => {
            set_last_error(&msg);
            return status;
        }
    };

    let session = match start_scene_with_options(&dialogue, block.as_deref(), options) {
        Ok(s) => s,
        Err(e) => {
            set_last_error(&e.to_string());
            return ReciteStatus::from(e);
        }
    };

    let handle = alloc_handle();
    lock_sessions().insert(
        handle,
        FfiSession {
            dialogue,
            session,
            handlers: BTreeMap::new(),
            owner_thread: thread::current().id(),
            begun: false,
        },
    );
    unsafe { *session_handle_out = handle };
    ReciteStatus::Ok
}

/// Runs the initial traversal drain for a session created with
/// `recite_session_create`.
///
/// Must be called exactly once per session, after all condition handlers have
/// been registered with `recite_session_register_condition`. On success writes
/// the first output batch to `*batch_out`.
///
/// # Safety
/// `batch_out` must be a valid non-null pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn recite_session_begin(
    session_handle: u64,
    batch_out: *mut ReciteBuffer,
) -> ReciteStatus {
    if batch_out.is_null() {
        set_last_error("null pointer argument");
        return ReciteStatus::Validation;
    }

    let mut guard = lock_sessions();
    let ffi_session = match guard.get_mut(&session_handle) {
        Some(s) => s,
        None => {
            set_last_error("unknown session handle");
            return ReciteStatus::InvalidHandle;
        }
    };
    if let Err(status) = ensure_session_thread(ffi_session) {
        return status;
    }

    if ffi_session.begun {
        set_last_error("recite_session_begin called twice on the same handle");
        return ReciteStatus::SessionAlreadyActive;
    }
    let context = FfiContext {
        handlers: &ffi_session.handlers,
    };
    let session_checkpoint = ffi_session.session.clone();
    clear_condition_status();
    match drain_to_batch(&ffi_session.dialogue, &mut ffi_session.session, &context) {
        Ok(batch) => {
            ffi_session.begun = true;
            unsafe { *batch_out = batch };
            ReciteStatus::Ok
        }
        Err((status, msg)) => {
            ffi_session.session = session_checkpoint;
            set_last_error(&msg);
            status
        }
    }
}

/// Convenience that combines `recite_session_create` and `recite_session_begin`.
///
/// Use this when no conditions appear in the opening block of the scene. For
/// scenes that evaluate conditions at scene start, use `recite_session_create`
/// + `recite_session_register_condition` + `recite_session_begin` instead.
///
/// `start_block` and `locale` are nullable UTF-8 NUL-terminated strings.
/// On success writes a non-zero handle to `*session_handle_out` and the initial
/// output batch to `*batch_out`.
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

    // Create the session entry (no traversal yet).
    let status =
        unsafe { recite_session_create(asset_handle, start_block, locale, session_handle_out) };
    if status != ReciteStatus::Ok {
        return status;
    }

    // Run the initial drain.
    let handle = unsafe { *session_handle_out };
    let begin_status = unsafe { recite_session_begin(handle, batch_out) };
    if begin_status != ReciteStatus::Ok {
        // Remove the created (but unbegun) session so the caller doesn't hold a
        // handle that can't be used.
        lock_sessions().remove(&handle);
        unsafe { *session_handle_out = 0 };
    }
    begin_status
}

/// Registers a condition handler on an existing session handle.
///
/// For conditions that appear in the opening block of a scene, call this
/// after `recite_session_create` and before `recite_session_begin`. For
/// conditions that only appear in later blocks (e.g. after a choice), it is
/// also safe to register after `recite_session_start` returns.
///
/// `name` is a UTF-8 NUL-terminated condition function name. `userdata` is
/// passed back to the handler on each invocation.
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
            if let Err(status) = ensure_session_thread(ffi_session) {
                return status;
            }
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
    if let Err(status) = ensure_session_thread(ffi_session) {
        return status;
    }

    let context = FfiContext {
        handlers: &ffi_session.handlers,
    };
    let session_checkpoint = ffi_session.session.clone();
    clear_condition_status();
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
            match unsafe { CStr::from_ptr(failure_reason) }.to_str() {
                Ok(reason) => reason.to_owned(),
                Err(_) => {
                    set_last_error("failure_reason is not valid UTF-8");
                    return ReciteStatus::Validation;
                }
            }
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
    if let Err(status) = ensure_session_thread(ffi_session) {
        return status;
    }

    let context = FfiContext {
        handlers: &ffi_session.handlers,
    };
    let session_checkpoint = ffi_session.session.clone();
    clear_condition_status();
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
    if let Err(status) = ensure_session_thread(ffi_session) {
        return status;
    }
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
/// `*session_handle_out` and a resumption output batch to `*batch_out`. The
/// batch is empty when the restored session is at a pending-prompt boundary.
/// A pending blocking effect is re-emitted once in the resumption batch with
/// the same request ID so the host can reconcile or re-present it.
/// If the snapshot encoded an ended session, `recite_session_restore` returns
/// `RECITE_ERR_NO_ACTIVE_SESSION`.
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
            return restore_status(&e);
        }
    };

    let context = FfiContext {
        handlers: &BTreeMap::new(),
    };
    clear_condition_status();
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
            owner_thread: thread::current().id(),
            begun: true,
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
        Err(e) if is_boundary_error(&e) => empty_batch(),
        Err(e) => Err((ReciteStatus::from(e.clone()), e.to_string())),
    }
}

fn drain_restored(
    dialogue: &CompiledDialogue,
    session: &mut DialogueSession,
    context: &FfiContext<'_>,
) -> Result<ReciteBuffer, (ReciteStatus, String)> {
    // After restore, a pending prompt is valid — return an empty batch rather
    // than an error so the host can re-display its own state. A pending
    // blocking effect is re-emitted once by `next_with` so the host receives
    // its stable request ID for reconciliation.
    // A restored ended session propagates NoActiveSession to the caller.
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
    encode_batch_output(events, encode_batch)
}

fn empty_batch() -> Result<ReciteBuffer, (ReciteStatus, String)> {
    encode_batch_output(Vec::new(), encode_batch)
}

fn encode_batch_output(
    events: Vec<DialogueEvent>,
    encoder: fn(Vec<DialogueEvent>) -> Result<Vec<u8>, FfiOutputEncodeError>,
) -> Result<ReciteBuffer, (ReciteStatus, String)> {
    encoder(events)
        .map(ReciteBuffer::from_bytes)
        .map_err(flatten_output_encode_error)
}

fn flatten_output_encode_error(error: FfiOutputEncodeError) -> (ReciteStatus, String) {
    // This is the C ABI boundary: preserve the typed encoder error internally,
    // then expose the existing stable status and thread-local detail string.
    (ReciteStatus::DialogueFault, error.to_string())
}

fn ensure_session_thread(ffi_session: &FfiSession) -> Result<(), ReciteStatus> {
    let current = thread::current().id();
    if current == ffi_session.owner_thread {
        return Ok(());
    }

    set_last_error("session handle used from a different thread than the one that created it");
    Err(ReciteStatus::Validation)
}

fn is_boundary_error(e: &recite_runtime::DialogueError) -> bool {
    matches!(
        e,
        recite_runtime::DialogueError::PromptPending { .. }
            | recite_runtime::DialogueError::EffectPending { .. }
    )
}
