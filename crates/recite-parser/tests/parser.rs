use std::{fs, path::PathBuf};

use expect_test::{Expect, expect};
use recite_core::{
    Argument, Block, Choice, ChoiceEcho, ConditionExpression, Diagnostic, DivertTarget, EffectMode,
    IfBranch, Line, MatchBranch, MatchPattern, ScalarValue, SpeakerId, Statement, StatementKind,
    Value,
};
use recite_parser::{LoweredSourceFile, ReciteSyntaxKind, parse};

const TEST_PATH: &str = "dialogue/tavern.recite";

#[path = "parser/branches_and_recovery.rs"]
mod branches_and_recovery;
#[path = "parser/lowering.rs"]
mod lowering;
#[path = "parser/metadata.rs"]
mod metadata;
#[path = "parser/statements.rs"]
mod statements;
#[path = "parser/syntax_and_fixtures.rs"]
mod syntax_and_fixtures;

fn lower(source: &str) -> LoweredSourceFile {
    parse(TEST_PATH, source).lower_source_file()
}

fn single_block(lowered: &LoweredSourceFile) -> &Block {
    assert_eq!(lowered.source_file.blocks.len(), 1);
    &lowered.source_file.blocks[0]
}

fn line_statement(block: &Block, index: usize) -> &Line {
    let Statement::Line(line) = &block.statements[index] else {
        panic!("expected statement {index} to be a line");
    };

    line
}

fn choice_statement(block: &Block, index: usize) -> &Choice {
    let Statement::Choice(choice) = &block.statements[index] else {
        panic!("expected statement {index} to be a choice");
    };

    choice
}

fn nested_choice(line: &Line, index: usize) -> &Choice {
    let Statement::Choice(choice) = &line.statements[index] else {
        panic!("expected nested statement {index} to be a choice");
    };

    choice
}

fn if_statement(block: &Block, index: usize) -> &IfBranch {
    let Statement::If(branch) = &block.statements[index] else {
        panic!("expected statement {index} to be an if branch");
    };

    branch
}

fn match_statement(block: &Block, index: usize) -> &MatchBranch {
    let Statement::Match(branch) = &block.statements[index] else {
        panic!("expected statement {index} to be a match branch");
    };

    branch
}

fn comment_statement(block: &Block, index: usize) -> &recite_core::Comment {
    let Statement::Comment(comment) = &block.statements[index] else {
        panic!("expected statement {index} to be a comment");
    };

    comment
}

fn assert_diagnostic_codes<const N: usize>(lowered: &LoweredSourceFile, expected: [&str; N]) {
    assert_eq!(
        lowered
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<Vec<_>>(),
        expected
    );
}

fn assert_snapshot(actual: &str, expected: Expect) {
    expected.assert_eq(actual);
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

fn push_span_fields(output: &mut String, indent: &str, span: &recite_core::SourceSpan) {
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

fn lowered_summary(lowered: &LoweredSourceFile) -> String {
    let mut summary = String::new();

    summary.push_str("diagnostics:\n");
    if lowered.diagnostics.is_empty() {
        summary.push_str("  <none>\n");
    } else {
        for diagnostic in &lowered.diagnostics {
            summary.push_str(&format!(
                "  - {} @ {}:{}\n",
                diagnostic.code.as_str(),
                diagnostic.span.start.line(),
                diagnostic.span.start.column()
            ));
        }
    }

    summary.push_str("blocks:\n");
    for block in &lowered.source_file.blocks {
        summary.push_str(&format!(
            "  - {} default={} statements={}\n",
            block.id.as_str(),
            block.is_default,
            block.statements.len()
        ));

        for statement in &block.statements {
            match statement {
                Statement::Comment(comment) => summary.push_str(&format!(
                    "    - comment {:?} @ {}:{}\n",
                    comment.text,
                    comment.span.start.line(),
                    comment.span.start.column()
                )),
                Statement::Line(line) => summary.push_str(&format!(
                    "    - line {} speaker={} text={:?} metadata=[{}]\n",
                    line.id
                        .as_ref()
                        .map(recite_core::LineId::as_str)
                        .unwrap_or("<missing>"),
                    line.speaker
                        .as_ref()
                        .map(SpeakerId::as_str)
                        .unwrap_or("<none>"),
                    line.source_text.text,
                    line.metadata
                        .iter()
                        .map(|entry| entry.key.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )),
                other => summary.push_str(&format!("    - {:?}\n", other.kind())),
            }
        }
    }

    summary
}
