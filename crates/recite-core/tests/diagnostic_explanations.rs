#![cfg(test)]

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::Path;

use recite_core::{
    DiagnosticCategory, DiagnosticCode, explain_diagnostic_code, known_diagnostic_explanations,
    suggest_diagnostic_code,
};

#[test]
fn diagnostic_explanations_are_valid_and_ordered() {
    let explanations = known_diagnostic_explanations().collect::<Vec<_>>();
    assert!(!explanations.is_empty());

    let mut previous = "";
    for explanation in explanations {
        let code = explanation.code.as_str();
        assert!(previous < code, "{code} is not strictly sorted");
        previous = code;
        assert_eq!(
            explanation.category,
            explanation.code.category(),
            "{code} category does not match code prefix"
        );
        assert!(
            !explanation.meaning.trim().is_empty(),
            "{code} is missing meaning text"
        );
        assert!(
            !explanation.common_causes.is_empty(),
            "{code} is missing common causes"
        );
        assert!(
            !explanation.remediation.is_empty(),
            "{code} is missing remediation"
        );
    }
}

#[test]
fn diagnostic_explanation_lookup_returns_author_guidance() {
    let code = DiagnosticCode::new_static("RECITE_PARSE001");
    let explanation = match explain_diagnostic_code(&code) {
        Some(explanation) => explanation,
        None => panic!("RECITE_PARSE001 should have an explanation"),
    };

    assert_eq!(explanation.category, DiagnosticCategory::Parse);
    assert!(explanation.meaning.contains("source text"));
    assert!(
        explanation
            .remediation
            .iter()
            .any(|step| step.contains("reported span"))
    );
}

#[test]
fn diagnostic_explanations_expose_stable_structured_presentation_references() {
    let code = DiagnosticCode::new_static("RECITE_PARSE001");
    let explanation = explain_diagnostic_code(&code).expect("known diagnostic");
    let presentation = explanation.presentation();

    assert_eq!(
        explanation.default_code_presentation_id().as_str(),
        "diagnostic-parse-001"
    );
    assert_eq!(
        presentation.meaning.id().as_str(),
        "diagnostic-parse-001-meaning"
    );
    assert!(presentation.meaning.arguments().is_empty());
    assert_eq!(
        presentation.common_causes[0].id().as_str(),
        "diagnostic-parse-001-cause-001"
    );
    assert_eq!(
        presentation.remediation[0].id().as_str(),
        "diagnostic-parse-001-remediation-001"
    );
    assert!(presentation.common_causes[0].arguments().is_empty());
    assert!(presentation.remediation[0].arguments().is_empty());
}

#[test]
fn diagnostic_code_suggestions_accept_close_inputs() {
    let suggestion = match suggest_diagnostic_code("recite_parse001") {
        Some(suggestion) => suggestion,
        None => panic!("lowercase code should suggest RECITE_PARSE001"),
    };
    assert_eq!(suggestion.code.as_str(), "RECITE_PARSE001");

    let suggestion = match suggest_diagnostic_code("RECITE_PARSE01") {
        Some(suggestion) => suggestion,
        None => panic!("near code should suggest RECITE_PARSE001"),
    };
    assert_eq!(suggestion.code.as_str(), "RECITE_PARSE001");
}

#[test]
fn diagnostic_explanations_match_shifted_parser_and_validation_codes() {
    assert_explanation_contains("RECITE_PARSE002", "before any block header");
    assert_explanation_contains("RECITE_PARSE010", "divert");
    assert_explanation_contains("RECITE_PARSE017", "Prose");
    assert_explanation_contains("RECITE_VALIDATE011", "ambiguous");
    assert_explanation_contains("RECITE_VALIDATE013", "line contains a nested statement");
    assert_explanation_contains("RECITE_VALIDATE014", "choice contains a nested statement");
    assert_explanation_contains("RECITE_VALIDATE037", "argument value");
    assert_explanation_contains("RECITE_VALIDATE041", "without a requirement");
    assert_explanation_contains("RECITE_VALIDATE046", "plural dialogue line");
}

#[test]
fn plural_shape_explanation_preserves_stable_presentation_slots() {
    let code = DiagnosticCode::new_static("RECITE_VALIDATE046");
    let explanation = explain_diagnostic_code(&code).expect("known plural explanation");
    let presentation = explanation.presentation();

    assert_eq!(
        presentation
            .common_causes
            .iter()
            .map(|item| item.id().as_str())
            .collect::<Vec<_>>(),
        [
            "diagnostic-validate-046-cause-001",
            "diagnostic-validate-046-cause-002",
            "diagnostic-validate-046-cause-003",
            "diagnostic-validate-046-cause-004",
        ]
    );
    assert_eq!(
        presentation
            .remediation
            .iter()
            .map(|item| item.id().as_str())
            .collect::<Vec<_>>(),
        [
            "diagnostic-validate-046-remediation-001",
            "diagnostic-validate-046-remediation-002",
            "diagnostic-validate-046-remediation-003",
            "diagnostic-validate-046-remediation-004",
        ]
    );
    assert_eq!(
        explanation.common_causes,
        [
            "The plural source forms do not have the required two-form body shape.",
            "The singular or plural form contains a newline instead of exactly one body line.",
            "A plural line has no `bind=(count:int=$value)` binding.",
            "The `count` binding uses a type other than `int`.",
        ]
    );
    assert_eq!(
        explanation.remediation,
        [
            "Provide exactly one singular body line and one immediately following `|` continuation.",
            "Keep the singular and plural forms to one body line each.",
            "Declare the count source with `bind=(count:int=$value)`.",
            "Change the `count` binding type to `int`.",
        ]
    );
}

#[test]
fn diagnostic_explanations_cover_emitted_codes() -> io::Result<()> {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut emitted = BTreeSet::new();
    for crate_path in [
        "crates/recite-core/src",
        "crates/recite-parser/src",
        "crates/recite-compiler/src",
        "crates/recite-lsp/src",
    ] {
        collect_codes_in_dir(&workspace.join(crate_path), &mut emitted)?;
    }

    let explained = known_diagnostic_explanations()
        .map(|explanation| explanation.code.as_str().to_owned())
        .collect::<BTreeSet<_>>();

    assert_eq!(
        explained, emitted,
        "diagnostic explanation catalog must match emitted diagnostic codes"
    );

    Ok(())
}

fn assert_explanation_contains(code: &str, expected: &str) {
    let code = match DiagnosticCode::new(code) {
        Ok(code) => code,
        Err(error) => panic!("{code} should be a valid diagnostic code: {error}"),
    };
    let explanation = match explain_diagnostic_code(&code) {
        Some(explanation) => explanation,
        None => panic!("{} should have an explanation", code.as_str()),
    };
    assert!(
        explanation.meaning.contains(expected),
        "{} explanation meaning did not contain {expected:?}: {}",
        code.as_str(),
        explanation.meaning
    );
}

fn collect_codes_in_dir(dir: &Path, codes: &mut BTreeSet<String>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.ends_with("diagnostic/explanation") {
            continue;
        }
        if path.is_dir() {
            collect_codes_in_dir(&path, codes)?;
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
            collect_codes_in_file(&path, codes)?;
        }
    }
    Ok(())
}

fn collect_codes_in_file(path: &Path, codes: &mut BTreeSet<String>) -> io::Result<()> {
    let source = fs::read_to_string(path)?;
    let marker = "DiagnosticCode::new_static(\"";
    for line in source.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") {
            continue;
        }

        let mut rest = line;
        while let Some(index) = rest.find(marker) {
            let start = index + marker.len();
            let after_marker = &rest[start..];
            let Some(end) = after_marker.find('"') else {
                break;
            };
            codes.insert(after_marker[..end].to_owned());
            rest = &after_marker[end + 1..];
        }
    }

    Ok(())
}
