use std::ffi::c_void;

use recite_adapter::{LoadedDialogue, SessionDriver, StartRequest};
use recite_compiler::compile::{CompileInput, CompileOptions, compile_inputs};
use recite_core::{
    ChoiceId,
    compiled::{CompiledAssetId, CompilerVersion, SchemaFingerprint, SourceMapId},
};
use recite_ffi::{
    ReciteBuffer, ReciteConditionQuery, ReciteConditionResult, ReciteStatus, recite_asset_free,
    recite_asset_load, recite_buffer_free, recite_session_begin, recite_session_choose,
    recite_session_free, recite_session_prepare_restore, recite_session_register_condition,
    recite_session_snapshot,
};
use recite_runtime::{
    ConditionEvaluationError, ConditionQuery, ConditionValue, DialogueSessionOptions,
    LocaleResolution,
};

pub(super) const HISTORY_SIZES: [usize; 4] = [0, 32, 256, 2_048];

pub(super) struct HostFixture {
    asset: u64,
    checkpoint: Vec<u8>,
}

impl HostFixture {
    pub(super) fn new(history: usize, deferred: bool) -> Self {
        let source = if deferred {
            include_str!("../../../../fixtures/recite/valid/benchmarks/session_deferred.recite")
        } else {
            include_str!("../../../../fixtures/recite/valid/benchmarks/session_history.recite")
        };
        let options = CompileOptions::new(
            must(CompilerVersion::new("benchmarks")),
            must(CompiledAssetId::new("session.recitec")),
            must(SourceMapId::new("session.recitec.map")),
            SchemaFingerprint::NoSchema,
        );
        let report = must(compile_inputs(
            [CompileInput::new("session.recite", source)],
            options,
        ));
        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
        let compiled = match report.asset {
            Some(asset) => asset,
            None => panic!("missing benchmark asset"),
        };
        let loaded = must(LoadedDialogue::from_bytes(&compiled.messagepack));
        let mut driver = SessionDriver::new();
        must(driver.start(
            StartRequest {
                asset: &loaded,
                block_id: None,
                options: DialogueSessionOptions::new(),
            },
            &ready,
            LocaleResolution::new(),
        ));
        let choice = must(ChoiceId::new("11111111111111111111"));
        for _ in 0..history {
            must(driver.select_choice(choice.clone(), &ready, LocaleResolution::new()));
        }
        let checkpoint = must(driver.snapshot());
        let mut asset = 0;
        // SAFETY: the compiled byte buffer and writable output live through the call.
        check(unsafe {
            recite_asset_load(
                compiled.messagepack.as_ptr(),
                compiled.messagepack.len(),
                &raw mut asset,
            )
        });
        let fixture = Self { asset, checkpoint };
        // Validate the real host projection and checkpoint once, outside all
        // measured regions, so a faster incomplete/no-op workload cannot win.
        let mut session = fixture.prepare();
        let output = session.choose();
        let decoded: serde_json::Value = must(rmp_serde::from_slice(output.bytes()));
        assert_eq!(decoded["events"][0]["kind"], "prompt");
        assert_eq!(decoded["events"][0]["choices"][0]["id"], choice.as_str());
        let mut snapshot = Batch(ReciteBuffer::null());
        // SAFETY: the session is live on this thread and snapshot is writable.
        check(unsafe { recite_session_snapshot(session.0, &raw mut snapshot.0) });
        must(driver.select_choice(choice, &ready, LocaleResolution::new()));
        assert_eq!(snapshot.bytes(), must(driver.snapshot()));
        fixture
    }

    pub(super) fn prepare(&self) -> HostSession {
        let mut handle = 0;
        // SAFETY: the fixture owns the asset and checkpoint; output is writable.
        check(unsafe {
            recite_session_prepare_restore(
                self.asset,
                self.checkpoint.as_ptr(),
                self.checkpoint.len(),
                &raw mut handle,
            )
        });
        let session = HostSession(handle);
        // SAFETY: the callback uses immutable static result storage and no userdata.
        check(unsafe {
            recite_session_register_condition(
                handle,
                c"ready".as_ptr(),
                Some(condition),
                std::ptr::null_mut(),
            )
        });
        let mut initial = Batch(ReciteBuffer::null());
        // SAFETY: this thread owns the prepared handle; initial is writable.
        check(unsafe { recite_session_begin(handle, &raw mut initial.0) });
        session
    }
}

impl Drop for HostFixture {
    fn drop(&mut self) {
        recite_asset_free(self.asset);
    }
}

pub(super) struct HostSession(u64);

impl HostSession {
    pub(super) fn choose(&mut self) -> Batch {
        let mut batch = Batch(ReciteBuffer::null());
        // SAFETY: this thread owns the live handle, choice is static UTF-8,
        // and batch is writable. The returned allocation is owned by Batch.
        check(unsafe {
            recite_session_choose(self.0, c"11111111111111111111".as_ptr(), &raw mut batch.0)
        });
        batch
    }
}

impl Drop for HostSession {
    fn drop(&mut self) {
        recite_session_free(self.0);
    }
}

pub(super) struct Batch(ReciteBuffer);

impl Batch {
    fn bytes(&self) -> &[u8] {
        // SAFETY: the successful FFI call created the allocation, owned by
        // this Batch and released only when its borrow has ended.
        unsafe { std::slice::from_raw_parts(self.0.data, self.0.len) }
    }
}

impl Drop for Batch {
    fn drop(&mut self) {
        // SAFETY: Batch frees its unique FFI allocation exactly once.
        unsafe {
            recite_buffer_free(&raw mut self.0);
        }
    }
}

unsafe extern "C" fn condition(
    _: *const ReciteConditionQuery,
    _: *mut c_void,
) -> ReciteConditionResult {
    const VALUE: &[u8] = b"\x82\xa4kind\xa4bool\xa5value\xc3";
    ReciteConditionResult {
        ok: 1,
        value_msgpack: VALUE.as_ptr(),
        value_len: VALUE.len(),
        error_message: std::ptr::null(),
    }
}

fn ready(_: ConditionQuery<'_>) -> Result<ConditionValue, ConditionEvaluationError> {
    Ok(ConditionValue::Bool(true))
}

fn check(status: ReciteStatus) {
    assert_eq!(status, ReciteStatus::Ok, "FFI benchmark operation failed");
}

fn must<T, E: std::fmt::Display>(value: Result<T, E>) -> T {
    match value {
        Ok(value) => value,
        Err(error) => panic!("invalid FFI benchmark: {error}"),
    }
}
