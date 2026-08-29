use std::{fs, path::PathBuf};

use recite_core::{Diagnostic, SourceSpan};
use recite_ui::{UiCatalog, UiLocale};

pub(crate) fn assert_diagnostic_snapshot(diagnostics: &[Diagnostic], snapshot_name: String) {
    let catalog = UiCatalog::load(&UiLocale::default()).expect("default UI catalog");
    assert_text_snapshot(&render_diagnostics(diagnostics, &catalog), snapshot_name);
}

pub(crate) fn assert_text_snapshot(actual: &str, snapshot_name: String) {
    let snapshot_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots");
    insta::with_settings!({ snapshot_path => snapshot_path }, {
        insta::assert_snapshot!(snapshot_name, actual);
    });
}

pub(crate) fn fixture_source(relative_path: &str) -> String {
    fs::read_to_string(workspace_path(relative_path))
        .unwrap_or_else(|error| panic!("failed to read fixture `{relative_path}`: {error}"))
}

pub(crate) fn fixture_snapshot_name(source_path: &str, suffix: &str) -> String {
    let stem = source_path
        .strip_suffix(".recite")
        .expect("Recite fixture paths end with .recite");

    sanitize_snapshot_name(&format!("{stem}{suffix}"))
}

fn render_diagnostics(diagnostics: &[Diagnostic], catalog: &UiCatalog) -> String {
    let mut output = String::from("diagnostics:\n");

    if diagnostics.is_empty() {
        output.push_str("<none>\n");
        return output;
    }

    for diagnostic in diagnostics {
        let record = diagnostic
            .record()
            .expect("first-party diagnostic has a structured record");
        let rendered = catalog
            .render_diagnostic(&record)
            .expect("first-party diagnostic has a renderable presentation");
        output.push_str(&format!(
            "- code: {}\n  severity: {:?}\n  message: {}\n",
            record.code.as_str(),
            record.severity,
            rendered.primary_text
        ));
        push_span_fields(&mut output, "  ", &record.span);

        if !rendered.related.is_empty() {
            output.push_str("  related:\n");
            for related in &rendered.related {
                output.push_str(&format!("  - message: {}\n", related.text));
                push_span_fields(&mut output, "    ", &related.span);
            }
        }

        if let Some(help) = &rendered.help {
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

fn workspace_path(relative_path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative_path)
}

fn sanitize_snapshot_name(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '_'
            }
        })
        .collect()
}
