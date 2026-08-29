#![cfg(test)]

use recite_compiler::{ValidationReport, validate_source_files, validate_source_files_with_schema};
use recite_core::{
    Block, BlockId, Choice, ChoiceId, ChoiceTarget, Diagnostic, DivertTarget, Line, LineId,
    SourceFile, SourceMetadata, SourceMetadataEntry, SourceMetadataScalar, SourcePosition,
    SourceSpan, SourceText, Statement,
};
use recite_parser::parse;

#[path = "validation/asset_constraints.rs"]
mod asset_constraints;
#[path = "validation/conditions_schema.rs"]
mod conditions_schema;
#[path = "validation/effects_schema.rs"]
mod effects_schema;
#[path = "../../../tests/support/fixtures.rs"]
mod fixture_support;
#[path = "validation/fixtures.rs"]
mod fixtures;
#[path = "validation/ids_blocks_and_references.rs"]
mod ids_blocks_and_references;
#[path = "validation/interpolation.rs"]
mod interpolation;
#[path = "validation/markup.rs"]
mod markup;
#[path = "validation/metadata_schema/mod.rs"]
mod metadata_schema;
#[path = "validation/ordering.rs"]
mod ordering;
#[path = "validation/source_spans.rs"]
mod source_spans;

use fixture_support::assert_diagnostic_snapshot;

fn lower(path: &str, source: &str) -> recite_core::SourceFile {
    let parse = parse(path, source);
    let lowered = parse.lower_source_file();

    assert!(
        lowered.diagnostics.is_empty(),
        "test fixture must parse/lower cleanly: {:?}",
        lowered.diagnostics
    );

    lowered.source_file
}

fn lower_fixture(path: &str) -> SourceFile {
    lower(path, fixture_support::fixture_source(path).as_str())
}

fn diagnostic_snapshot_name(source_path: &str) -> String {
    fixture_support::fixture_snapshot_name(source_path, ".diagnostics.txt")
}

fn assert_codes<const N: usize>(report: &ValidationReport, expected: [&str; N]) {
    for diagnostic in &report.diagnostics {
        assert!(
            diagnostic.record().is_ok(),
            "compiler diagnostic must satisfy its structured record contract: {diagnostic:?}"
        );
    }
    assert_eq!(
        report
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<Vec<_>>(),
        expected
    );
}

fn assert_spans<const N: usize>(report: &ValidationReport, expected: [(u32, u32); N]) {
    assert_eq!(
        report
            .diagnostics
            .iter()
            .map(diagnostic_start)
            .collect::<Vec<_>>(),
        expected
    );
}

fn diagnostic_start(diagnostic: &Diagnostic) -> (u32, u32) {
    (diagnostic.span.start.line(), diagnostic.span.start.column())
}

fn span(file: &str, line: u32, column: u32) -> SourceSpan {
    SourceSpan::point(
        file,
        SourcePosition::new(line, column).expect("valid source position"),
    )
}

fn span_range(
    file: &str,
    start_line: u32,
    start_column: u32,
    end_line: u32,
    end_column: u32,
) -> SourceSpan {
    SourceSpan::new(
        file,
        SourcePosition::new(start_line, start_column).expect("valid source position"),
        Some(SourcePosition::new(end_line, end_column).expect("valid source position")),
    )
}
