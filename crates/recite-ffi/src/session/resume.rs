//! Resume saves across compiled revisions through the C ABI.

use recite_adapter::resume_names::{
    fallback_reason_landmark, fallback_reason_name, landmark_choices, landmark_kind, landmark_line,
    prompt_resume_name, resume_level_name,
};
use recite_adapter::{LoadedDialogue, PreparedRestore, SessionDriver};
use recite_runtime::resume::{
    ResumeFallback, ResumeFallbackReason, ResumeLandmark, ResumeLevel, ResumeOptions, ResumeReport,
};
use serde::Serialize;

use crate::asset::{alloc_handle, lock_assets};
use crate::buffer::{ReciteBuffer, checked_bytes};
use crate::error::{ReciteStatus, set_last_error};
use crate::output::{FfiEffect, ffi_effect};

/// Accept only a restore against the exact saved asset.
pub const RECITE_RESUME_LEVEL_EXACT: u32 = 0;
/// Also accept an asset whose saved block is unchanged.
pub const RECITE_RESUME_LEVEL_UNCHANGED_BLOCK: u32 = 1;
/// Also accept text-only edits in the saved block.
pub const RECITE_RESUME_LEVEL_TEXT_EDITED: u32 = 2;
/// Accept any deterministic relocation of the saved position.
pub const RECITE_RESUME_LEVEL_RELOCATED: u32 = 3;

/// Format version of the MessagePack resume report.
pub(crate) const RESUME_REPORT_FORMAT_VERSION: u16 = 0;

/// Prepares a save against an asset that may be a patched revision of the
/// one it was saved against, without running traversal.
///
/// `max_level` is one of the `RECITE_RESUME_LEVEL_*` constants. On success,
/// `*report_out` receives a MessagePack named-map report (free it with
/// `recite_buffer_free`). Its `outcome` is `resumed`, with a prepared session
/// written to `*session_handle_out`, or `fallback`, with `*session_handle_out`
/// set to 0 and the saved block, deferred effects and pending blocking effect
/// in the report. Configure the prepared session, then call
/// `recite_session_begin`: it re-presents a pending prompt or re-emits a
/// pending blocking effect with its original request ID.
///
/// Malformed or unsupported snapshots, and snapshots of a different asset,
/// return an error status and allocate neither a handle nor a report.
///
/// # Safety
/// Both output pointers must be valid and non-null. `snapshot_bytes` must be
/// non-null and valid for `snapshot_len` bytes when the length fits Rust
/// `isize`; larger lengths are rejected before reading the snapshot.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn recite_session_prepare_resume(
    asset_handle: u64,
    snapshot_bytes: *const u8,
    snapshot_len: usize,
    max_level: u32,
    session_handle_out: *mut u64,
    report_out: *mut ReciteBuffer,
) -> ReciteStatus {
    if session_handle_out.is_null() || report_out.is_null() {
        set_last_error("null pointer argument");
        return ReciteStatus::Validation;
    }
    let Some(level) = level_from_abi(max_level) else {
        set_last_error(&format!("unknown resume level {max_level}"));
        return ReciteStatus::Validation;
    };
    let options = ResumeOptions::new().with_max_level(level);
    let (handle, prepared) =
        match unsafe { prepare(asset_handle, snapshot_bytes, snapshot_len, &options) } {
            Ok(prepared) => prepared,
            Err(status) => return status,
        };
    let report = match &prepared {
        PreparedRestore::Resumed(report) => ResumeReportWire::resumed(report),
        PreparedRestore::Fallback(fallback) => ResumeReportWire::fallback(fallback),
        _ => {
            set_last_error("unsupported resume outcome");
            return ReciteStatus::SaveLoadIncompatibility;
        }
    };
    match rmp_serde::to_vec_named(&report) {
        Ok(bytes) => {
            unsafe {
                *session_handle_out = handle.unwrap_or(0);
                *report_out = ReciteBuffer::from_bytes(bytes);
            }
            ReciteStatus::Ok
        }
        Err(error) => {
            if let Some(handle) = handle
                && let Ok(mut guard) = super::lock_sessions()
            {
                guard.remove(&handle);
            }
            set_last_error(&format!("failed to encode resume report: {error}"));
            ReciteStatus::DialogueFault
        }
    }
}

/// Resumes a save and begins it in one call, for hosts that need no
/// condition handlers, interpolation values or locale configuration.
///
/// Behaves like `recite_session_prepare_resume` followed by
/// `recite_session_begin`. A fallback writes the report, sets
/// `*session_handle_out` to 0 and leaves `*batch_out` empty.
///
/// # Safety
/// All output pointers must be valid and non-null. Snapshot bytes follow the
/// rules of `recite_session_prepare_resume`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn recite_session_resume(
    asset_handle: u64,
    snapshot_bytes: *const u8,
    snapshot_len: usize,
    max_level: u32,
    session_handle_out: *mut u64,
    report_out: *mut ReciteBuffer,
    batch_out: *mut ReciteBuffer,
) -> ReciteStatus {
    if session_handle_out.is_null() || report_out.is_null() || batch_out.is_null() {
        set_last_error("null pointer argument");
        return ReciteStatus::Validation;
    }
    let mut handle = 0;
    let mut report = ReciteBuffer::null();
    let status = unsafe {
        recite_session_prepare_resume(
            asset_handle,
            snapshot_bytes,
            snapshot_len,
            max_level,
            &raw mut handle,
            &raw mut report,
        )
    };
    if status != ReciteStatus::Ok {
        return status;
    }
    if handle == 0 {
        unsafe {
            *session_handle_out = 0;
            *report_out = report;
            *batch_out = ReciteBuffer::null();
        }
        return ReciteStatus::Ok;
    }
    let status = unsafe { super::begin_and_publish(handle, session_handle_out, batch_out) };
    if status == ReciteStatus::Ok {
        unsafe { *report_out = report };
    } else {
        unsafe { crate::buffer::recite_buffer_free(&raw mut report) };
    }
    status
}

/// Prepares a restore; a resumed session is registered and its handle returned.
unsafe fn prepare(
    asset_handle: u64,
    snapshot_bytes: *const u8,
    snapshot_len: usize,
    options: &ResumeOptions,
) -> Result<(Option<u64>, PreparedRestore), ReciteStatus> {
    let bytes = match unsafe { checked_bytes(snapshot_bytes, snapshot_len, "snapshot bytes") } {
        Ok(bytes) => bytes,
        Err(error) => {
            set_last_error(&error);
            return Err(ReciteStatus::Validation);
        }
    };
    let Some(dialogue) = lock_assets().get(&asset_handle).cloned() else {
        set_last_error("unknown asset handle");
        return Err(ReciteStatus::InvalidHandle);
    };
    let mut driver = SessionDriver::new();
    let prepared = LoadedDialogue::from_shared(dialogue)
        .and_then(|asset| driver.prepare_restore(&asset, bytes, options))
        .map_err(|error| {
            set_last_error(&error.to_string());
            ReciteStatus::from(error)
        })?;
    if !matches!(prepared, PreparedRestore::Resumed(_)) {
        return Ok((None, prepared));
    }
    let handle = alloc_handle();
    let mut guard = super::lock_sessions()?;
    guard.insert(handle, super::FfiSession::prepared(driver));
    Ok((Some(handle), prepared))
}

/// Restores through the legacy exact-only entrypoints: any change to the
/// compiled asset is a save/load incompatibility.
pub(super) unsafe fn prepare_exact(
    asset_handle: u64,
    snapshot_bytes: *const u8,
    snapshot_len: usize,
) -> Result<u64, ReciteStatus> {
    match unsafe {
        prepare(
            asset_handle,
            snapshot_bytes,
            snapshot_len,
            &ResumeOptions::exact_only(),
        )
    }? {
        (Some(handle), _) => Ok(handle),
        (None, PreparedRestore::Fallback(fallback)) => {
            set_last_error(&format!(
                "save of block `{}` does not match this compiled asset ({}); use recite_session_prepare_resume to resume it in a patched asset",
                fallback.block_id,
                fallback_reason_name(&fallback.reason)
            ));
            Err(ReciteStatus::SaveLoadIncompatibility)
        }
        (None, _) => {
            set_last_error("unsupported resume outcome");
            Err(ReciteStatus::SaveLoadIncompatibility)
        }
    }
}

fn level_from_abi(level: u32) -> Option<ResumeLevel> {
    Some(match level {
        RECITE_RESUME_LEVEL_EXACT => ResumeLevel::Exact,
        RECITE_RESUME_LEVEL_UNCHANGED_BLOCK => ResumeLevel::UnchangedBlock,
        RECITE_RESUME_LEVEL_TEXT_EDITED => ResumeLevel::TextEdited,
        RECITE_RESUME_LEVEL_RELOCATED => ResumeLevel::Relocated,
        _ => return None,
    })
}

/// The named-map resume report. Field names follow
/// `recite_adapter::resume_names` and match Godot's restore dictionary.
#[derive(Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
enum ResumeReportWire<'a> {
    Resumed {
        report_format_version: u16,
        level: &'static str,
        block_id: &'a str,
        prompt: &'static str,
        schema_changed: bool,
        compiler_changed: bool,
    },
    Fallback(Box<FallbackWire<'a>>),
}

#[derive(Serialize)]
struct FallbackWire<'a> {
    report_format_version: u16,
    reason: &'static str,
    required_level: Option<&'static str>,
    allowed_level: Option<&'static str>,
    reason_landmark: Option<LandmarkWire<'a>>,
    block_id: &'a str,
    block_available: bool,
    landmark: Option<LandmarkWire<'a>>,
    ended: bool,
    deferred_effects: Vec<FfiEffect>,
    pending_effect: Option<FfiEffect>,
    selected_choice_history: Vec<&'a str>,
    locale: Option<&'a str>,
}

#[derive(Serialize)]
struct LandmarkWire<'a> {
    kind: &'static str,
    line: Option<&'a str>,
    choices: Vec<&'a str>,
}

impl<'a> ResumeReportWire<'a> {
    fn resumed(report: &'a ResumeReport) -> Self {
        Self::Resumed {
            report_format_version: RESUME_REPORT_FORMAT_VERSION,
            level: resume_level_name(report.level),
            block_id: report.block_id.as_str(),
            prompt: prompt_resume_name(report.prompt),
            schema_changed: report.schema_changed,
            compiler_changed: report.compiler_changed,
        }
    }

    fn fallback(fallback: &'a ResumeFallback) -> Self {
        let (required_level, allowed_level) = match &fallback.reason {
            ResumeFallbackReason::LevelRefused { required, allowed } => (
                Some(resume_level_name(*required)),
                Some(resume_level_name(*allowed)),
            ),
            _ => (None, None),
        };
        Self::Fallback(Box::new(FallbackWire {
            report_format_version: RESUME_REPORT_FORMAT_VERSION,
            reason: fallback_reason_name(&fallback.reason),
            required_level,
            allowed_level,
            reason_landmark: fallback_reason_landmark(&fallback.reason).map(LandmarkWire::new),
            block_id: fallback.block_id.as_str(),
            block_available: fallback.block_available,
            landmark: fallback.landmark.as_ref().map(LandmarkWire::new),
            ended: fallback.ended,
            deferred_effects: fallback
                .deferred_effects
                .iter()
                .cloned()
                .map(ffi_effect)
                .collect(),
            pending_effect: fallback.pending_effect.clone().map(ffi_effect),
            selected_choice_history: fallback
                .selected_choice_history
                .iter()
                .map(|choice| choice.as_str())
                .collect(),
            locale: fallback.locale.as_ref().map(|locale| locale.as_str()),
        }))
    }
}

impl<'a> LandmarkWire<'a> {
    fn new(landmark: &'a ResumeLandmark) -> Self {
        Self {
            kind: landmark_kind(landmark),
            line: landmark_line(landmark),
            choices: landmark_choices(landmark),
        }
    }
}
