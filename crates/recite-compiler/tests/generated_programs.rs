//! Properties of the compiler over generated programs: valid programs compile
//! without diagnostics, compilation is deterministic, and breaking one rule
//! the specification says validation must reject is reported.

use proptest::prelude::*;
use recite_compiler::compile::{
    CompileInput, CompileOptions, CompileReport, compile_inputs_with_schema,
};
use recite_core::{
    compiled::{CompiledAssetId, CompilerVersion, SourceMapId},
    schema::{ProjectSchema, load_schema_manifest_str},
};
use recite_testkit::{ASSET_ID, Program, SCHEMA_JSON, SCHEMA_PATH, apply_patch, patch, program};

#[path = "generated_programs/rejections.rs"]
mod rejections;

fn schema() -> ProjectSchema {
    load_schema_manifest_str(SCHEMA_PATH, SCHEMA_JSON)
        .schema
        .unwrap_or_else(|| panic!("generated schema is valid"))
}

fn compile_sources(sources: &[(String, String)], schema: &ProjectSchema) -> CompileReport {
    compile_inputs_with_schema(
        sources
            .iter()
            .map(|(path, source)| CompileInput::new(path.as_str(), source.as_str())),
        CompileOptions::new(
            CompilerVersion::new("0.0.1").unwrap_or_else(|error| panic!("{error}")),
            CompiledAssetId::new(ASSET_ID).unwrap_or_else(|error| panic!("{error}")),
            SourceMapId::new("generated.recitec.map").unwrap_or_else(|error| panic!("{error}")),
            schema.canonical_fingerprint(),
        ),
        schema,
    )
    .unwrap_or_else(|error| panic!("compile does not hard fail: {error}"))
}

fn assert_compiles(program: &Program, schema: &ProjectSchema) -> Result<Vec<u8>, TestCaseError> {
    let sources = program.sources();
    let report = compile_sources(&sources, schema);
    prop_assert!(
        report.diagnostics.is_empty(),
        "generated program must be valid: {:?}\n{}",
        report
            .diagnostics
            .iter()
            .map(|diagnostic| format!("{} {}", diagnostic.code.as_str(), diagnostic.message))
            .collect::<Vec<_>>(),
        render(&sources)
    );
    report
        .asset
        .map(|asset| asset.messagepack)
        .ok_or_else(|| TestCaseError::fail("a valid program compiles to an asset"))
}

fn render(sources: &[(String, String)]) -> String {
    sources
        .iter()
        .map(|(path, source)| format!("--- {path}\n{source}"))
        .collect()
}

proptest! {
    #[test]
    fn generated_programs_and_their_patches_compile_cleanly(
        program in program(),
        ops in patch(),
    ) {
        let schema = schema();
        assert_compiles(&program, &schema)?;
        assert_compiles(&apply_patch(&program, &ops), &schema)?;
    }

    #[test]
    fn compiling_a_program_twice_gives_byte_identical_assets(program in program()) {
        let schema = schema();
        let first = assert_compiles(&program, &schema)?;
        let second = assert_compiles(&program, &schema)?;
        prop_assert_eq!(first, second);
    }
}
