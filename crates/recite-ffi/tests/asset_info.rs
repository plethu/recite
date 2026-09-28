#[path = "support/mod.rs"]
mod support;

use recite_core::compiled::{ContentFingerprint, SchemaFingerprint};
use recite_ffi::{
    ReciteBuffer, ReciteStatus, recite_asset_free, recite_asset_info, recite_asset_load,
    recite_buffer_free,
};
use serde::Deserialize;
use support::{compile_to_bytes, compile_to_bytes_with_schema};

#[derive(Debug, Deserialize)]
struct FingerprintInfo {
    algorithm: String,
    #[serde(with = "serde_bytes")]
    digest: Vec<u8>,
}

#[derive(Debug, Deserialize)]
struct AssetInfo {
    asset_info_format_version: u16,
    asset_id: String,
    content_fingerprint: FingerprintInfo,
    schema_fingerprint: Option<FingerprintInfo>,
    format_version: u16,
    compiler_compatibility_version: u16,
    compiler_version: String,
    source_map_id: String,
}

fn load_info(source: &str) -> AssetInfo {
    let bytes = compile_to_bytes(source);
    let mut asset = 0;
    assert_eq!(
        unsafe { recite_asset_load(bytes.as_ptr(), bytes.len(), &raw mut asset) },
        ReciteStatus::Ok
    );
    let mut output = ReciteBuffer::null();
    assert_eq!(
        unsafe { recite_asset_info(asset, &raw mut output) },
        ReciteStatus::Ok
    );
    let info = match rmp_serde::from_slice::<AssetInfo>(unsafe {
        std::slice::from_raw_parts(output.data, output.len)
    }) {
        Ok(info) => info,
        Err(error) => panic!("asset info should decode: {error}"),
    };
    unsafe { recite_buffer_free(&raw mut output) };
    recite_asset_free(asset);
    info
}

#[test]
fn asset_info_reports_canonical_identity_and_no_schema() {
    let first = load_info(":: start default\n> line@11111111111111111111\n  Old.\n-> END\n");
    let changed = load_info(":: start default\n> line@11111111111111111111\n  New.\n-> END\n");
    assert_eq!(first.asset_info_format_version, 0);
    assert_eq!(first.asset_id, "test/main.recitec");
    assert_eq!(
        first
            .schema_fingerprint
            .as_ref()
            .map(|value| value.algorithm.as_str()),
        None
    );
    assert_eq!(first.format_version, 0);
    assert_eq!(first.compiler_compatibility_version, 0);
    assert_eq!(first.compiler_version, "0.0.1");
    assert_eq!(first.source_map_id, "test/main.recitec.map");
    assert_eq!(first.content_fingerprint.algorithm, "blake3");
    assert_eq!(first.content_fingerprint.digest.len(), 32);
    assert_ne!(
        first.content_fingerprint.digest,
        changed.content_fingerprint.digest
    );
}

#[test]
fn asset_info_rejects_null_output_and_unknown_or_malformed_handles() {
    assert_eq!(
        unsafe { recite_asset_info(0, std::ptr::null_mut()) },
        ReciteStatus::Validation
    );
    let mut output = ReciteBuffer::null();
    assert_eq!(
        unsafe { recite_asset_info(u64::MAX, &raw mut output) },
        ReciteStatus::InvalidHandle
    );
    let malformed = b"not messagepack";
    let mut handle = 0;
    assert_eq!(
        unsafe { recite_asset_load(malformed.as_ptr(), malformed.len(), &raw mut handle) },
        ReciteStatus::AssetLoadOrDecode
    );
    assert_eq!(handle, 0);
    assert_eq!(
        unsafe { recite_asset_info(handle, &raw mut output) },
        ReciteStatus::InvalidHandle
    );
}

#[test]
fn asset_info_reports_present_schema_fingerprint() {
    let schema = match ContentFingerprint::blake3(vec![7; 32]) {
        Ok(value) => value,
        Err(error) => panic!("schema fingerprint should validate: {error}"),
    };
    let bytes = compile_to_bytes_with_schema(
        ":: start default\n> line@11111111111111111111\n  Hello.\n-> END\n",
        SchemaFingerprint::Fingerprint(schema),
    );
    let mut asset = 0;
    assert_eq!(
        unsafe { recite_asset_load(bytes.as_ptr(), bytes.len(), &raw mut asset) },
        ReciteStatus::Ok
    );
    let mut output = ReciteBuffer::null();
    assert_eq!(
        unsafe { recite_asset_info(asset, &raw mut output) },
        ReciteStatus::Ok
    );
    let info = match rmp_serde::from_slice::<AssetInfo>(unsafe {
        std::slice::from_raw_parts(output.data, output.len)
    }) {
        Ok(info) => info,
        Err(error) => panic!("asset info should decode: {error}"),
    };
    let schema = match info.schema_fingerprint {
        Some(schema) => schema,
        None => panic!("schema fingerprint should be present"),
    };
    assert_eq!(schema.algorithm, "blake3");
    assert_eq!(schema.digest, vec![7; 32]);
    unsafe { recite_buffer_free(&raw mut output) };
    recite_asset_free(asset);
}
