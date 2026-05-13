use std::{fs, path::PathBuf};

use recite_compiler::{ValidationReport, validate_source_files};
use recite_core::{
    Block, BlockId, Choice, ChoiceId, ChoiceTarget, Diagnostic, DivertTarget, Line, LineId,
    SourceFile, SourcePosition, SourceSpan, SourceText, Statement,
};
use recite_parser::parse;

#[path = "validation/fixtures.rs"]
mod fixtures;
#[path = "validation/ids_blocks_and_references.rs"]
mod ids_blocks_and_references;
#[path = "validation/ordering.rs"]
mod ordering;
#[path = "validation/source_spans.rs"]
mod source_spans;

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
    lower(path, fixture_source(path).as_str())
}

fn assert_diagnostic_snapshot(diagnostics: &[Diagnostic], expected_path: String) {
    assert_eq!(
        render_diagnostics(diagnostics),
        fixture_source(expected_path.as_str())
    );
}

fn render_diagnostics(diagnostics: &[Diagnostic]) -> String {
    let mut output = String::from("diagnostics:\n");

    if diagnostics.is_empty() {
        output.push_str("<none>\n");
        return output;
    }

    for diagnostic in diagnostics {
        output.push_str(&format!(
            "- code: {}\n  severity: {:?}\n  message: {}\n",
            diagnostic.code.as_str(),
            diagnostic.severity,
            diagnostic.message
        ));
        push_span_fields(&mut output, "  ", &diagnostic.span);

        if !diagnostic.related.is_empty() {
            output.push_str("  related:\n");
            for related in &diagnostic.related {
                output.push_str(&format!("  - message: {}\n", related.message));
                push_span_fields(&mut output, "    ", &related.span);
            }
        }

        if let Some(help) = &diagnostic.help {
            output.push_str(&format!("  help: {help}\n"));
        }
    }

    output
}

fn push_span_fields(output: &mut String, indent: &str, span: &SourceSpan) {
    output.push_str(&format!(
        "{indent}file: {}\n{indent}line: {}\n{indent}column: {}\n",
        span.file,
        span.start.line(),
        span.start.column()
    ));

    if let Some(end) = span.end {
        output.push_str(&format!(
            "{indent}end_line: {}\n{indent}end_column: {}\n",
            end.line(),
            end.column()
        ));
    }
}

fn fixture_source(relative_path: &str) -> String {
    fs::read_to_string(workspace_path(relative_path))
        .unwrap_or_else(|error| panic!("failed to read fixture `{relative_path}`: {error}"))
}

fn workspace_path(relative_path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative_path)
}

fn diagnostic_snapshot_path(source_path: &str) -> String {
    source_path
        .strip_suffix(".recite")
        .expect("Recite fixture paths end with .recite")
        .to_owned()
        + ".diagnostics.txt"
}

fn assert_codes<const N: usize>(report: &ValidationReport, expected: [&str; N]) {
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
