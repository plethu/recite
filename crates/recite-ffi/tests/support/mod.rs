#![allow(clippy::unwrap_used, clippy::expect_used, dead_code, unused_imports)]

use std::ffi::CString;

use recite_compiler::{CompileInput, CompileOptions, compile_inputs};
use recite_core::{CompiledAssetId, CompilerVersion, SchemaFingerprint, SourceMapId};
pub(crate) use recite_ffi::{
    ReciteBuffer, ReciteConditionQuery, ReciteConditionResult, ReciteStatus, recite_asset_free,
    recite_asset_load, recite_buffer_free, recite_last_error_message,
    recite_session_acknowledge_effect, recite_session_begin, recite_session_choose,
    recite_session_create, recite_session_free, recite_session_register_condition,
    recite_session_restore, recite_session_snapshot, recite_session_start,
};

pub(crate) fn compile_to_bytes(source: &str) -> Vec<u8> {
    compile_to_bytes_with_schema(source, SchemaFingerprint::NoSchema)
}

pub(crate) fn compile_to_bytes_with_schema(
    source: &str,
    schema_fingerprint: SchemaFingerprint,
) -> Vec<u8> {
    compile_to_bytes_with_schema_and_compiler(source, schema_fingerprint, "0.0.1")
}

pub(crate) fn compile_to_bytes_with_schema_and_compiler(
    source: &str,
    schema_fingerprint: SchemaFingerprint,
    compiler_version: &str,
) -> Vec<u8> {
    let report = compile_inputs(
        [CompileInput::new("test.recite", source)],
        CompileOptions::new(
            CompilerVersion::new(compiler_version).unwrap(),
            CompiledAssetId::new("test/main.recitec").unwrap(),
            SourceMapId::new("test/main.recitec.map").unwrap(),
            schema_fingerprint,
        ),
    )
    .expect("compile does not hard fail");
    assert!(
        report.diagnostics.is_empty(),
        "test source should compile cleanly: {:?}",
        report.diagnostics
    );
    report.asset.expect("compiler emits an asset").messagepack
}

pub(crate) fn cstr(s: &str) -> CString {
    CString::new(s).unwrap()
}

pub(crate) fn run_on_non_owner_thread(
    call: impl FnOnce() -> ReciteStatus + Send + 'static,
) -> ReciteStatus {
    std::thread::spawn(call)
        .join()
        .expect("thread should not panic")
}

pub(crate) fn decode_batch(buf: &ReciteBuffer) -> serde_json::Value {
    let bytes = unsafe { std::slice::from_raw_parts(buf.data, buf.len) };
    rmp_serde::from_slice(bytes).expect("valid msgpack batch")
}

pub(crate) fn event_kinds(batch: &serde_json::Value) -> Vec<&str> {
    batch["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["kind"].as_str().unwrap())
        .collect()
}
