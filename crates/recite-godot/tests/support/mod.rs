#![allow(
    dead_code,
    reason = "shared Godot adapter helpers are reused selectively across integration binaries"
)]

use std::path::PathBuf;

use recite_compiler::{CompileInput, CompileOptions, compile_inputs};
use recite_core::{CompiledAssetId, CompilerVersion, SchemaFingerprint, SourceMapId};
use recite_godot::{AdapterError, ReciteDialogueAsset, ReciteOutput};

pub(crate) fn compile_asset(
    path: &str,
    asset_id: &str,
    source: impl Into<String>,
) -> ReciteDialogueAsset {
    let bytes = compile_bytes(path, asset_id, source);
    match ReciteDialogueAsset::load_from_bytes(&bytes) {
        Ok(asset) => asset,
        Err(error) => panic!("compiled bytes should decode: {error}"),
    }
}

pub(crate) fn compile_bytes(path: &str, asset_id: &str, source: impl Into<String>) -> Vec<u8> {
    let compiler_version = match CompilerVersion::new("0.0.1") {
        Ok(value) => value,
        Err(error) => panic!("compiler version should be valid: {error}"),
    };
    let asset_id = match CompiledAssetId::new(asset_id) {
        Ok(value) => value,
        Err(error) => panic!("asset ID should be valid: {error}"),
    };
    let source_map_id = match SourceMapId::new("dialogue/test.recitec.map") {
        Ok(value) => value,
        Err(error) => panic!("source map ID should be valid: {error}"),
    };
    let report = match compile_inputs(
        [CompileInput::new(path, source.into())],
        CompileOptions::new(
            compiler_version,
            asset_id,
            source_map_id,
            SchemaFingerprint::NoSchema,
        ),
    ) {
        Ok(report) => report,
        Err(error) => panic!("compile should not hard fail: {error}"),
    };
    if !report.diagnostics.is_empty() {
        panic!(
            "test source should compile without diagnostics: {:?}",
            report.diagnostics
        );
    }
    match report.asset {
        Some(asset) => asset.messagepack,
        None => panic!("compiler should emit an asset"),
    }
}

pub(crate) fn must_ok(result: Result<Vec<ReciteOutput>, AdapterError>) -> Vec<ReciteOutput> {
    match result {
        Ok(outputs) => outputs,
        Err(error) => panic!("adapter operation should succeed: {error}"),
    }
}

pub(crate) fn must_ok_unit(result: Result<(), AdapterError>) {
    if let Err(error) = result {
        panic!("adapter operation should succeed: {error}");
    }
}

pub(crate) fn assert_error_code(
    result: Result<Vec<ReciteOutput>, AdapterError>,
    expected_code: &str,
) {
    match result {
        Ok(outputs) => panic!("adapter operation should fail, got outputs: {outputs:?}"),
        Err(error) => assert_eq!(error.code(), expected_code),
    }
}

pub(crate) fn output_kinds<const N: usize>(outputs: &[ReciteOutput]) -> [&str; N] {
    let mut kinds = [""; N];
    assert_eq!(outputs.len(), N);
    for (index, output) in outputs.iter().enumerate() {
        kinds[index] = output_kind(output);
    }
    kinds
}

pub(crate) fn assert_line(output: &ReciteOutput, expected_id: &str, expected_text: &str) {
    let ReciteOutput::Line(line) = output else {
        panic!("expected line output, got {output:?}");
    };
    assert_eq!(line.id.as_str(), expected_id);
    assert_eq!(line.text, expected_text);
}

pub(crate) fn assert_prompt_choice_ids<const N: usize>(
    output: &ReciteOutput,
    expected_ids: [&str; N],
) {
    let ReciteOutput::Prompt { choices, .. } = output else {
        panic!("expected prompt output, got {output:?}");
    };
    assert_eq!(
        choices
            .iter()
            .map(|choice| choice.id.as_str())
            .collect::<Vec<_>>(),
        expected_ids
    );
}

pub(crate) fn assert_effect(
    output: &ReciteOutput,
    expected_function: &str,
    expected_mode: &str,
) -> String {
    let ReciteOutput::Effect(effect) = output else {
        panic!("expected effect output, got {output:?}");
    };
    assert_eq!(effect.function, expected_function);
    assert_eq!(format!("{}", effect.mode), expected_mode);
    effect.id.as_str().to_owned()
}

pub(crate) fn assert_deferred_effects<const N: usize>(
    output: &ReciteOutput,
    expected_functions: [&str; N],
) {
    let ReciteOutput::End { deferred_effects } = output else {
        panic!("expected end output, got {output:?}");
    };
    assert_eq!(
        deferred_effects
            .iter()
            .map(|effect| effect.function.as_str())
            .collect::<Vec<_>>(),
        expected_functions
    );
}

pub(crate) fn temp_asset_path(file_name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("recite-godot-{}-{file_name}", std::process::id()));
    path
}

fn output_kind(output: &ReciteOutput) -> &'static str {
    match output {
        ReciteOutput::Line(_) => "line",
        ReciteOutput::Prompt { .. } => "prompt",
        ReciteOutput::Effect(_) => "effect",
        ReciteOutput::End { .. } => "end",
        _ => panic!("unrecognised ReciteOutput variant: {output:?}"),
    }
}
